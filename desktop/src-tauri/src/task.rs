//! Task data model for the translation queue.

use crate::args::{PdfToEpubArgs, TranslateArgs};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// The kind of job a queued task runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum TaskKind {
    /// Translate a source book into a target language.
    Translation(TranslateArgs),
    /// Convert a scanned PDF to an EPUB via OCR (+ optional verify/refine).
    Ocr(PdfToEpubArgs),
    /// Convert a scanned PDF to EPUB via OCR, then translate the resulting EPUB.
    /// The OCR output path feeds the translation source.
    OcrThenTranslate(PdfToEpubArgs, TranslateArgs),
}

impl TaskKind {
    /// Path of the source file consumed by the task.
    pub fn source_path(&self) -> String {
        match self {
            Self::Translation(a) => a.source.clone(),
            Self::Ocr(a) | Self::OcrThenTranslate(a, _) => a.pdf_path.clone(),
        }
    }

    /// Path where the task writes its output.
    pub fn output_path(&self) -> String {
        match self {
            Self::Translation(t) | Self::OcrThenTranslate(_, t) => t.output.clone(),
            Self::Ocr(a) => a.output_path.clone(),
        }
    }
}

/// Lifecycle status of a queued translation task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    /// Paused by the user. The task was running and has been cancelled; it can
    /// be resumed from the last checkpoint.
    Paused,
}

/// A single queued translation job.
#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: String,
    pub source_path: String,
    pub output_path: String,
    pub status: TaskStatus,
    pub progress_percent: u32,
    /// Total number of translatable chapters, populated when the task starts.
    pub chapter_total: Option<u32>,
    /// Number of chapters that have already finished, used to resume progress
    /// after a queue state refresh.
    pub chapters_completed: Option<u32>,
    pub message: String,
    pub error: Option<String>,
    /// Full job arguments. Skipped during serialization so provider secrets and
    /// large argument structs are never sent to the frontend.
    #[serde(skip)]
    pub kind: TaskKind,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

impl Task {
    /// Create a new pending task from the job arguments captured at enqueue time.
    pub fn new(kind: TaskKind) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .inspect_err(|err| tracing::warn!("system time is before Unix epoch: {err}"))
            .unwrap_or_default()
            .as_secs();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_path: kind.source_path(),
            output_path: kind.output_path(),
            status: TaskStatus::Pending,
            progress_percent: 0,
            chapter_total: None,
            chapters_completed: None,
            message: String::new(),
            error: None,
            kind,
            created_at: now,
            started_at: None,
            completed_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{PdfToEpubArgs, PromptTemplates, TranslateArgs};

    fn sample_args() -> TranslateArgs {
        TranslateArgs {
            source: "input.epub".to_string(),
            output: "output.epub".to_string(),
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

    #[test]
    fn task_new_starts_pending_with_uuid() {
        let task = Task::new(TaskKind::Translation(sample_args()));
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(!task.id.is_empty());
        assert_eq!(task.source_path, "input.epub");
        assert_eq!(task.output_path, "output.epub");
        assert_eq!(task.progress_percent, 0);
        assert!(task.chapter_total.is_none());
        assert!(task.chapters_completed.is_none());
    }

    #[test]
    fn task_status_serializes_to_snake_case() {
        let value = serde_json::to_value(TaskStatus::Failed).unwrap();
        assert_eq!(value, "failed");
        let value = serde_json::to_value(TaskStatus::Paused).unwrap();
        assert_eq!(value, "paused");
    }

    #[test]
    fn pipeline_task_paths_span_both_stages() {
        let ocr_args = PdfToEpubArgs {
            pdf_path: "scan.pdf".to_string(),
            output_path: "intermediate.epub".to_string(),
            ocr_api_key: "k".to_string(),
            ..Default::default()
        };
        let mut translate_args = sample_args();
        translate_args.output = "final.epub".to_string();
        let task = Task::new(TaskKind::OcrThenTranslate(ocr_args, translate_args));
        // Source is the PDF (stage-1 input); output is the bilingual EPUB (stage-2 output).
        assert_eq!(task.source_path, "scan.pdf");
        assert_eq!(task.output_path, "final.epub");
    }

    #[test]
    fn pipeline_task_kind_round_trips_through_serde() {
        let ocr_args = PdfToEpubArgs {
            pdf_path: "scan.pdf".to_string(),
            output_path: "intermediate.epub".to_string(),
            ocr_api_key: "k".to_string(),
            ..Default::default()
        };
        let kind = TaskKind::OcrThenTranslate(ocr_args, sample_args());
        let json = serde_json::to_string(&kind).unwrap();
        let restored: TaskKind = serde_json::from_str(&json).unwrap();
        assert!(matches!(restored, TaskKind::OcrThenTranslate(_, _)));
        assert_eq!(restored.source_path(), "scan.pdf");
        assert_eq!(restored.output_path(), "output.epub");
    }
}
