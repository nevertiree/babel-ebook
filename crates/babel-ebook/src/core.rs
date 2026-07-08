//! Core orchestration for the babel-ebook pipeline.

use std::sync::Arc;

use serde::Serialize;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::cache::TranslationCache;
use crate::chunking::count_tokens;
use crate::config::Config;
use crate::epub::{read_epub, should_translate_doc, write_epub};
use crate::html::{process_document, translate_text};
use crate::t;
use crate::translator::Translator;

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
    tracing::info!(
        source = %config.source.display(),
        output = %config.output.display(),
        "starting EPUB translation"
    );

    let owned_cache = cache.map_or_else(
        || TranslationCache::new(config.cache_dir.clone()),
        TranslationCache::clone,
    );
    let cache = &owned_cache;

    let mut book = read_epub(&config.source)?;
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

    let semaphore = Arc::new(Semaphore::new(config.concurrency.max(1)));
    let failures = run_all_chapters(
        &mut book,
        translatable_indices,
        translator,
        config,
        cache,
        progress,
        semaphore,
    )
    .await?;

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

fn run_dry_run(
    book: &crate::epub::EpubBook,
    indices: &[usize],
    progress: Option<&dyn ProgressCallback>,
) {
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

#[allow(clippy::future_not_send)]
async fn run_all_chapters(
    book: &mut crate::epub::EpubBook,
    indices: Vec<usize>,
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    progress: Option<&dyn ProgressCallback>,
    semaphore: Arc<Semaphore>,
) -> Result<Vec<(String, BabelEbookError)>, BabelEbookError> {
    let futures = indices.into_iter().map(|index| {
        let semaphore = Arc::clone(&semaphore);
        let content = &book.chapters[index].content;
        let href = book.chapters[index].href.clone();
        async move {
            tracing::info!(index, href, "starting chapter translation");
            emit_progress(
                progress,
                ProgressEvent::ChapterStarted {
                    index,
                    href: href.clone(),
                },
            );
            let _permit = acquire_permit(semaphore).await?;
            let result = process_document(content, translator, config, cache, &href).await;
            match &result {
                Ok(_) => {
                    tracing::info!(index, href, "chapter translation finished");
                    emit_progress(
                        progress,
                        ProgressEvent::ChapterFinished {
                            index,
                            href: href.clone(),
                        },
                    );
                }
                Err(err) => {
                    tracing::error!(index, href, error = %err, "chapter translation failed");
                    emit_progress(
                        progress,
                        ProgressEvent::Failed {
                            index,
                            href: href.clone(),
                            error: err.to_string(),
                        },
                    );
                }
            }
            Ok::<(usize, Result<Vec<u8>, BabelEbookError>), BabelEbookError>((index, result))
        }
    });

    let mut failures: Vec<(String, BabelEbookError)> = Vec::new();
    for result in futures_util::future::join_all(futures).await {
        let (index, processed) = result?;
        match processed {
            Ok(content) => {
                let href = book.chapters[index].href.clone();
                let msg = t!("log_document_translated", href = href.as_str());
                tracing::info!("{msg}");
                book.chapters[index].content = content;

                if config.translation_scope.toc {
                    if let Some(title) = book.chapters[index].title.clone() {
                        match translate_title(&title, translator, config, cache, &href).await {
                            Ok(translated) => {
                                book.chapters[index].title = Some(translated);
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
            Err(err) => {
                let href = book.chapters[index].href.clone();
                let msg = t!(
                    "log_document_failed",
                    href = href.as_str(),
                    error = err.to_string()
                );
                tracing::error!("{msg}");
                failures.push((href, err));
            }
        }
    }
    Ok(failures)
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

async fn acquire_permit(
    semaphore: Arc<Semaphore>,
) -> Result<OwnedSemaphorePermit, BabelEbookError> {
    semaphore
        .acquire_owned()
        .await
        .map_err(|err| BabelEbookError::Anyhow(anyhow::anyhow!("semaphore closed: {err}")))
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
