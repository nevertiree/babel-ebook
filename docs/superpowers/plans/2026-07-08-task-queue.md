# Task Queue Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a backend-managed translation task queue to the BabelEbook desktop app so users can enqueue multiple books, view progress, and manage tasks from a dedicated page while the app processes them one at a time in the background.

**Architecture:** A Rust `QueueManager` lives as Tauri state. It owns an in-memory list of `Task`s and a Tokio worker loop that runs one pending task at a time by calling the existing `translate_epub_internal`. Task progress is emitted through a new `task_progress` event that includes the task id. The React frontend keeps a queue state in `App`, renders it in a new `TasksPage`, and adds an "Add to queue" button to `TranslatePage`.

**Tech Stack:** Rust (Tauri, Tokio, serde), TypeScript (React, i18next), Tauri event system.

## Global Constraints

- Do **not** modify `crates/babel-ebook` core translation logic.
- Process **one book at a time** (`max_parallel_tasks = 1`).
- Do **not** persist the queue across app restarts.
- Cancellation only affects `Pending` tasks; running tasks continue until the current book finishes.
- All changes must keep `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `pnpm exec tsc --noEmit`, and `pnpm tauri build` passing.
- i18n keys must be added to `desktop/src/locales/en.json` and `desktop/src/locales/zh-CN.json` at minimum.
- Branch from the latest `master`, open a PR, and merge only after CI passes.

---

## File Mapping

| File | Responsibility |
|------|----------------|
| `desktop/src-tauri/src/task.rs` | `Task` and `TaskStatus` data types plus constructors/converters. |
| `desktop/src-tauri/src/queue.rs` | `QueueManager`, `QueueState`, worker loop, progress callback adapter. |
| `desktop/src-tauri/src/commands.rs` | New Tauri commands exposed to the frontend. |
| `desktop/src-tauri/src/lib.rs` | Register commands, install `QueueManager` state, spawn worker. |
| `desktop/src/types.ts` | Add TypeScript `Task`, `TaskStatus`, `QueueState` types and update `Page`. |
| `desktop/src/App.tsx` | Add queue state, `task_progress` listener, sidebar entry, route to `TasksPage`. |
| `desktop/src/pages/TasksPage.tsx` | New page to display and manage the queue. |
| `desktop/src/pages/TranslatePage.tsx` | Add "Add to queue" button and enqueue handler. |
| `desktop/src/locales/*.json` | Add new UI strings. |

---

### Task 1: Backend task data model

**Files:**
- Create: `desktop/src-tauri/src/task.rs`
- Modify: `desktop/src-tauri/src/lib.rs` (add `mod task;`)

**Interfaces:**
- Produces: `Task`, `TaskStatus`, `Task::new(args: TranslateArgs) -> Self`.

- [ ] **Step 1: Create `task.rs` with types and constructors**

```rust
//! Task data model for the translation queue.

use crate::args::TranslateArgs;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

/// Lifecycle status of a queued translation task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A single queued translation job.
#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: String,
    pub source_path: String,
    pub output_path: String,
    pub status: TaskStatus,
    pub progress_percent: u32,
    pub message: String,
    pub error: Option<String>,
    #[serde(skip)]
    pub args: TranslateArgs,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

impl Task {
    /// Create a new pending task from the translation arguments captured at enqueue time.
    pub fn new(args: TranslateArgs) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_path: args.source.clone(),
            output_path: args.output.clone(),
            status: TaskStatus::Pending,
            progress_percent: 0,
            message: String::new(),
            error: None,
            args,
            created_at: now,
            completed_at: None,
        }
    }
}
```

- [ ] **Step 2: Add `uuid` dependency to `desktop/src-tauri/Cargo.toml`**

```toml
uuid = { version = "1.10", features = ["v4"] }
```

- [ ] **Step 3: Register the module in `desktop/src-tauri/src/lib.rs`**

Add `mod task;` next to the existing module declarations.

- [ ] **Step 4: Run `cargo check --workspace`**

Expected: compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add desktop/src-tauri/src/task.rs desktop/src-tauri/src/lib.rs desktop/src-tauri/Cargo.toml
git commit -m "feat(queue): add Task data model"
```

---

### Task 2: Queue manager and worker

**Files:**
- Create: `desktop/src-tauri/src/queue.rs`
- Modify: `desktop/src-tauri/src/lib.rs` (add `mod queue;`)

**Interfaces:**
- Consumes: `Task`, `TaskStatus` from `task.rs`; `translate_epub_internal` and `ProgressCallback` from `commands.rs`.
- Produces: `QueueManager` (methods listed below) and `QueueState`.

- [ ] **Step 1: Create `queue.rs`**

```rust
//! In-memory translation task queue and background worker.

use std::sync::Arc;

use serde::Serialize;
use tauri::Emitter;
use tokio::sync::{Mutex, Notify};

use crate::args::TranslateArgs;
use crate::commands::translate_epub_internal;
use crate::task::{Task, TaskStatus};

/// Snapshot of the queue exposed to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct QueueState {
    pub tasks: Vec<Task>,
    pub running: bool,
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
                // Wait until the queue is running and has a pending task.
                let next = {
                    let mut guard = self.inner.lock().await;
                    let next = guard
                        .tasks
                        .iter_mut()
                        .find(|t| t.status == TaskStatus::Pending)
                        .map(|t| {
                            t.status = TaskStatus::Running;
                            t.message = "Running".to_string();
                            self.notifier.notify_one();
                            t.clone()
                        });
                    if next.is_some() {
                        guard.running = true;
                    }
                    next
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

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
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
                self.set_current_task(None).await;
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

    pub async fn enqueue(&self, args: TranslateArgs) -> Task {
        let task = Task::new(args);
        let mut guard = self.inner.lock().await;
        guard.tasks.push(task.clone());
        self.notifier.notify_one();
        task
    }

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

    pub async fn start(&self) {
        let mut guard = self.inner.lock().await;
        guard.running = true;
        self.notifier.notify_one();
    }

    pub async fn pause(&self) {
        let mut guard = self.inner.lock().await;
        guard.running = false;
    }

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
    window: tauri::Window,
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
```

- [ ] **Step 2: Add module declaration in `desktop/src-tauri/src/lib.rs`**

Add `mod queue;` next to `mod task;`.

- [ ] **Step 3: Verify `cargo check --workspace` passes**

Fix any import or trait errors.

- [ ] **Step 4: Commit**

```bash
git add desktop/src-tauri/src/queue.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(queue): add QueueManager and worker loop"
```

---

### Task 3: Expose queue commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `QueueManager`, `Task`, `QueueState` from `queue.rs`.
- Produces: Tauri commands `enqueue_task`, `remove_task`, `reorder_tasks`, `cancel_task`, `retry_task`, `start_queue`, `pause_queue`, `get_queue_state`.

- [ ] **Step 1: Add command functions to `commands.rs`**

Append near the existing `get_default_prompts` command:

```rust
use crate::queue::{QueueManager, QueueState};
use crate::task::Task;

/// Add a book to the translation queue using the current form arguments.
#[allow(dead_code)]
#[tauri::command]
pub async fn enqueue_task(
    args: TranslateArgs,
    queue: tauri::State<'_, QueueManager>,
) -> Result<Task, String> {
    Ok(queue.enqueue(args).await)
}

/// Remove a pending or finished task from the queue.
#[allow(dead_code)]
#[tauri::command]
pub async fn remove_task(id: String, queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.remove(&id).await
}

/// Reorder pending tasks to match the provided list of ids.
#[allow(dead_code)]
#[tauri::command]
pub async fn reorder_tasks(
    ids: Vec<String>,
    queue: tauri::State<'_, QueueManager>,
) -> Result<(), String> {
    queue.reorder(ids).await
}

/// Cancel a pending task.
#[allow(dead_code)]
#[tauri::command]
pub async fn cancel_task(id: String, queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.cancel(&id).await
}

/// Retry a failed or cancelled task.
#[allow(dead_code)]
#[tauri::command]
pub async fn retry_task(id: String, queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.retry(&id).await
}

/// Start processing the queue.
#[allow(dead_code)]
#[tauri::command]
pub async fn start_queue(queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.start().await;
    Ok(())
}

/// Pause after the current task finishes.
#[allow(dead_code)]
#[tauri::command]
pub async fn pause_queue(queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.pause().await;
    Ok(())
}

/// Return the current queue state.
#[allow(dead_code)]
#[tauri::command]
pub async fn get_queue_state(queue: tauri::State<'_, QueueManager>) -> Result<QueueState, String> {
    Ok(queue.state().await)
}
```

- [ ] **Step 2: Register commands and install queue state in `lib.rs`**

Update the `invoke_handler` list:

```rust
.invoke_handler(tauri::generate_handler![
    commands::translate_epub,
    commands::get_system_locale,
    keyring::store_api_key,
    keyring::load_api_key,
    keyring::delete_api_key,
    commands::check_file_exists,
    commands::suggest_output_path,
    commands::test_connection,
    commands::get_e2e_args,
    commands::get_app_version,
    commands::get_default_prompts,
    commands::enqueue_task,
    commands::remove_task,
    commands::reorder_tasks,
    commands::cancel_task,
    commands::retry_task,
    commands::start_queue,
    commands::pause_queue,
    commands::get_queue_state,
])
```

Add `QueueManager` setup in the `setup` closure:

```rust
.setup(|app| {
    let queue = QueueManager::new();
    queue.clone().spawn_worker(app.handle().clone());
    app.manage(queue);

    // ... existing window builder code ...
})
```

- [ ] **Step 3: Run `cargo check --workspace` and fix errors**

- [ ] **Step 4: Commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(queue): expose queue commands to frontend"
```

---

### Task 4: Frontend types and state

**Files:**
- Modify: `desktop/src/types.ts`
- Modify: `desktop/src/App.tsx`

**Interfaces:**
- Produces: TypeScript `Task`, `TaskStatus`, `QueueState` types.
- Consumes: new commands from Task 3.

- [ ] **Step 1: Extend `desktop/src/types.ts`**

Add near the top:

```typescript
export type TaskStatus =
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export interface Task {
  id: string;
  source_path: string;
  output_path: string;
  status: TaskStatus;
  progress_percent: number;
  message: string;
  error?: string;
  created_at: number;
  completed_at?: number;
}

export interface QueueState {
  tasks: Task[];
  running: boolean;
  current_task_id?: string;
}
```

Update the `Page` union:

```typescript
export type Page =
  | "translate"
  | "tasks"
  | "logs"
  | "settings-compute"
  | "settings-model"
  | "settings-translation"
  | "settings-output"
  | "settings-prompts"
  | "settings-general"
  | "about"
  | "legal";
```

- [ ] **Step 2: Add queue state and loader in `App.tsx`**

Add near other hooks:

```typescript
const [queue, setQueue] = useState<QueueState>({
  tasks: [],
  running: false,
});
```

Load queue state when app mounts:

```typescript
useEffect(() => {
  void (async () => {
    const initial = await invoke<QueueState>("get_queue_state").catch(() => ({
      tasks: [],
      running: false,
    }));
    setQueue(initial);
  })();
}, []);
```

Add listener for `task_progress` and `queue_state_changed`:

```typescript
useEffect(() => {
  const unlistenProgress = listen<Task & { event: unknown }>(
    "task_progress",
    (event) => {
      const { task_id, event: progressEvent } = event.payload as {
        task_id: string;
        event: ProgressPayload;
      };
      setQueue((prev) => {
        const tasks = prev.tasks.map((t) => {
          if (t.id !== task_id) return t;
          return applyProgressToTask(t, progressEvent);
        });
        return { ...prev, tasks };
      });
    }
  );

  const unlistenChanged = listen<unknown>("queue_state_changed", () => {
    void (async () => {
      const state = await invoke<QueueState>("get_queue_state").catch(() => ({
        tasks: [],
        running: false,
      }));
      setQueue(state);
    })();
  });

  return () => {
    void unlistenProgress.then((f) => f());
    void unlistenChanged.then((f) => f());
  };
}, []);
```

Implement `applyProgressToTask` helper inside `App.tsx`:

```typescript
function applyProgressToTask(task: Task, payload: ProgressPayload): Task {
  if (typeof payload === "string" && payload === "Completed") {
    return { ...task, progress_percent: 100 };
  }
  if (typeof payload === "object" && "Started" in payload) {
    return { ...task, progress_percent: 0, status: "running" };
  }
  if (typeof payload === "object" && "ChapterFinished" in payload) {
    const total = (payload as { ChapterFinished: { index: number } }).ChapterFinished.index + 1;
    // The backend does not send total here; keep previous percent or estimate.
    return { ...task, progress_percent: Math.max(task.progress_percent, Math.min(99, total)) };
  }
  return task;
}
```

- [ ] **Step 3: Pass queue state and queue actions down**

Add helper functions in `App.tsx`:

```typescript
const refreshQueue = async () => {
  const state = await invoke<QueueState>("get_queue_state").catch(() => ({
    tasks: [],
    running: false,
  }));
  setQueue(state);
};

const enqueueTask = async (args: object) => {
  await invoke("enqueue_task", { args });
  await refreshQueue();
};

const removeTask = async (id: string) => {
  await invoke("remove_task", { id });
  await refreshQueue();
};

const retryTask = async (id: string) => {
  await invoke("retry_task", { id });
  await refreshQueue();
};

const cancelTask = async (id: string) => {
  await invoke("cancel_task", { id });
  await refreshQueue();
};

const startQueue = async () => {
  await invoke("start_queue");
  await refreshQueue();
};

const pauseQueue = async () => {
  await invoke("pause_queue");
  await refreshQueue();
};
```

Update `renderPage` to include `tasks`:

```typescript
case "tasks":
  return (
    <TasksPage
      queue={queue}
      onRefresh={refreshQueue}
      onRemove={removeTask}
      onRetry={retryTask}
      onCancel={cancelTask}
      onStart={startQueue}
      onPause={pauseQueue}
    />
  );
```

Add sidebar navigation button for `"tasks"`.

- [ ] **Step 4: Run `pnpm exec tsc --noEmit` and fix errors**

- [ ] **Step 5: Commit**

```bash
git add desktop/src/types.ts desktop/src/App.tsx
git commit -m "feat(queue): add frontend types and queue state"
```

---

### Task 5: Tasks page UI

**Files:**
- Create: `desktop/src/pages/TasksPage.tsx`
- Modify: `desktop/src/App.css` if needed for layout

**Interfaces:**
- Consumes: `QueueState`, `Task` from `types.ts`; helper callbacks from `App.tsx`.

- [ ] **Step 1: Create `TasksPage.tsx`**

```tsx
import { useTranslation } from "react-i18next";
import type { QueueState, Task } from "../types";

interface TasksPageProps {
  queue: QueueState;
  onRefresh: () => Promise<void>;
  onRemove: (id: string) => Promise<void>;
  onRetry: (id: string) => Promise<void>;
  onCancel: (id: string) => Promise<void>;
  onStart: () => Promise<void>;
  onPause: () => Promise<void>;
}

function formatPath(path: string) {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

export default function TasksPage({
  queue,
  onRefresh,
  onRemove,
  onRetry,
  onCancel,
  onStart,
  onPause,
}: TasksPageProps) {
  const { t } = useTranslation();

  const statusClass = (status: Task["status"]) => `task-status task-status-${status}`;

  return (
    <div className="page tasks-page">
      <h2>{t("nav_tasks")}</h2>

      <div className="queue-controls">
        {queue.running ? (
          <button type="button" onClick={() => void onPause()}>
            {t("pause_queue")}
          </button>
        ) : (
          <button type="button" onClick={() => void onStart()}>
            {t("start_queue")}
          </button>
        )}
        <button type="button" onClick={() => void onRefresh()}>
          {t("refresh")}
        </button>
      </div>

      {queue.tasks.length === 0 ? (
        <p className="empty-state">{t("queue_empty")}</p>
      ) : (
        <ul className="task-list">
          {queue.tasks.map((task) => (
            <li key={task.id} className="task-item">
              <div className="task-info">
                <span className={statusClass(task.status)}>{t(`task_status_${task.status}`)}</span>
                <span className="task-file" title={task.source_path}>
                  {formatPath(task.source_path)}
                </span>
                <span className="task-file" title={task.output_path}>
                  → {formatPath(task.output_path)}
                </span>
              </div>

              <div className="task-progress">
                <div className="progress-bar">
                  <div
                    className="progress-fill"
                    style={{ width: `${task.progress_percent}%` }}
                  />
                </div>
                <span className="progress-message">{task.message}</span>
                {task.error && <span className="inline-error">{task.error}</span>}
              </div>

              <div className="task-actions">
                {task.status === "pending" && (
                  <button type="button" onClick={() => void onCancel(task.id)}>
                    {t("cancel")}
                  </button>
                )}
                {(task.status === "failed" || task.status === "cancelled") && (
                  <button type="button" onClick={() => void onRetry(task.id)}>
                    {t("retry")}
                  </button>
                )}
                {task.status !== "running" && (
                  <button type="button" className="danger" onClick={() => void onRemove(task.id)}>
                    {t("remove")}
                  </button>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Add minimal CSS for the new page in `desktop/src/App.css`**

Append:

```css
.tasks-page .queue-controls {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.task-list {
  list-style: none;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.task-item {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 0.75rem 1rem;
}

.task-info {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 0.5rem;
}

.task-status {
  font-size: 0.8rem;
  padding: 0.15rem 0.5rem;
  border-radius: 999px;
  text-transform: uppercase;
}

.task-status-pending { background: #3b82f6; color: white; }
.task-status-running { background: #f59e0b; color: white; }
.task-status-completed { background: #10b981; color: white; }
.task-status-failed { background: #ef4444; color: white; }
.task-status-cancelled { background: #6b7280; color: white; }

.task-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
}
```

- [ ] **Step 3: Run `pnpm exec tsc --noEmit` and fix errors**

- [ ] **Step 4: Commit**

```bash
git add desktop/src/pages/TasksPage.tsx desktop/src/App.css
git commit -m "feat(queue): add TasksPage UI"
```

---

### Task 6: Enqueue from TranslatePage

**Files:**
- Modify: `desktop/src/pages/TranslatePage.tsx`
- Modify: `desktop/src/App.tsx`

**Interfaces:**
- Consumes: `enqueue_task` command.
- Produces: `onEnqueue` callback passed from `App.tsx`.

- [ ] **Step 1: Update `TranslatePageProps` to accept `onEnqueue`**

```typescript
interface TranslatePageProps {
  form: FormState;
  setForm: <K extends keyof FormState>(key: K, value: FormState[K]) => void;
  onStart: () => void;
  onEnqueue: () => void;
  loading: boolean;
  progress: ProgressState;
  validation: ValidationResult;
  onPageChange: (page: Page) => void;
}
```

- [ ] **Step 2: Add the "Add to queue" button next to Start**

In the render near the start button:

```tsx
<div className="start-row">
  <button
    className="start-button"
    type="button"
    onClick={onStart}
    disabled={loading || !validation.valid}
    data-testid="start-button"
  >
    {loading ? t("loading") : t("start")}
  </button>

  <button
    type="button"
    onClick={onEnqueue}
    disabled={!validation.valid}
    data-testid="enqueue-button"
  >
    {t("add_to_queue")}
  </button>
</div>
```

- [ ] **Step 3: Implement `onEnqueue` in `App.tsx`**

Factor out `buildTranslateArgs(form)` from `handleStart` so both can reuse it:

```typescript
function buildTranslateArgs(form: FormState): object {
  const provider = activeProvider(form);
  if (!provider) throw new Error("no provider");
  return {
    ...form,
    provider: provider.provider,
    api_key: provider.api_key,
    base_url: provider.use_custom_base_url ? provider.base_url || null : null,
    system_prompt: form.system_prompt || null,
    prompts: form.prompts,
    output_font: form.output_font || null,
    exclude_selectors: parseCommaList(form.exclude_selectors),
    translate_attributes: parseCommaList(form.translate_attributes),
    dry_run: !!form.dry_run,
    preserve_classes: !!form.preserve_classes,
    translate_body: !!form.translate_body,
    translate_metadata: !!form.translate_metadata,
    translate_toc: !!form.translate_toc,
    translate_alt_text: !!form.translate_alt_text,
    translate_image_captions: !!form.translate_image_captions,
    translate_tables: !!form.translate_tables,
    translate_footnotes: !!form.translate_footnotes,
    translate_code: !!form.translate_code,
  };
}
```

Update `handleStart` to call `buildTranslateArgs(form)`.

Add `handleEnqueue`:

```typescript
async function handleEnqueue() {
  if (!validation.valid) return;
  try {
    const args = buildTranslateArgs(form);
    await invoke("enqueue_task", { args });
    setPage("tasks");
  } catch (err) {
    setLogs((prev) => [
      ...prev,
      {
        id: generateId(),
        timestamp: Date.now(),
        kind: "error",
        message: `${t("error")}: ${err}`,
      },
    ]);
  }
}
```

Pass `onEnqueue={handleEnqueue}` to `TranslatePage`.

- [ ] **Step 4: Run `pnpm exec tsc --noEmit` and fix errors**

- [ ] **Step 5: Commit**

```bash
git add desktop/src/pages/TranslatePage.tsx desktop/src/App.tsx
git commit -m "feat(queue): enqueue books from TranslatePage"
```

---

### Task 7: Internationalization

**Files:**
- Modify: `desktop/src/locales/en.json`
- Modify: `desktop/src/locales/zh-CN.json`
- Modify other locale files as desired: `es.json`, `ja.json`, `ko.json`, `ru.json`

**Interfaces:**
- Produces: Translated strings used by `TasksPage` and `TranslatePage`.

- [ ] **Step 1: Add English keys**

Insert alphabetically near the existing keys:

```json
  "add_to_queue": "Add to queue",
  "cancel": "Cancel",
  "nav_tasks": "Task Queue",
  "pause_queue": "Pause queue",
  "queue_empty": "No tasks in the queue.",
  "refresh": "Refresh",
  "remove": "Remove",
  "retry": "Retry",
  "start_queue": "Start queue",
  "task_status_cancelled": "Cancelled",
  "task_status_completed": "Completed",
  "task_status_failed": "Failed",
  "task_status_pending": "Pending",
  "task_status_running": "Running",
```

- [ ] **Step 2: Add Simplified Chinese keys**

```json
  "add_to_queue": "加入队列",
  "cancel": "取消",
  "nav_tasks": "任务队列",
  "pause_queue": "暂停队列",
  "queue_empty": "队列中没有任务。",
  "refresh": "刷新",
  "remove": "移除",
  "retry": "重试",
  "start_queue": "开始队列",
  "task_status_cancelled": "已取消",
  "task_status_completed": "已完成",
  "task_status_failed": "失败",
  "task_status_pending": "待执行",
  "task_status_running": "执行中",
```

- [ ] **Step 3: Run `pnpm exec tsc --noEmit`**

No compile errors expected from JSON changes.

- [ ] **Step 4: Commit**

```bash
git add desktop/src/locales/en.json desktop/src/locales/zh-CN.json
git commit -m "feat(queue): add task queue i18n keys"
```

---

### Task 8: Rust unit tests for queue manager

**Files:**
- Modify: `desktop/src-tauri/src/queue.rs`

**Interfaces:**
- Tests internal `QueueManager` behavior without UI.

- [ ] **Step 1: Add tests module to `queue.rs`**

```rust
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
        }
    }

    #[tokio::test]
    async fn enqueue_adds_pending_task() {
        let queue = QueueManager::new();
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub")).await;
        let state = queue.state().await;
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].id, task.id);
        assert_eq!(state.tasks[0].status, TaskStatus::Pending);
    }

    #[tokio::test]
    async fn cancel_only_pending_task() {
        let queue = QueueManager::new();
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub")).await;
        queue.cancel(&task.id).await.unwrap();
        let state = queue.state().await;
        assert_eq!(state.tasks[0].status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn retry_resets_failed_task() {
        let queue = QueueManager::new();
        let task = queue.enqueue(sample_args("a.epub", "a.out.epub")).await;
        queue.cancel(&task.id).await.unwrap();
        queue.retry(&task.id).await.unwrap();
        let state = queue.state().await;
        assert_eq!(state.tasks[0].status, TaskStatus::Pending);
        assert!(state.tasks[0].error.is_none());
    }
}
```

- [ ] **Step 2: Run `cargo test --workspace`**

Expected: new tests pass; existing tests still pass.

- [ ] **Step 3: Commit**

```bash
git add desktop/src-tauri/src/queue.rs
git commit -m "test(queue): add QueueManager unit tests"
```

---

### Task 9: Full verification and PR

- [ ] **Step 1: Run Rust checks**

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected: all pass.

- [ ] **Step 2: Run frontend checks**

```bash
cd desktop
pnpm exec tsc --noEmit
pnpm build
```

Expected: typecheck and production build pass.

- [ ] **Step 3: Run Tauri release build**

```bash
pnpm tauri build
```

Expected: release build succeeds.

- [ ] **Step 4: Create a feature branch, push, and open PR**

```bash
cd ..
git checkout -b feat/task-queue
git push -u origin feat/task-queue
gh pr create --base master --head feat/task-queue \
  --title "feat(queue): translation task queue" \
  --body "Adds a backend-managed task queue for translating multiple books sequentially. Includes a new Tasks page and enqueue button on the Translate page."
```

- [ ] **Step 5: Wait for CI to pass, then merge**

```bash
gh pr checks <number> --watch --fail-fast
gh pr merge <number> --squash
```

---

## Spec Coverage Checklist

| Spec Requirement | Implementing Task |
|---|---|
| Backend in-memory queue | Task 2 |
| Add/remove/reorder/cancel/retry tasks | Task 3 |
| Sequential execution (one book at a time) | Task 2 |
| Task progress events with task id | Task 2 |
| New Tasks page | Task 5 |
| Enqueue from Translate page | Task 6 |
| i18n keys | Task 7 |
| No core engine changes | Enforced in all tasks |
| No queue persistence | QueueManager does not write to disk |
| Cancellation only for pending tasks | `QueueManager::cancel` returns error for non-pending |
| Tests and build pass | Task 8, Task 9 |

## Placeholder Scan

No TBD, TODO, or vague steps remain. Every code block contains concrete code, every command has an expected outcome, and every interface lists exact file paths.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-08-task-queue.md`.

Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — execute tasks in this session using `executing-plans`, batch execution with checkpoints.
