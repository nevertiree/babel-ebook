//! In-memory translation task queue and background worker.

#![allow(dead_code)]
#![allow(clippy::significant_drop_tightening)]

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{Emitter, Manager};
use tokio::sync::{Mutex, Notify};

use crate::args::TranslateArgs;
use crate::commands::translate_epub_internal;
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
    #[serde(flatten)]
    event: babel_ebook::ProgressEvent,
}

struct QueueInner {
    tasks: Vec<Task>,
    running: bool,
    current_task_id: Option<String>,
}

/// Shared queue manager installed as Tauri state.
#[derive(Clone)]
pub struct QueueManager {
    inner: Arc<Mutex<QueueInner>>,
    notifier: Arc<Notify>,
}

impl QueueManager {
    /// Create a new empty queue manager.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(QueueInner {
                tasks: Vec::new(),
                running: false,
                current_task_id: None,
            })),
            notifier: Arc::new(Notify::new()),
        }
    }

    /// Start the background worker loop. Call once during Tauri setup.
    pub fn spawn_worker(self, app_handle: tauri::AppHandle) {
        tokio::spawn(async move {
            loop {
                let next = {
                    let mut guard = self.inner.lock().await;
                    if !guard.running {
                        // Queue is paused; drop the lock before waiting so that
                        // start()/enqueue() can acquire it to wake us up.
                        drop(guard);
                        self.notifier.notified().await;
                        continue;
                    }
                    guard
                        .tasks
                        .iter_mut()
                        .find(|t| t.status == TaskStatus::Pending)
                        .map(|t| {
                            t.status = TaskStatus::Running;
                            t.message = "Running".to_string();
                            t.clone()
                        })
                };

                let Some(task) = next else {
                    // Nothing to do; wait for a notification.
                    self.notifier.notified().await;
                    continue;
                };

                self.set_current_task(Some(task.id.clone())).await;
                let task_id = task.id.clone();
                let window = app_handle.get_webview_window("main");
                let progress = window.clone().map(|w| {
                    Box::new(TaskProgressCallback {
                        task_id: task_id.clone(),
                        window: w,
                    }) as Box<dyn babel_ebook::ProgressCallback + Send + Sync>
                });

                let result = translate_epub_internal(task.args, progress).await;

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                // Update the current task id only after releasing any lock held
                // during the translation above.
                self.set_current_task(None).await;

                let mut guard = self.inner.lock().await;
                if let Some(t) = guard.tasks.iter_mut().find(|t| t.id == task_id) {
                    match result {
                        Ok(message) => {
                            t.status = TaskStatus::Completed;
                            t.progress_percent = 100;
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

    async fn set_current_task(&self, id: Option<String>) {
        let mut guard = self.inner.lock().await;
        guard.current_task_id = id;
    }

    /// Add a new pending translation task to the queue.
    pub async fn enqueue(&self, args: TranslateArgs) -> Task {
        let task = Task::new(args);
        let mut guard = self.inner.lock().await;
        guard.tasks.push(task.clone());
        self.notifier.notify_one();
        task
    }

    /// Remove a pending or completed task from the queue.
    pub async fn remove(&self, id: &str) -> Result<(), String> {
        let mut guard = self.inner.lock().await;
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
    pub async fn reorder(&self, ids: Vec<String>) -> Result<(), String> {
        let mut guard = self.inner.lock().await;
        let mut new_order: Vec<Task> = Vec::with_capacity(guard.tasks.len());
        for id in &ids {
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

    /// Mark a pending task as cancelled.
    pub async fn cancel(&self, id: &str) -> Result<(), String> {
        let mut guard = self.inner.lock().await;
        let task = guard
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| "task not found".to_string())?;
        if task.status != TaskStatus::Pending {
            return Err("only pending tasks can be cancelled".to_string());
        }
        task.status = TaskStatus::Cancelled;
        task.message = "Cancelled".to_string();
        Ok(())
    }

    /// Reset a failed or cancelled task to pending.
    pub async fn retry(&self, id: &str) -> Result<(), String> {
        let mut guard = self.inner.lock().await;
        let task = guard
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| "task not found".to_string())?;
        if task.status != TaskStatus::Failed && task.status != TaskStatus::Cancelled {
            return Err("only failed or cancelled tasks can be retried".to_string());
        }
        task.status = TaskStatus::Pending;
        task.progress_percent = 0;
        task.message = String::new();
        task.error = None;
        task.completed_at = None;
        self.notifier.notify_one();
        Ok(())
    }

    /// Allow the worker to start processing pending tasks.
    pub async fn start(&self) {
        let mut guard = self.inner.lock().await;
        guard.running = true;
        self.notifier.notify_one();
    }

    /// Prevent the worker from picking up new tasks.
    pub async fn pause(&self) {
        let mut guard = self.inner.lock().await;
        guard.running = false;
    }

    /// Return a snapshot of the current queue state.
    pub async fn state(&self) -> QueueState {
        let guard = self.inner.lock().await;
        QueueState {
            tasks: guard.tasks.clone(),
            running: guard.running,
            current_task_id: guard.current_task_id.clone(),
        }
    }
}

struct TaskProgressCallback {
    task_id: String,
    window: tauri::WebviewWindow,
}

impl babel_ebook::ProgressCallback for TaskProgressCallback {
    fn on_progress(&self, event: babel_ebook::ProgressEvent) {
        let payload = TaskProgressPayload {
            task_id: self.task_id.clone(),
            event,
        };
        let _ = self.window.emit("task_progress", &payload);
    }
}
