//! Ordered, bounded-concurrency translation pipeline.

use std::collections::HashSet;
use std::sync::Arc;

use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::cache::TranslationCache;
use crate::checkpoint::{ChapterCheckpoint, ChapterStatus, Checkpoint, CheckpointStore};
use crate::config::Config;
use crate::core::{BabelEbookError, CancellationToken, ProgressCallback, ProgressEvent};
use crate::epub::{Chapter, EpubBook};
use crate::html::process_document;
use crate::translator::Translator;

/// Result of running the ordered pipeline.
pub struct PipelineResult {
    /// `(href, error)` pairs for chapters that failed to translate.
    pub failures: Vec<(String, BabelEbookError)>,
    /// Updated chapters in original spine order.
    pub chapters: Vec<Chapter>,
}

/// Translate chapters in index order while allowing `config.concurrency`
/// concurrent requests.
///
/// Results are applied to `book.chapters` in the original spine order so that
/// resume positions remain stable.
///
/// The returned future is not `Send` because `kuchiki` uses `Rc` internally.
/// Callers that need a `Send` future should run the work on a local runtime.
#[allow(clippy::future_not_send)]
#[allow(clippy::too_many_arguments)]
pub async fn run_ordered_pipeline(
    book: &mut EpubBook,
    indices: Vec<usize>,
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    checkpoint_store: Option<&CheckpointStore>,
    job_id: Option<&str>,
    source_hash: &str,
    progress: Option<&dyn ProgressCallback>,
    cancellation: Option<&CancellationToken>,
) -> Result<PipelineResult, BabelEbookError> {
    if indices.is_empty() {
        return Ok(PipelineResult {
            failures: Vec::new(),
            chapters: book.chapters.clone(),
        });
    }

    let job_id = resolve_job_id(checkpoint_store, job_id, &config.source);
    let mut checkpoint = build_checkpoint(book, &indices, checkpoint_store, &job_id, source_hash);
    checkpoint.source_path = config.source.to_string_lossy().into_owned();
    let completed = restore_completed_chapters(book, &indices, &checkpoint, progress);
    let pending_indices: Vec<usize> = indices
        .into_iter()
        .filter(|i| !completed.contains(i))
        .collect();

    let checkpoint = Arc::new(std::sync::Mutex::new(checkpoint));
    let semaphore = Arc::new(Semaphore::new(config.concurrency.max(1)));
    let mut futures = FuturesUnordered::new();

    for &index in &pending_indices {
        ensure_not_cancelled(cancellation)?;

        let href = book.chapters[index].href.clone();
        let content = book.chapters[index].content.clone();
        let semaphore = Arc::clone(&semaphore);
        let checkpoint = Arc::clone(&checkpoint);
        let store = checkpoint_store.cloned();
        let job_id = job_id.clone();
        futures.push(async move {
            emit_progress(
                progress,
                ProgressEvent::ChapterStarted {
                    index,
                    href: href.clone(),
                },
            );
            let result = match acquire_permit(semaphore).await {
                Ok(permit) => {
                    let output = if cancellation.is_some_and(CancellationToken::is_cancelled) {
                        Err(BabelEbookError::Cancelled)
                    } else {
                        process_document(&content, translator, config, cache, &href).await
                    };
                    drop(permit);
                    output
                }
                Err(err) => Err(err),
            };

            update_checkpoint_entry(&checkpoint, index, &result, store.as_ref(), &job_id)?;

            Ok::<(usize, Result<Vec<u8>, BabelEbookError>), BabelEbookError>((index, result))
        });
    }

    let mut results = Vec::with_capacity(pending_indices.len());
    while let Some(item) = futures.next().await {
        let (index, result) = item?;
        if matches!(result, Err(BabelEbookError::Cancelled)) {
            return Err(BabelEbookError::Cancelled);
        }
        results.push((index, result));
        ensure_not_cancelled(cancellation)?;
    }
    results.sort_by_key(|(index, _)| *index);

    let mut failures = Vec::new();
    for (index, result) in results {
        match result {
            Ok(content) => {
                book.chapters[index].content = content;
                emit_progress(
                    progress,
                    ProgressEvent::ChapterFinished {
                        index,
                        href: book.chapters[index].href.clone(),
                    },
                );
            }
            Err(err) => {
                let href = book.chapters[index].href.clone();
                emit_progress(
                    progress,
                    ProgressEvent::Failed {
                        index,
                        href: href.clone(),
                        error: err.to_string(),
                    },
                );
                failures.push((href, err));
            }
        }
    }

    if let Some(store) = checkpoint_store {
        let cp = checkpoint
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        store.save(&cp)?;
    }

    Ok(PipelineResult {
        failures,
        chapters: book.chapters.clone(),
    })
}

fn ensure_not_cancelled(cancellation: Option<&CancellationToken>) -> Result<(), BabelEbookError> {
    if cancellation.is_some_and(CancellationToken::is_cancelled) {
        return Err(BabelEbookError::Cancelled);
    }
    Ok(())
}

fn resolve_job_id(
    checkpoint_store: Option<&CheckpointStore>,
    job_id: Option<&str>,
    source: &std::path::Path,
) -> String {
    if checkpoint_store.is_some() {
        job_id.map_or_else(
            || CheckpointStore::generate_job_id(source),
            ToString::to_string,
        )
    } else {
        job_id.unwrap_or("").to_string()
    }
}

fn build_checkpoint(
    book: &EpubBook,
    indices: &[usize],
    checkpoint_store: Option<&CheckpointStore>,
    job_id: &str,
    source_hash: &str,
) -> Checkpoint {
    let loaded = checkpoint_store.and_then(|store| store.load(job_id));
    let mut checkpoint = if let Some(cp) = loaded {
        if !cp.source_hash.is_empty() && cp.source_hash != source_hash {
            tracing::warn!(
                job_id,
                stored_hash = %cp.source_hash,
                current_hash = %source_hash,
                "source hash mismatch; ignoring existing checkpoint"
            );
            Checkpoint {
                job_id: job_id.to_string(),
                source_hash: source_hash.to_string(),
                source_path: String::new(),
                chapters: Vec::new(),
            }
        } else {
            cp
        }
    } else {
        Checkpoint {
            job_id: job_id.to_string(),
            source_hash: source_hash.to_string(),
            source_path: String::new(),
            chapters: Vec::new(),
        }
    };

    let existing_indices: HashSet<usize> = checkpoint.chapters.iter().map(|c| c.index).collect();
    for &index in indices {
        if !existing_indices.contains(&index) {
            checkpoint.chapters.push(ChapterCheckpoint {
                index,
                href: book.chapters[index].href.clone(),
                status: ChapterStatus::Pending,
                content: None,
                error: None,
            });
        }
    }
    checkpoint.chapters.sort_by_key(|c| c.index);
    checkpoint
}

fn restore_completed_chapters(
    book: &mut EpubBook,
    indices: &[usize],
    checkpoint: &Checkpoint,
    progress: Option<&dyn ProgressCallback>,
) -> HashSet<usize> {
    let completed: HashSet<usize> = checkpoint
        .chapters
        .iter()
        .filter(|c| c.status == ChapterStatus::Completed)
        .map(|c| c.index)
        .collect();
    for &index in indices {
        if completed.contains(&index) {
            if let Some(content) = checkpoint
                .chapters
                .iter()
                .find(|c| c.index == index)
                .and_then(|c| c.content.clone())
            {
                book.chapters[index].content = content;
                emit_progress(
                    progress,
                    ProgressEvent::ChapterStarted {
                        index,
                        href: book.chapters[index].href.clone(),
                    },
                );
                emit_progress(
                    progress,
                    ProgressEvent::ChapterFinished {
                        index,
                        href: book.chapters[index].href.clone(),
                    },
                );
            }
        }
    }
    completed
}

fn update_checkpoint_entry(
    checkpoint: &Arc<std::sync::Mutex<Checkpoint>>,
    index: usize,
    result: &Result<Vec<u8>, BabelEbookError>,
    store: Option<&CheckpointStore>,
    job_id: &str,
) -> Result<(), BabelEbookError> {
    let cp_to_save = {
        let mut cp = checkpoint
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = cp.chapters.iter_mut().find(|c| c.index == index) {
            match result {
                Ok(content) => {
                    entry.status = ChapterStatus::Completed;
                    entry.content = Some(content.clone());
                    entry.error = None;
                }
                Err(err) => {
                    entry.status = ChapterStatus::Failed;
                    entry.error = Some(err.to_string());
                }
            }
        }
        cp.clone()
    };

    if let Some(store) = store {
        if let Err(err) = store.save(&cp_to_save) {
            tracing::warn!(job_id, error = %err, "failed to save checkpoint");
            return Err(err);
        }
    }
    Ok(())
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
    if let Some(callback) = progress {
        callback.on_progress(event);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::thread;

    use super::*;
    use crate::config::{Config, OutputMode, PromptTemplates, TranslationScope, TranslationStyle};
    use crate::epub::{Chapter, EpubBook, EpubMetadata};
    use crate::translator::{TranslateContext, Translator};
    use async_trait::async_trait;

    struct DummyTranslator;

    #[async_trait]
    impl Translator for DummyTranslator {
        fn name(&self) -> String {
            "dummy".into()
        }

        fn max_output_tokens(&self) -> usize {
            1000
        }

        async fn translate(
            &self,
            text: &str,
            _ctx: &TranslateContext<'_>,
        ) -> Result<String, BabelEbookError> {
            Ok(format!("[{}]", text.trim()))
        }
    }

    fn make_book(contents: Vec<&str>) -> EpubBook {
        EpubBook {
            metadata: EpubMetadata::default(),
            chapters: contents
                .into_iter()
                .enumerate()
                .map(|(i, c)| Chapter {
                    href: format!("ch{i:02}.xhtml"),
                    title: None,
                    content: format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?><html xmlns="http://www.w3.org/1999/xhtml"><body><p>{c}</p></body></html>"#
                    )
                    .into_bytes(),
                })
                .collect(),
            resources: vec![],
        }
    }

    fn make_config() -> Config {
        Config {
            source: PathBuf::default(),
            output: PathBuf::default(),
            provider: "dummy".into(),
            api_key: None,
            base_url: None,
            model: "dummy".into(),
            concurrency: 2,
            max_input_tokens: 4000,
            max_output_tokens: 2000,
            cache_dir: std::env::temp_dir().join(format!("test-cache-{}", std::process::id())),
            checkpoint_dir: std::env::temp_dir()
                .join(format!("test-checkpoint-{}", std::process::id())),
            resume_job_id: None,
            temperature: 0.3,
            source_lang: "en".into(),
            target_lang: "zh-CN".into(),
            skip_doc_patterns: vec![],
            translate_tags: vec!["p".into()],
            system_prompt: None,
            dry_run: false,
            verbose: false,
            provider_config: None,
            providers: HashMap::default(),
            output_mode: OutputMode::TranslationOnly,
            translation_scope: TranslationScope::default(),
            style: TranslationStyle::default(),
            chapter_prompts: HashMap::default(),
            prompts: PromptTemplates::default(),
            glossary: vec![],
            exclude_selectors: vec![],
            translate_attributes: vec![],
            preserve_classes: false,
            output_font: None,
            refine: false,
        }
    }

    #[tokio::test]
    async fn pipeline_skips_completed_chapters() {
        let dir = tempfile::tempdir().unwrap();
        let store = CheckpointStore::new(dir.path().to_path_buf()).unwrap();
        let mut book = make_book(vec!["alpha", "beta", "gamma"]);
        let mut config = make_config();
        config.checkpoint_dir = dir.path().join("checkpoints");
        let job_id = CheckpointStore::generate_job_id(&config.source);
        // Pre-populate checkpoint: chapter 0 completed.
        store
            .save(&Checkpoint {
                job_id: job_id.clone(),
                source_hash: "hash".into(),
                source_path: config.source.to_string_lossy().into_owned(),
                chapters: vec![
                    ChapterCheckpoint {
                        index: 0,
                        href: "ch00.xhtml".into(),
                        status: ChapterStatus::Completed,
                        content: Some(b"<p>DONE</p>".to_vec()),
                        error: None,
                    },
                    ChapterCheckpoint {
                        index: 1,
                        href: "ch01.xhtml".into(),
                        status: ChapterStatus::Pending,
                        content: None,
                        error: None,
                    },
                    ChapterCheckpoint {
                        index: 2,
                        href: "ch02.xhtml".into(),
                        status: ChapterStatus::Pending,
                        content: None,
                        error: None,
                    },
                ],
            })
            .unwrap();

        let cache = TranslationCache::new(config.cache_dir.clone());
        let result = run_ordered_pipeline(
            &mut book,
            vec![0, 1, 2],
            &DummyTranslator,
            &config,
            &cache,
            Some(&store),
            Some(&job_id),
            "hash",
            None,
            None,
        )
        .await
        .unwrap();
        assert!(result.failures.is_empty());
        assert!(String::from_utf8_lossy(&book.chapters[0].content).contains("DONE"));
        assert!(String::from_utf8_lossy(&book.chapters[1].content).contains("[beta]"));
        assert!(String::from_utf8_lossy(&book.chapters[2].content).contains("[gamma]"));
    }

    #[tokio::test]
    async fn pipeline_stops_before_scheduling_when_cancelled() {
        let mut book = make_book(vec!["alpha", "beta"]);
        let config = make_config();
        let cache = TranslationCache::new(config.cache_dir.clone());
        let cancellation = CancellationToken::default();
        cancellation.cancel();

        let result = run_ordered_pipeline(
            &mut book,
            vec![0, 1],
            &DummyTranslator,
            &config,
            &cache,
            None,
            None,
            "",
            None,
            Some(&cancellation),
        )
        .await;
        let Err(err) = result else {
            panic!("expected cancelled pipeline");
        };

        assert!(matches!(err, BabelEbookError::Cancelled));
    }

    #[test]
    fn pipeline_preserves_order() {
        let handle = thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build current-thread runtime");

            rt.block_on(async {
                let mut book = make_book(vec!["alpha", "bravo", "charlie"]);
                let config = make_config();
                let cache = TranslationCache::new(config.cache_dir.clone());
                let result = run_ordered_pipeline(
                    &mut book,
                    vec![0, 1, 2],
                    &DummyTranslator,
                    &config,
                    &cache,
                    None,
                    None,
                    "",
                    None,
                    None,
                )
                .await
                .unwrap();
                assert!(result.failures.is_empty());

                let texts: Vec<String> = book
                    .chapters
                    .iter()
                    .map(|c| String::from_utf8_lossy(&c.content).to_string())
                    .collect();
                assert!(texts[0].contains("[alpha]"));
                assert!(texts[1].contains("[bravo]"));
                assert!(texts[2].contains("[charlie]"));
            });
        });
        handle.join().expect("test thread panicked");
    }
}
