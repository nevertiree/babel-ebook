//! Core orchestration for the babel-ebook pipeline.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use serde::Serialize;

use crate::cache::TranslationCache;
use crate::checkpoint::CheckpointStore;
use crate::chunking::count_tokens;
use crate::config::Config;
use crate::epub::{should_translate_doc, write_epub};
use crate::html::translate_text;
use crate::input_formats::read_input_book;
use crate::pipeline::run_ordered_pipeline;
use crate::t;
use crate::translator::Translator;

/// Cooperative cancellation signal for long-running translations.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Request cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Return whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Progress events emitted during `translate_epub`.
#[derive(Debug, Clone, Serialize)]
pub enum ProgressEvent {
    /// Translation has started with the given number of translatable chapters.
    Started {
        /// Number of chapters that will be translated.
        total: usize,
    },
    /// A chapter has started translation.
    ChapterStarted {
        /// Index of the chapter in the EPUB spine.
        index: usize,
        /// Href of the chapter document.
        href: String,
    },
    /// A chapter has finished translation successfully.
    ChapterFinished {
        /// Index of the chapter in the EPUB spine.
        index: usize,
        /// Href of the chapter document.
        href: String,
    },
    /// A chapter failed to translate.
    Failed {
        /// Index of the chapter in the EPUB spine.
        index: usize,
        /// Href of the chapter document.
        href: String,
        /// Error message describing the failure.
        error: String,
    },
    /// Translation has completed and the output EPUB has been written.
    Completed,
}

/// Callback trait for reporting progress during `translate_epub`.
///
/// Implementations must be thread-safe because progress may be reported
/// from multiple concurrent chapter translation tasks.
pub trait ProgressCallback: Send + Sync {
    /// Called whenever a progress event occurs.
    fn on_progress(&self, event: ProgressEvent);
}

/// Unified error type for the babel-ebook crate.
#[derive(Debug, thiserror::Error)]
pub enum BabelEbookError {
    /// Translation was cancelled by the caller.
    Cancelled,

    /// An API request failed after exhausting retries.
    ApiError(String),

    /// The requested translation provider is not supported.
    ProviderNotFound(String),

    /// Invalid configuration value.
    Configuration(String),

    /// A catch-all for unexpected errors.
    Anyhow(#[from] anyhow::Error),
}

impl std::fmt::Display for BabelEbookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "translation cancelled"),
            Self::ApiError(msg) => write!(f, "{}", t!("err_api", msg = msg.as_str())),
            Self::ProviderNotFound(provider) => {
                write!(
                    f,
                    "{}",
                    t!("err_provider_not_found", provider = provider.as_str())
                )
            }
            Self::Configuration(msg) => {
                write!(f, "{}", t!("err_configuration", msg = msg.as_str()))
            }
            Self::Anyhow(err) => err.fmt(f),
        }
    }
}

/// Translate an EPUB according to *config* using *translator*.
///
/// Chapters are filtered through `should_translate_doc` and processed with
/// bounded concurrency. Failures for individual chapters are recorded and
/// logged, but the overall operation continues and writes the (partially)
/// updated EPUB to `config.output`.
///
/// If *progress* is provided, lifecycle events are emitted at the start and
/// end of the operation and for each chapter as it is processed.
///
/// The returned future is not `Send` because `kuchiki` uses `Rc` internally.
/// Callers that need a `Send` future should run the work on a local runtime
/// (for example Tauri does this via `spawn_blocking`).
#[allow(clippy::future_not_send)]
pub async fn translate_epub(
    config: &Config,
    translator: &dyn Translator,
    cache: Option<&TranslationCache>,
    progress: Option<&dyn ProgressCallback>,
) -> Result<(), BabelEbookError> {
    translate_epub_with_cancellation(config, translator, cache, progress, None).await
}

/// Translate an EPUB with an optional cooperative cancellation signal.
#[allow(clippy::future_not_send)]
pub async fn translate_epub_with_cancellation(
    config: &Config,
    translator: &dyn Translator,
    cache: Option<&TranslationCache>,
    progress: Option<&dyn ProgressCallback>,
    cancellation: Option<&CancellationToken>,
) -> Result<(), BabelEbookError> {
    tracing::info!(
        source = %config.source.display(),
        output = %config.output.display(),
        "starting EPUB translation"
    );

    ensure_not_cancelled(cancellation)?;

    let owned_cache = cache.map_or_else(
        || TranslationCache::new(config.cache_dir.clone()),
        TranslationCache::clone,
    );
    let cache = &owned_cache;

    let mut book = read_input_book(&config.source)?;
    ensure_not_cancelled(cancellation)?;

    let source_hash = CheckpointStore::source_hash(&config.source)?;
    let translatable_indices = translatable_chapters(&book, &config.skip_doc_patterns)?;
    tracing::info!(
        total_chapters = book.chapters.len(),
        translatable_chapters = translatable_indices.len(),
        "EPUB loaded"
    );

    if config.dry_run {
        run_dry_run(&book, &translatable_indices, progress);
        return Ok(());
    }

    emit_progress(
        progress,
        ProgressEvent::Started {
            total: translatable_indices.len(),
        },
    );

    let (checkpoint_store, job_id) = if config.dry_run {
        (None, None)
    } else {
        let store = CheckpointStore::new(config.checkpoint_dir.clone())?;
        let id = config
            .resume_job_id
            .clone()
            .unwrap_or_else(|| CheckpointStore::generate_job_id(&config.source));
        (Some(store), Some(id))
    };

    let pipeline_result = run_ordered_pipeline(
        &mut book,
        translatable_indices.clone(),
        translator,
        config,
        cache,
        checkpoint_store.as_ref(),
        job_id.as_deref(),
        &source_hash,
        progress,
        cancellation,
    )
    .await?;
    let failures = pipeline_result.failures;

    ensure_not_cancelled(cancellation)?;

    if config.translation_scope.toc {
        let failed_hrefs: std::collections::HashSet<&str> =
            failures.iter().map(|(href, _)| href.as_str()).collect();
        for index in &translatable_indices {
            if let Some(title) = book.chapters[*index].title.clone() {
                let href = book.chapters[*index].href.clone();
                if failed_hrefs.contains(href.as_str()) {
                    continue;
                }
                match translate_title(&title, translator, config, cache, &href).await {
                    Ok(translated) => {
                        ensure_not_cancelled(cancellation)?;
                        book.chapters[*index].title = Some(translated);
                    }
                    Err(err) => {
                        let err_msg = t!(
                            "log_title_translate_failed",
                            href = href.as_str(),
                            error = err.to_string()
                        );
                        tracing::warn!("{err_msg}");
                    }
                }
            }
        }
    }

    ensure_not_cancelled(cancellation)?;

    tracing::info!(output = %config.output.display(), "writing translated EPUB");
    write_epub(&book, &config.output)?;
    tracing::info!(output = %config.output.display(), "EPUB written successfully");
    emit_progress(progress, ProgressEvent::Completed);

    if !failures.is_empty() {
        let documents = failures
            .iter()
            .map(|(href, _)| href.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let msg = t!("log_failed_documents", documents = documents);
        tracing::warn!("{msg}");
    }

    Ok(())
}

fn ensure_not_cancelled(cancellation: Option<&CancellationToken>) -> Result<(), BabelEbookError> {
    if cancellation.is_some_and(CancellationToken::is_cancelled) {
        return Err(BabelEbookError::Cancelled);
    }
    Ok(())
}

fn run_dry_run(
    book: &crate::epub::EpubBook,
    indices: &[usize],
    progress: Option<&dyn ProgressCallback>,
) {
    // `run_dry_run` receives an already-loaded book; the caller uses
    // `read_input_book` before invoking it.
    emit_progress(
        progress,
        ProgressEvent::Started {
            total: indices.len(),
        },
    );
    let (total, count) = estimate_source_tokens(book, indices);
    let msg = t!(
        "log_estimated_tokens",
        total = total.to_string(),
        count = count.to_string()
    );
    tracing::info!("{msg}");
    emit_progress(progress, ProgressEvent::Completed);
}

async fn translate_title(
    title: &str,
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    href: &str,
) -> Result<String, BabelEbookError> {
    let prompt_key = format!("toc:{href}");
    translate_text(title, translator, config, cache, &prompt_key).await
}

fn emit_progress(progress: Option<&dyn ProgressCallback>, event: ProgressEvent) {
    if let Some(p) = progress {
        p.on_progress(event);
    }
}

/// Return the indices of chapters that should be translated.
///
/// # Errors
///
/// Returns `BabelEbookError::Configuration` if any skip pattern is an invalid
/// regular expression.
pub fn translatable_chapters(
    book: &crate::epub::EpubBook,
    skip_patterns: &[String],
) -> Result<Vec<usize>, BabelEbookError> {
    let mut indices = Vec::new();
    for (index, chapter) in book.chapters.iter().enumerate() {
        if should_translate_doc(&chapter.href, skip_patterns)? {
            indices.push(index);
        }
    }
    Ok(indices)
}

/// Estimate the total number of source tokens across the given chapter indices.
#[must_use]
pub fn estimate_source_tokens(book: &crate::epub::EpubBook, indices: &[usize]) -> (usize, usize) {
    let total: usize = indices
        .iter()
        .map(|&index| {
            let text = String::from_utf8_lossy(&book.chapters[index].content);
            count_tokens(&text)
        })
        .sum();
    (total, indices.len())
}
