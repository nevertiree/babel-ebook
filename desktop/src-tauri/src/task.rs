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
