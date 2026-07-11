//! Persistent translation task queue and background worker.

#![allow(dead_code)]
#![allow(clippy::significant_drop_tightening)]

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use babel_ebook::{CancellationToken, TranslationWorker};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, Runtime};
use tauri_plugin_store::Store;
use tokio::sync::Notify;

use crate::args::TranslateArgs;
use crate::commands::run_translation;
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

/// Serializable representation of a queued task that is written to disk.
///
/// Runtime-only fields such as [`CancellationToken`] are intentionally not
/// included so the store file can be safely reloaded on the next app launch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTask {
    /// Unique task identifier.
    pub id: String,
    /// Path to the source EPUB.
    pub source_path: String,
    /// Path where the translated EPUB will be written.
    pub output_path: String,
    /// Current lifecycle status.
    pub status: TaskStatus,
    /// Overall completion percentage (0-100).
    pub progress_percent: u32,
    /// Total number of translatable chapters, populated when the task starts.
    pub chapter_total: Option<u32>,
    /// Number of chapters that have already finished.
    pub chapters_completed: Option<u32>,
    /// Short human-readable status message.
    pub message: String,
    /// Error message when the task failed.
    pub error: Option<String>,
    /// Translation arguments captured at enqueue time.
    pub args: TranslateArgs,
    /// Unix timestamp when the task was created.
    pub created_at: u64,
    /// Unix timestamp when the task finished, failed or was cancelled.
    pub completed_at: Option<u64>,
}

impl From<&Task> for StoredTask {
    fn from(task: &Task) -> Self {
        Self {
            id: task.id.clone(),
            source_path: task.source_path.clone(),
            output_path: task.output_path.clone(),
            status: task.status,
            progress_percent: task.progress_percent,
            chapter_total: task.chapter_total,
            chapters_completed: task.chapters_completed,
            message: task.message.clone(),
            error: task.error.clone(),
            args: task.args.clone(),
            created_at: task.created_at,
            completed_at: task.completed_at,
        }
    }
}

impl From<StoredTask> for Task {
    fn from(stored: StoredTask) -> Self {
        Self {
            id: stored.id,
            source_path: stored.source_path,
            output_path: stored.output_path,
            status: stored.status,
            progress_percent: stored.progress_percent,
            chapter_total: stored.chapter_total,
            chapters_completed: stored.chapters_completed,
            message: stored.message,
            error: stored.error,
            args: stored.args,
            created_at: stored.created_at,
            completed_at: stored.completed_at,
        }
    }
}

/// Abstraction over where the queue persists its task list.
pub trait QueueStore: Send + Sync {
    /// Load the previously saved task list.
    ///
    /// Returns an empty vector when no persisted state exists yet.
    fn load(&self) -> Result<Vec<StoredTask>, String>;
    /// Save the given task list.
    fn save(&self, tasks: &[StoredTask]) -> Result<(), String>;
}

/// [`QueueStore`] implementation backed by `tauri_plugin_store`.
pub struct TauriTaskStore<R: Runtime>(Arc<Store<R>>);

impl<R: Runtime> TauriTaskStore<R> {
    /// Wrap an existing Tauri store.
    pub const fn new(store: Arc<Store<R>>) -> Self {
        Self(store)
    }
}

impl<R: Runtime> QueueStore for TauriTaskStore<R> {
    fn load(&self) -> Result<Vec<StoredTask>, String> {
        self.0.get("tasks").map_or_else(
            || Ok(Vec::new()),
            |value| serde_json::from_value(value).map_err(|e| e.to_string()),
        )
    }

    fn save(&self, tasks: &[StoredTask]) -> Result<(), String> {
        self.0.set(
            "tasks",
            serde_json::to_value(tasks).map_err(|e| e.to_string())?,
        );
        self.0.save().map_err(|e| e.to_string())
    }
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
    store: Option<Arc<dyn QueueStore>>,
}

impl QueueManager {
    /// Create a new empty queue manager without persistence.
    ///
    /// This is primarily useful for tests that do not need to survive restarts.
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
            store: None,
        }
    }

    /// Create a queue manager that loads its state from the given store.
    ///
    /// Any task that was running when the app last exited is restored as
    /// [`TaskStatus::Paused`] because the in-flight translation was killed.
    /// The queue itself always starts in the stopped state.
    pub fn with_store(store: Arc<dyn QueueStore>) -> Self {
        let stored = store.load().unwrap_or_else(|e| {
            tracing::error!("failed to load persisted queue state: {e}");
            Vec::new()
        });

        let mut tasks = Vec::with_capacity(stored.len());
        for mut stored_task in stored {
            if stored_task.status == TaskStatus::Running {
                stored_task.status = TaskStatus::Paused;
                stored_task.message = "Paused".to_string();
                stored_task.completed_at = None;
            }
            tasks.push(Task::from(stored_task));
        }

        Self {
            inner: Arc::new(std::sync::Mutex::new(QueueInner {
                tasks,
                running: false,
                current_task_id: None,
                current_cancellation: None,
            })),
            notifier: Arc::new(Notify::new()),
            store: Some(store),
        }
    }

    /// Write the current task list to the configured store, if any.
    fn persist(&self) -> Result<(), String> {
        let Some(store) = self.store.as_ref() else {
            return Ok(());
        };

        let guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let stored: Vec<StoredTask> = guard.tasks.iter().map(StoredTask::from).collect();
        drop(guard);

        store.save(&stored)
    }

    /// Start the background worker loop. Call once during Tauri setup.
    #[allow(clippy::too_many_lines)]
    pub fn spawn_worker(self, app_handle: tauri::AppHandle, worker: Arc<TranslationWorker>) {
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

                // Persist the transition to Running so a crash while processing
                // this task is restored as Paused on the next launch.
                if matches!(action, Action::Run(_)) {
                    if let Err(e) = self.persist() {
                        tracing::error!("failed to persist running task: {e}");
                    }
                }

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

                let result =
                    run_translation(task.args, progress, Some(cancellation), &worker).await;

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
                        if let Err(e) = self.persist() {
                            tracing::error!("failed to persist cancelled/paused task: {e}");
                        }
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

                if let Err(e) = self.persist() {
                    tracing::error!("failed to persist completed/failed task: {e}");
                }

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
    /// `get_queue_state` always returns an up-to-date snapshot. Progress
    /// updates are intentionally not persisted to avoid writing to disk on
    /// every chunk.
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
        drop(guard);

        if let Err(e) = self.persist() {
            tracing::error!("failed to persist enqueued task: {e}");
        }
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
        drop(guard);
        self.persist()
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
        drop(guard);
        self.persist()
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
        drop(guard);
        self.persist()
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

        let result = self.persist();
        if let Some(token) = current_cancellation {
            token.cancel();
        }
        result
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
        task.progress_percent = 0;
        task.chapters_completed = None;
        task.chapter_total = None;
        drop(guard);
        self.persist()?;
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

        if let Err(e) = self.persist() {
            tracing::error!("failed to persist queue pause: {e}");
        }
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
    percent.clamp(0.0, 100.0) as u32
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
    use std::fs;
    use std::path::{Path, PathBuf};

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

    /// In-memory store backed by a JSON file, used to exercise persistence
    /// without starting a full Tauri app.
    struct JsonFileTaskStore {
        path: PathBuf,
    }

    impl JsonFileTaskStore {
        fn new(path: impl AsRef<Path>) -> Self {
            Self {
                path: path.as_ref().to_path_buf(),
            }
        }
    }

    impl QueueStore for JsonFileTaskStore {
        fn load(&self) -> Result<Vec<StoredTask>, String> {
            if !self.path.exists() {
                return Ok(Vec::new());
            }
            let text = fs::read_to_string(&self.path).map_err(|e| e.to_string())?;
            serde_json::from_str(&text).map_err(|e| e.to_string())
        }

        fn save(&self, tasks: &[StoredTask]) -> Result<(), String> {
            let text = serde_json::to_string_pretty(tasks).map_err(|e| e.to_string())?;
            fs::write(&self.path, text).map_err(|e| e.to_string())
        }
    }

    fn temp_store() -> (tempfile::TempDir, Arc<JsonFileTaskStore>) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let store = Arc::new(JsonFileTaskStore::new(dir.path().join("queue.json")));
        (dir, store)
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
        assert_eq!(state.tasks[0].progress_percent, 0);
        assert_eq!(state.tasks[0].chapters_completed, None);
        assert_eq!(state.tasks[0].chapter_total, None);
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

    #[tokio::test]
    async fn progress_reaches_one_hundred_after_all_chapters() {
        let queue = QueueManager::new();
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        queue.update_task_progress(&task.id, &babel_ebook::ProgressEvent::Started { total: 3 });
        for index in 0..3 {
            queue.update_task_progress(
                &task.id,
                &babel_ebook::ProgressEvent::ChapterFinished {
                    index,
                    href: format!("ch{index:02}.xhtml"),
                },
            );
        }
        let state = queue.state();
        let task = &state.tasks[0];
        assert_eq!(task.chapters_completed, Some(3));
        assert_eq!(task.progress_percent, 100);
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

    #[tokio::test]
    async fn enqueue_persists_tasks() {
        let (_dir, store) = temp_store();
        let queue = QueueManager::with_store(store.clone());
        let args = sample_args("a.epub", "a.out.epub");
        let task = queue.enqueue(args.clone());

        // Simulate an app restart by loading the same store into a new manager.
        let restored = QueueManager::with_store(store);
        let state = restored.state();
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].id, task.id);
        assert_eq!(state.tasks[0].status, TaskStatus::Pending);
        assert_eq!(state.tasks[0].source_path, "a.epub");
        assert_eq!(state.tasks[0].output_path, "a.out.epub");
        assert_eq!(state.tasks[0].args.source, args.source);
        assert_eq!(state.tasks[0].args.output, args.output);
    }

    #[tokio::test]
    async fn running_task_is_restored_as_paused() {
        let (_dir, store) = temp_store();
        let stored = StoredTask {
            id: "task-1".to_string(),
            source_path: "a.epub".to_string(),
            output_path: "a.out.epub".to_string(),
            status: TaskStatus::Running,
            progress_percent: 42,
            chapter_total: Some(10),
            chapters_completed: Some(4),
            message: "Running".to_string(),
            error: None,
            args: sample_args("a.epub", "a.out.epub"),
            created_at: 1,
            completed_at: Some(2),
        };
        store.save(&[stored]).unwrap();

        let queue = QueueManager::with_store(store);
        let state = queue.state();
        assert!(!state.running);
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].status, TaskStatus::Paused);
        assert_eq!(state.tasks[0].message, "Paused");
        assert!(state.tasks[0].completed_at.is_none());
    }

    #[tokio::test]
    async fn completed_task_survives_restart() {
        let (_dir, store) = temp_store();
        let stored = StoredTask {
            id: "task-1".to_string(),
            source_path: "a.epub".to_string(),
            output_path: "a.out.epub".to_string(),
            status: TaskStatus::Completed,
            progress_percent: 100,
            chapter_total: Some(5),
            chapters_completed: Some(5),
            message: "Done".to_string(),
            error: None,
            args: sample_args("a.epub", "a.out.epub"),
            created_at: 1,
            completed_at: Some(2),
        };
        store.save(&[stored]).unwrap();

        let queue = QueueManager::with_store(store);
        let state = queue.state();
        assert_eq!(state.tasks[0].status, TaskStatus::Completed);
        assert_eq!(state.tasks[0].progress_percent, 100);
        assert_eq!(state.tasks[0].completed_at, Some(2));
    }

    #[tokio::test]
    async fn pause_persists_task_status() {
        let (_dir, store) = temp_store();
        let queue = QueueManager::with_store(store.clone());
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        let token = CancellationToken::default();
        {
            let mut guard = queue
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.tasks[0].status = TaskStatus::Running;
            guard.current_task_id = Some(task.id);
            guard.current_cancellation = Some(token);
            guard.running = true;
        }

        queue.pause();

        let restored = QueueManager::with_store(store);
        let state = restored.state();
        assert!(!state.running);
        assert_eq!(state.tasks[0].status, TaskStatus::Paused);
    }

    #[tokio::test]
    async fn cancel_remove_and_retry_are_persisted() {
        let (_dir, store) = temp_store();
        let queue = QueueManager::with_store(store.clone());
        let first = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        let second = queue.enqueue(sample_args("b.epub", "b.out.epub"));

        queue.cancel(&first.id).unwrap();
        queue.remove(&second.id).unwrap();

        let restored = QueueManager::with_store(store.clone());
        assert_eq!(restored.state().tasks.len(), 1);
        assert_eq!(restored.state().tasks[0].status, TaskStatus::Cancelled);

        queue.retry(&first.id).unwrap();
        let restored_after_retry = QueueManager::with_store(store);
        assert_eq!(
            restored_after_retry.state().tasks[0].status,
            TaskStatus::Pending
        );
    }

    #[tokio::test]
    async fn reorder_is_persisted() {
        let (_dir, store) = temp_store();
        let queue = QueueManager::with_store(store.clone());
        let first = queue.enqueue(sample_args("a.epub", "a.out.epub"));
        let second = queue.enqueue(sample_args("b.epub", "b.out.epub"));

        queue
            .reorder(&[second.id.clone(), first.id.clone()])
            .unwrap();

        let restored = QueueManager::with_store(store);
        let state = restored.state();
        assert_eq!(state.tasks[0].id, second.id);
        assert_eq!(state.tasks[1].id, first.id);
    }

    #[tokio::test]
    async fn progress_updates_are_not_persisted() {
        let (_dir, store) = temp_store();
        let queue = QueueManager::with_store(store.clone());
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

        let restored = QueueManager::with_store(store);
        let state = restored.state();
        assert_eq!(state.tasks[0].progress_percent, 0);
        assert_eq!(state.tasks[0].chapters_completed, None);
        assert_eq!(state.tasks[0].chapter_total, None);
    }

    #[test]
    fn stored_task_does_not_include_runtime_fields() {
        let task = Task::new(sample_args("a.epub", "a.out.epub"));
        let stored = StoredTask::from(&task);
        // CancellationToken is a runtime-only field on Task and must never be
        // serialised into the persisted representation.
        let value = serde_json::to_value(&stored).unwrap();
        let object = value.as_object().expect("stored task is a JSON object");
        assert!(!object.contains_key("cancellation_token"));
        assert!(!object.contains_key("token"));
    }
}
