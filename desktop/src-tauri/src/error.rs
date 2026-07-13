//! Shared error type for the Tauri desktop backend.
//!
//! Returning `Result<T, String` from every command makes it easy to lose error
//! context and hard to enforce consistent messages. `AppError` centralises the
//! domain errors produced by the queue and store layers; commands convert them
//! to strings at the Tauri boundary so the frontend contract stays unchanged.

use std::fmt;

/// Domain errors that can occur in the desktop backend.
#[derive(Debug)]
pub enum AppError {
    /// Persisted task store failure.
    Store(String),
    /// Requested task does not exist.
    TaskNotFound,
    /// A running task cannot be removed.
    CannotRemoveRunningTask,
    /// Only pending or running tasks may be cancelled.
    CancelInvalidStatus,
    /// No task is currently running.
    NoRunningTask,
    /// The requested task is not the one currently running.
    NotCurrentTask,
    /// The requested task is not in a running state.
    TaskNotRunning,
    /// Only paused tasks can be resumed.
    ResumeInvalidStatus,
    /// Only failed, cancelled or paused tasks can be retried.
    RetryInvalidStatus,
    /// A running task cannot be reordered.
    CannotReorderRunningTask,
    /// Reorder list referenced an unknown task id.
    UnknownTaskId(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(msg) => write!(f, "store error: {msg}"),
            Self::TaskNotFound => write!(f, "task not found"),
            Self::CannotRemoveRunningTask => write!(f, "cannot remove a running task"),
            Self::CancelInvalidStatus => {
                write!(f, "only pending or running tasks can be cancelled")
            }
            Self::NoRunningTask => write!(f, "no running task"),
            Self::NotCurrentTask => write!(f, "task is not currently running"),
            Self::TaskNotRunning => write!(f, "task is not running"),
            Self::ResumeInvalidStatus => write!(f, "only paused tasks can be resumed"),
            Self::RetryInvalidStatus => {
                write!(f, "only failed, cancelled or paused tasks can be retried")
            }
            Self::CannotReorderRunningTask => write!(f, "cannot reorder a running task"),
            Self::UnknownTaskId(id) => write!(f, "unknown task id: {id}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::Store(err.to_string())
    }
}
