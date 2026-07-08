# Task Queue Design for BabelEbook Desktop

## Status

Approved approach: **B — backend in-memory queue + worker**.

## Goal

Allow users to add multiple books to a translation queue, view and manage the queue, and let the application process the books sequentially in the background while the user continues to interact with the app.

## Background

The current desktop app only supports translating one book at a time:

- `TranslatePage` calls `translate_epub(args)` directly.
- `commands.rs` runs the full translation synchronously inside a `spawn_blocking` task.
- Progress is emitted through a single `translation_progress` event with no task identity.

This design keeps the core translation engine untouched and builds the queue management layer entirely inside the Tauri desktop crate.

## Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                         Frontend                             │
│  TranslatePage          TasksPage            App (listeners) │
│      │                      │                       │        │
│      └──────────┬───────────┘                       │        │
│                 │ invoke                            │        │
└─────────────────┼───────────────────────────────────┘        │
                  │                                             │
┌─────────────────┼───────────────────────────────────────────┐
│                 ▼                                             │
│   ┌─────────────────────┐     ┌─────────────────────┐        │
│   │   QueueManager      │────▶│    Worker Task      │        │
│   │   (Tauri State)     │     │   (Tokio async)     │        │
│   └─────────────────────┘     └──────────┬──────────┘        │
│              ▲                           │                   │
│              │                           │                   │
│              │        spawn_blocking     │                   │
│              │        translate_epub     │                   │
│              │        internal           │                   │
│              │                           │                   │
│              └───────────────────────────┘                   │
│                                                               │
│   emits: task_progress { task_id, event }                     │
└───────────────────────────────────────────────────────────────┘
```

## Data Model

### Rust

```rust
#[derive(Debug, Clone, Serialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: String,
    pub source_path: String,
    pub output_path: String,
    pub status: TaskStatus,
    pub progress_percent: u32,
    pub message: String,
    pub error: Option<String>,
    pub args: TranslateArgs,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueState {
    pub tasks: Vec<Task>,
    pub running: bool,
    pub current_task_id: Option<String>,
}
```

### TypeScript

```typescript
export type TaskStatus = "pending" | "running" | "completed" | "failed" | "cancelled";

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

The `TranslateArgs` snapshot captured at enqueue time is opaque to the frontend queue view, but is required by the worker to run the task independently of the current form state.

## Backend Components

### `desktop/src-tauri/src/task.rs`

- Define `Task`, `TaskStatus`, and helper constructors/converters.

### `desktop/src-tauri/src/queue.rs`

`QueueManager` holds:

```rust
pub struct QueueManager {
    inner: Arc<Mutex<QueueInner>>,
    notifier: Arc<Notify>,
}

struct QueueInner {
    tasks: Vec<Task>,
    running: bool,
    current_task_id: Option<String>,
}
```

Public methods:

- `enqueue(args: TranslateArgs) -> Task`
- `remove(id: &str) -> Result<(), String>`
- `reorder(ids: Vec<String>) -> Result<(), String>`
- `cancel(id: &str) -> Result<(), String>`
- `start()` / `pause()`
- `state() -> QueueState`

The worker loop:

```rust
async fn worker_loop(manager: QueueManager) {
    loop {
        // wait until running and a pending task exists
        // pick next pending task, mark running
        // run translation via translate_epub_internal with a TaskProgressCallback
        // mark completed/failed, then loop
    }
}
```

### Progress Callback

A new `TaskProgressCallback` implements `ProgressCallback` and emits `task_progress` events:

```rust
pub struct TaskProgressCallback {
    window: tauri::Window,
    task_id: String,
}

impl ProgressCallback for TaskProgressCallback {
    fn on_progress(&self, event: ProgressEvent) {
        let _ = self.window.emit("task_progress", TaskProgressPayload {
            task_id: self.task_id.clone(),
            event,
        });
    }
}
```

### Commands

Add to `commands.rs` and register in `lib.rs`:

- `enqueue_task(args: TranslateArgs) -> Task`
- `remove_task(id: String)`
- `reorder_tasks(ids: Vec<String>)`
- `cancel_task(id: String)`
- `start_queue()` / `pause_queue()`
- `get_queue_state() -> QueueState`

The worker task is spawned once during Tauri setup.

## Frontend Components

### New page: `desktop/src/pages/TasksPage.tsx`

Displays:

- Queue controls: Start / Pause.
- Task list with columns: source file, output file, status, progress bar, message, actions.
- Actions per task: Remove, Retry (for failed), Cancel (for pending).

### Update `TranslatePage.tsx`

Add an **“Add to queue”** button next to the existing **“Start”** button. When clicked:

1. Build `TranslateArgs` from current form (same as `handleStart`).
2. Call `invoke("enqueue_task", { args })`.
3. Optionally navigate to the Tasks page.

### Update `App.tsx`

- Add `"tasks"` to the `Page` union and sidebar navigation.
- Add a listener for `task_progress` events that updates a new `queue` state.
- Pass queue state and updaters to `TasksPage`.

### State Management

Queue state lives in React state at the `App` level. It is loaded on demand via `get_queue_state` when the Tasks page opens and kept in sync by `task_progress` events.

No queue state is persisted to disk in this iteration.

## Concurrency

- **Book-level concurrency**: `max_parallel_tasks = 1` for the first version. The worker runs one book at a time.
- **Chapter-level concurrency**: unchanged. Inside a running book, the existing `concurrency` setting still controls how many chapter requests are sent to the LLM in parallel.

This keeps provider rate-limit risk low and avoids changing the core engine.

## Error Handling

- A failed task remains in the queue with `status: Failed` and `error` populated.
- Users can click **Retry** to reset the task to `Pending`.
- **Cancel** only affects `Pending` tasks. Cancelling a `Running` task is not supported in this iteration because the core translator does not expose a cancellation token.

## i18n

Add keys for the new UI (English + Simplified Chinese at minimum):

- `nav_tasks`, `add_to_queue`, `start_queue`, `pause_queue`, `retry`, `remove`, `cancel`, `queue_empty`, etc.

## Testing & Verification

- `cargo test --workspace` must pass.
- `cargo clippy --workspace --all-targets -- -D warnings` must pass.
- `pnpm exec tsc --noEmit` must pass.
- `pnpm tauri build` must pass.

Manual verification:

1. Add two different books to the queue.
2. Start the queue.
3. Observe that the first book runs, completes, and the output file is created.
4. Observe that the second book starts automatically.
5. Verify that removing a pending task and retrying a failed task work.

## Scope

**In scope:**

- `desktop/src-tauri/src/task.rs`
- `desktop/src-tauri/src/queue.rs`
- Extensions to `desktop/src-tauri/src/commands.rs`
- Registration in `desktop/src-tauri/src/lib.rs`
- `desktop/src/pages/TasksPage.tsx`
- Changes to `desktop/src/pages/TranslatePage.tsx`
- Changes to `desktop/src/App.tsx`
- i18n keys in locale files

**Out of scope:**

- Persisting the queue across app restarts.
- Cancelling a running translation mid-chapter.
- Running more than one book at a time.
- Any changes to `crates/babel-ebook` core translation logic.

## Stop Rule

If it becomes clear that task-level progress or cancellation cannot be implemented without a large refactor of the core translation engine, stop and report the blocker instead of modifying core code.
