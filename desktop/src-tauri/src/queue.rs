//! In-memory translation task queue and background worker.

#![allow(dead_code)]
#![allow(clippy::significant_drop_tightening)]

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use babel_ebook::CancellationToken;
use serde::Serialize;
use tauri::{Emitter, Manager};
use tokio::sync::Notify;

use crate::args::TranslateArgs;
use crate::commands::translate_epub_internal_with_cancellation;
use crate::task::{Task, TaskStatus};

/// Snapshot of the queue exposed to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct QueueState {
    /// All tasks in the queue, in display order.
    pub tasks: Vec<Task>,
    /// Whether the worker is currently allowed to process tasks.
    pub running: bool,
    /// ID of the task currently being processed, if any.
    pub current_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct TaskProgressPayload {
    task_id: String,
    event: babel_ebook::ProgressEvent,
}

struct QueueInner {
    tasks: Vec<Task>,
    running: bool,
    current_task_id: Option<String>,
    current_cancellation: Option<CancellationToken>,
}

/// Shared queue manager installed as Tauri state.
#[derive(Clone)]
pub struct QueueManager {
    inner: Arc<std::sync::Mutex<QueueInner>>,
    notifier: Arc<Notify>,
}

impl QueueManager {
    /// Create a new empty queue manager.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(QueueInner {
                tasks: Vec::new(),
                running: false,
                current_task_id: None,
                current_cancellation: None,
            })),
            notifier: Arc::new(Notify::new()),
        }
    }

    /// Start the background worker loop. Call once during Tauri setup.
    pub fn spawn_worker(self, app_handle: tauri::AppHandle) {
        tauri::async_runtime::spawn(async move {
            loop {
                enum Action {
                    Wait,
                    Run(Box<Task>),
                }

                let action = {
                    let mut guard = self
                        .inner
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if guard.running {
                        guard
                            .tasks
                            .iter_mut()
                            .find(|t| {
                                t.status == TaskStatus::Pending || t.status == TaskStatus::Paused
                            })
                            .map_or(Action::Wait, |t| {
                                t.status = TaskStatus::Running;
                                t.message = "Running".to_string();
                                Action::Run(Box::new(t.clone()))
                            })
                    } else {
                        Action::Wait
                    }
                };

                let task = match action {
                    Action::Wait => {
                        self.notifier.notified().await;
                        continue;
                    }
                    Action::Run(task) => task,
                };

                let cancellation = CancellationToken::default();
                self.set_current_task(Some(task.id.clone()), Some(cancellation.clone()));
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.emit("queue_state_changed", ());
                }
                let task_id = task.id.clone();
                let window = app_handle.get_webview_window("main");
                let queue = self.clone();
                let progress = window.clone().map(|w| {
                    Box::new(TaskProgressCallback {
                        task_id: task_id.clone(),
                        window: w,
                        queue,
                    }) as Box<dyn babel_ebook::ProgressCallback + Send + Sync>
                });

                let result = translate_epub_internal_with_cancellation(
                    task.args,
                    progress,
                    Some(cancellation),
                )
                .await;

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                // Update the current task id only after releasing any lock held
                // during the translation above.
                self.set_current_task(None, None);

                let mut guard = self
                    .inner
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(t) = guard.tasks.iter_mut().find(|t| t.id == task_id) {
                    if t.status == TaskStatus::Cancelled || t.status == TaskStatus::Paused {
                        t.completed_at = Some(now);
                        drop(guard);
                        if let Some(w) = window {
                            let _ = w.emit("queue_state_changed", ());
                        }
                        continue;
                    }
                    match result {
                        Ok(message) => {
                            t.status = TaskStatus::Completed;
                            t.progress_percent = 100;
                            t.chapters_completed = t.chapter_total;
                            t.message = message;
                            t.completed_at = Some(now);
                        }
                        Err(error) => {
                            t.status = TaskStatus::Failed;
                            t.message = "Failed".to_string();
                            t.error = Some(error);
                        }
                    }
                }
                drop(guard);

                if let Some(w) = window {
                    let _ = w.emit("queue_state_changed", ());
                }
            }
        });
    }

    fn set_current_task(&self, id: Option<String>, cancellation: Option<CancellationToken>) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.current_task_id = id;
        guard.current_cancellation = cancellation;
    }

    /// Update a task's progress fields from a core progress event.
    ///
    /// This is called synchronously from the progress callback so that
    /// `get_queue_state` always returns an up-to-date snapshot.
    fn update_task_progress(&self, task_id: &str, event: &babel_ebook::ProgressEvent) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(task) = guard.tasks.iter_mut().find(|t| t.id == task_id) else {
            return;
        };

        match event {
            babel_ebook::ProgressEvent::Started { total } => {
                task.chapter_total = Some(u32::try_from(*total).unwrap_or(u32::MAX));
                task.chapters_completed = Some(0);
                task.progress_percent = 0;
            }
            babel_ebook::ProgressEvent::ChapterFinished { .. } => {
                let completed = task.chapters_completed.unwrap_or(0).saturating_add(1);
                task.chapters_completed = Some(completed);
                task.progress_percent = compute_progress_percent(task);
            }
            babel_ebook::ProgressEvent::ChunkFinished {
                chunk_index,
                chunk_total,
                ..
            } if *chunk_total > 0 => {
                // Approximate within-chapter progress using the most recent chunk
                // event. This is refined by the frontend which tracks per-chapter
                // chunk state; the backend value is mainly a fallback for queue
                // state refreshes.
                let completed = task.chapters_completed.unwrap_or(0);
                let chunk_index = u32::try_from(*chunk_index).unwrap_or(u32::MAX);
                let chunk_total = u32::try_from(*chunk_total).unwrap_or(u32::MAX);
                let in_flight = f64::from(chunk_index.saturating_add(1)) / f64::from(chunk_total);
                task.progress_percent =
                    compute_progress_percent_raw(task.chapter_total, completed, in_flight);
            }
            _ => {}
        }
    }

    /// Add a new pending translation task to the queue.
    pub fn enqueue(&self, args: TranslateArgs) -> Task {
        let task = Task::new(args);
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.tasks.push(task.clone());
        self.notifier.notify_one();
        task
    }

    /// Remove a pending or completed task from the queue.
    pub fn remove(&self, id: &str) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let pos = guard
            .tasks
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| "task not found".to_string())?;
        let task = &guard.tasks[pos];
        if task.status == TaskStatus::Running {
            return Err("cannot remove a running task".to_string());
        }
        guard.tasks.remove(pos);
        Ok(())
    }

    /// Reorder tasks to match the provided list of IDs.
    pub fn reorder(&self, ids: &[String]) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut new_order: Vec<Task> = Vec::with_capacity(guard.tasks.len());
        for id in ids {
            let task = guard
                .tasks
                .iter()
                .find(|t| t.id == *id)
                .ok_or_else(|| format!("unknown task id: {id}"))?;
            if task.status == TaskStatus::Running {
                return Err("cannot reorder a running task".to_string());
            }
            new_order.push(task.clone());
        }
        // Append any tasks not in the provided ordering (preserve their relative order).
        for task in &guard.tasks {
            if !ids.contains(&task.id) {
                new_order.push(task.clone());
            }
        }
        guard.tasks = new_order;
        Ok(())
    }

    /// Mark a pending or running task as cancelled.
    pub fn cancel(&self, id: &str) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let current_cancellation = guard.current_cancellation.clone();
        let task = guard
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| "task not found".to_string())?;
        match task.status {
            TaskStatus::Pending => {
                task.status = TaskStatus::Cancelled;
                task.message = "Cancelled".to_string();
            }
            TaskStatus::Running => {
                task.status = TaskStatus::Cancelled;
                task.message = "Cancelling".to_string();
                if let Some(token) = current_cancellation {
                    token.cancel();
                }
            }
            _ => return Err("only pending or running tasks can be cancelled".to_string()),
        }
        Ok(())
    }

    /// Pause the currently running task so it can be resumed later.
    pub fn pause_task(&self, id: &str) -> Result<(), String> {
        let (current_id, current_cancellation) = {
            let guard = self
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (
                guard.current_task_id.clone(),
                guard.current_cancellation.clone(),
            )
        };

        let Some(current_id) = current_id else {
            return Err("no running task".to_string());
        };
        if current_id != id {
            return Err("task is not currently running".to_string());
        }

        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let task = guard
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| "task not found".to_string())?;
        if task.status != TaskStatus::Running {
            return Err("task is not running".to_string());
        }
        task.status = TaskStatus::Paused;
        task.message = "Paused".to_string();
        drop(guard);

        if let Some(token) = current_cancellation {
            token.cancel();
        }
        Ok(())
    }

    /// Reset a failed, cancelled or paused task to pending.
    pub fn retry(&self, id: &str) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let task = guard
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| "task not found".to_string())?;
        if task.status != TaskStatus::Failed
            && task.status != TaskStatus::Cancelled
            && task.status != TaskStatus::Paused
        {
            return Err("only failed, cancelled or paused tasks can be retried".to_string());
        }
        task.status = TaskStatus::Pending;
        task.message = String::new();
        task.error = None;
        task.completed_at = None;
        self.notifier.notify_one();
        Ok(())
    }

    /// Allow the worker to start processing pending tasks.
    pub fn start(&self) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.running = true;
        self.notifier.notify_one();
    }

    /// Pause the worker and cancel the currently running task so it can be
    /// resumed from its checkpoint later.
    pub fn pause(&self) {
        let current_cancellation = {
            let mut guard = self
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.running = false;
            if let Some(current_id) = guard.current_task_id.clone() {
                if let Some(task) = guard.tasks.iter_mut().find(|t| t.id == current_id) {
                    if task.status == TaskStatus::Running {
                        task.status = TaskStatus::Paused;
                        task.message = "Paused".to_string();
                    }
                }
            }
            guard.current_cancellation.clone()
        };
        if let Some(token) = current_cancellation {
            token.cancel();
        }
    }

    /// Return a snapshot of the current queue state.
    pub fn state(&self) -> QueueState {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        QueueState {
            tasks: guard.tasks.clone(),
            running: guard.running,
            current_task_id: guard.current_task_id.clone(),
        }
    }
}

fn compute_progress_percent(task: &Task) -> u32 {
    compute_progress_percent_raw(
        task.chapter_total,
        task.chapters_completed.unwrap_or(0),
        0.0,
    )
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn compute_progress_percent_raw(chapter_total: Option<u32>, completed: u32, in_flight: f64) -> u32 {
    let Some(total) = chapter_total else {
        return 0;
    };
    if total == 0 {
        return 0;
    }
    let percent = ((f64::from(completed) + in_flight) / f64::from(total)) * 100.0;
    percent.clamp(0.0, 99.0) as u32
}

struct TaskProgressCallback {
    task_id: String,
    window: tauri::WebviewWindow,
    queue: QueueManager,
}

impl babel_ebook::ProgressCallback for TaskProgressCallback {
    fn on_progress(&self, event: babel_ebook::ProgressEvent) {
        // Keep the backend task state in sync so the queue UI survives refreshes.
        self.queue.update_task_progress(&self.task_id, &event);

        // Emit a per-task event so the queue UI can update the correct task.
        let payload = TaskProgressPayload {
            task_id: self.task_id.clone(),
            event: event.clone(),
        };
        let _ = self.window.emit("task_progress", &payload);
        // Also emit the legacy translation_progress event so the log panel and
        // the translate-page progress bar receive updates from queued tasks.
        let _ = self.window.emit("translation_progress", &event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{PromptTemplates, TranslateArgs};

    fn sample_args(source: &str, output: &str) -> TranslateArgs {
        TranslateArgs {
            source: source.to_string(),
            output: output.to_string(),
            provider: "deepseek".to_string(),
            api_key: "test".to_string(),
            model: "deepseek-chat".to_string(),
            concurrency: 1,
            max_input_tokens: 1000,
            max_output_tokens: 500,
            temperature: 0.3,
            source_lang: "en".to_string(),
            target_lang: "zh-CN".to_string(),
            dry_run: true,
            base_url: None,
            output_mode: "bilingual".to_string(),
            style: "default".to_string(),
            preserve_classes: false,
            exclude_selectors: Vec::new(),
            translate_attributes: Vec::new(),
            translate_body: true,
            translate_metadata: true,
            translate_toc: true,
            translate_alt_text: true,
            translate_image_captions: true,
            translate_tables: true,
            translate_footnotes: true,
            translate_code: false,
            output_font: None,
            system_prompt: None,
            prompts: PromptTemplates::default(),
            refine: false,
            checkpoint_dir: ".babel_ebook_checkpoints".to_string(),
            resume: None,
        }
    }

    #[tokio::test]
    async fn enqueue_adds_pending_task() {
        let queue = QueueManager::new();
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        let state = queue.state();
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].id, task.id);
        assert_eq!(state.tasks[0].status, TaskStatus::Pending);
    }

    #[tokio::test]
    async fn cancel_only_pending_task() {
        let queue = QueueManager::new();
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        queue.cancel(&task.id).unwrap();
        let state = queue.state();
        assert_eq!(state.tasks[0].status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn cancel_running_task_requests_current_cancellation() {
        let queue = QueueManager::new();
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        let token = CancellationToken::default();
        {
            let mut guard = queue
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.tasks[0].status = TaskStatus::Running;
            guard.current_task_id = Some(task.id.clone());
            guard.current_cancellation = Some(token.clone());
        }

        queue.cancel(&task.id).unwrap();

        let state = queue.state();
        assert_eq!(state.tasks[0].status, TaskStatus::Cancelled);
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn retry_resets_failed_task() {
        let queue = QueueManager::new();
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        queue.cancel(&task.id).unwrap();
        queue.retry(&task.id).unwrap();
        let state = queue.state();
        assert_eq!(state.tasks[0].status, TaskStatus::Pending);
        assert!(state.tasks[0].error.is_none());
    }

    #[tokio::test]
    async fn pause_cancels_running_task() {
        let queue = QueueManager::new();
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        let token = CancellationToken::default();
        {
            let mut guard = queue
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.tasks[0].status = TaskStatus::Running;
            guard.current_task_id = Some(task.id);
            guard.current_cancellation = Some(token.clone());
            guard.running = true;
        }

        queue.pause();

        let state = queue.state();
        assert!(!state.running);
        assert_eq!(state.tasks[0].status, TaskStatus::Paused);
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn pause_task_pauses_only_current_running_task() {
        let queue = QueueManager::new();
        let first = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        let second = queue.enqueue(sample_args("b.epub", "b.out.epub"));
        let token = CancellationToken::default();
        {
            let mut guard = queue
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.tasks[0].status = TaskStatus::Running;
            guard.current_task_id = Some(first.id.clone());
            guard.current_cancellation = Some(token.clone());
        }

        assert!(queue.pause_task(&second.id).is_err());
        queue.pause_task(&first.id).unwrap();

        let state = queue.state();
        assert_eq!(state.tasks[0].status, TaskStatus::Paused);
        assert_eq!(state.tasks[1].status, TaskStatus::Pending);
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn remove_deletes_pending_task() {
        let queue = QueueManager::new();
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        queue.remove(&task.id).unwrap();
        let state = queue.state();
        assert!(state.tasks.is_empty());
    }

    #[tokio::test]
    async fn reorder_changes_task_order() {
        let queue = QueueManager::new();
        let first = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        let second = queue.enqueue(sample_args("b.epub", "b.out.epub"));
        queue
            .reorder(&[second.id.clone(), first.id.clone()])
            .unwrap();
        let state = queue.state();
        assert_eq!(state.tasks[0].id, second.id);
        assert_eq!(state.tasks[1].id, first.id);
    }

    #[tokio::test]
    async fn start_and_pause_toggle_running() {
        let queue = QueueManager::new();
        queue.start();
        assert!(queue.state().running);
        queue.pause();
        assert!(!queue.state().running);
    }

    #[tokio::test]
    async fn progress_state_survives_state_refresh() {
        let queue = QueueManager::new();
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        {
            let mut guard = queue
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.tasks[0].status = TaskStatus::Running;
        }

        queue.update_task_progress(&task.id, &babel_ebook::ProgressEvent::Started { total: 10 });
        queue.update_task_progress(
            &task.id,
            &babel_ebook::ProgressEvent::ChapterFinished {
                index: 0,
                href: "ch01.xhtml".to_string(),
            },
        );
        queue.update_task_progress(
            &task.id,
            &babel_ebook::ProgressEvent::ChapterFinished {
                index: 1,
                href: "ch02.xhtml".to_string(),
            },
        );

        let state = queue.state();
        let task = &state.tasks[0];
        assert_eq!(task.chapter_total, Some(10));
        assert_eq!(task.chapters_completed, Some(2));
        assert_eq!(task.progress_percent, 20);
    }

    #[test]
    fn task_progress_payload_serializes_with_nested_event() {
        let payload = TaskProgressPayload {
            task_id: "task-1".to_string(),
            event: babel_ebook::ProgressEvent::Started { total: 5 },
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"task_id\":\"task-1\""));
        assert!(json.contains("\"event\":{\"Started\":{\"total\":5}}"));

        let completed = TaskProgressPayload {
            task_id: "task-1".to_string(),
            event: babel_ebook::ProgressEvent::Completed,
        };
        let json_completed = serde_json::to_string(&completed).unwrap();
        assert!(json_completed.contains("\"event\":\"Completed\""));
    }
}
