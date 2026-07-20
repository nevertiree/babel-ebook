//! Batch import planning: turn a list of source file paths into translation
//! tasks, skipping unsupported formats and duplicate sources, and resolving
//! output-path collisions within the batch.
//!
//! This module is intentionally pure: it does not touch the queue state or the
//! filesystem, so the import heuristics can be unit-tested in isolation.

#![allow(clippy::literal_string_with_formatting_args)]

use std::collections::HashSet;
use std::path::Path;

use crate::args::TranslateArgs;
use serde::Serialize;

/// Why a source file was skipped during batch import.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// The file extension is not a supported input format.
    UnsupportedFormat,
    /// The same source file is already present in the queue.
    DuplicateInQueue,
    /// The same source file appeared earlier in this batch.
    DuplicateInBatch,
}

/// A source file that was skipped, together with the reason.
#[derive(Debug, Clone, Serialize)]
pub struct SkippedSource {
    /// The source path that was skipped.
    pub path: String,
    /// Why the source was skipped.
    pub reason: SkipReason,
}

/// The outcome of planning a batch import.
#[derive(Debug, Clone, Serialize)]
pub struct BatchPlan {
    /// Translation arguments ready to be enqueued, one per accepted source.
    pub enqueued: Vec<TranslateArgs>,
    /// Sources that were skipped, with reasons.
    pub skipped: Vec<SkippedSource>,
}

/// Input formats supported for translation, mirroring the frontend file filter.
const SUPPORTED_EXTENSIONS: &[&str] = &["epub", "mobi", "azw3", "txt", "srt", "docx"];

/// Plan a batch import.
///
/// Given a list of source file paths and a set of shared translation
/// arguments, produce one [`TranslateArgs`] per accepted source. Sources are
/// skipped when their format is unsupported or when they duplicate a file
/// already queued (or seen earlier in the same batch). Output paths are derived
/// from `output_filename_template` and de-duplicated within the batch by
/// appending a numeric suffix when collisions occur.
///
/// This function performs no filesystem access: callers that want to avoid
/// overwriting existing files should pass those paths via
/// `existing_sources_in_queue` or check the planned outputs afterwards.
#[allow(clippy::needless_pass_by_value)]
pub fn plan_batch(
    sources: &[String],
    base: &TranslateArgs,
    output_filename_template: &str,
    existing_sources_in_queue: &[String],
) -> BatchPlan {
    let mut enqueued: Vec<TranslateArgs> = Vec::new();
    let mut skipped: Vec<SkippedSource> = Vec::new();
    let mut seen_sources: HashSet<String> = HashSet::new();
    let mut seen_outputs: HashSet<String> = HashSet::new();

    for source in sources {
        if !is_supported_format(source) {
            skipped.push(SkippedSource {
                path: source.clone(),
                reason: SkipReason::UnsupportedFormat,
            });
            continue;
        }
        if existing_sources_in_queue.iter().any(|s| s == source) {
            skipped.push(SkippedSource {
                path: source.clone(),
                reason: SkipReason::DuplicateInQueue,
            });
            continue;
        }
        if !seen_sources.insert(source.clone()) {
            skipped.push(SkippedSource {
                path: source.clone(),
                reason: SkipReason::DuplicateInBatch,
            });
            continue;
        }

        let base_output = render_output_path(source, base, output_filename_template);
        let output = resolve_collision(&base_output, &seen_outputs);
        seen_outputs.insert(output.clone());

        let mut args = base.clone();
        args.source.clone_from(source);
        args.output = output;
        enqueued.push(args);
    }

    BatchPlan { enqueued, skipped }
}

/// Whether the file at `path` has a supported input extension.
fn is_supported_format(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Render an output path for `source` by substituting placeholders in
/// `template` and appending `.epub`. The result is placed beside the source
/// file (or in the current directory when the source has no parent).
fn render_output_path(source: &str, base: &TranslateArgs, template: &str) -> String {
    let source_path = Path::new(source);
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let parent = source_path
        .parent()
        .and_then(|p| p.to_str())
        .filter(|p| !p.is_empty())
        .unwrap_or(".");
    let active_template = if template.trim().is_empty() {
        "{stem}_{target_lang}"
    } else {
        template
    };
    let rendered = active_template
        .replace("{stem}", stem)
        .replace("{source_lang}", &base.source_lang)
        .replace("{target_lang}", &base.target_lang)
        .replace("{output_mode}", &base.output_mode);
    let filename = format!("{rendered}.epub");
    if parent == "." {
        filename
    } else {
        Path::new(parent)
            .join(&filename)
            .to_string_lossy()
            .into_owned()
    }
}

/// Return a path that does not collide with any entry in `taken`. When
/// `base_output` is already taken, a numeric suffix is inserted before the
/// extension, e.g. `book (1).epub`.
fn resolve_collision(base_output: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(base_output) {
        return base_output.to_string();
    }
    let mut idx = 1;
    loop {
        let candidate = with_suffix(base_output, idx);
        if !taken.contains(&candidate) {
            return candidate;
        }
        idx += 1;
    }
}

/// Insert ` (idx)` before the extension of `path`.
fn with_suffix(path: &str, idx: usize) -> String {
    let p = Path::new(path);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("epub");
    let parent = p
        .parent()
        .and_then(|p| p.to_str())
        .filter(|p| !p.is_empty())
        .unwrap_or(".");
    let filename = format!("{stem} ({idx}).{ext}");
    if parent == "." {
        filename
    } else {
        Path::new(parent)
            .join(&filename)
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::PromptTemplates;

    fn base_args() -> TranslateArgs {
        TranslateArgs {
            source: String::new(),
            output: String::new(),
            provider: "deepseek".to_string(),
            api_key: "k".to_string(),
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
    fn enqueues_supported_formats_and_skips_unsupported() {
        let base = base_args();
        let plan = plan_batch(
            &[
                "a.epub".to_string(),
                "b.mobi".to_string(),
                "bad.xyz".to_string(),
            ],
            &base,
            "{stem}_{target_lang}",
            &[],
        );
        assert_eq!(plan.enqueued.len(), 2);
        assert_eq!(plan.enqueued[0].source, "a.epub");
        assert_eq!(plan.enqueued[1].source, "b.mobi");
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].path, "bad.xyz");
        assert_eq!(plan.skipped[0].reason, SkipReason::UnsupportedFormat);
    }

    #[test]
    fn skips_duplicate_sources_in_queue_and_in_batch() {
        let base = base_args();
        let plan = plan_batch(
            &[
                "a.epub".to_string(),
                "b.epub".to_string(),
                "a.epub".to_string(),
            ],
            &base,
            "{stem}_{target_lang}",
            &["b.epub".to_string()],
        );
        assert_eq!(plan.enqueued.len(), 1);
        assert_eq!(plan.enqueued[0].source, "a.epub");
        assert_eq!(plan.skipped.len(), 2);
        assert_eq!(plan.skipped[0].path, "b.epub");
        assert_eq!(plan.skipped[0].reason, SkipReason::DuplicateInQueue);
        assert_eq!(plan.skipped[1].path, "a.epub");
        assert_eq!(plan.skipped[1].reason, SkipReason::DuplicateInBatch);
    }

    #[test]
    fn resolves_output_path_collisions_with_suffix() {
        let base = base_args();
        let plan = plan_batch(
            &["a.epub".to_string(), "a.mobi".to_string()],
            &base,
            "{stem}_{target_lang}",
            &[],
        );
        assert_eq!(plan.enqueued.len(), 2);
        assert_eq!(plan.enqueued[0].output, "a_zh-CN.epub");
        assert_eq!(plan.enqueued[1].output, "a_zh-CN (1).epub");
    }

    #[test]
    fn renders_output_path_from_template() {
        let base = base_args();
        let plan = plan_batch(
            &["my book.epub".to_string()],
            &base,
            "{stem}_{output_mode}_{target_lang}",
            &[],
        );
        assert_eq!(plan.enqueued.len(), 1);
        assert_eq!(plan.enqueued[0].output, "my book_bilingual_zh-CN.epub");
    }
}
