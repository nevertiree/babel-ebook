//! Ordered, bounded-concurrency translation pipeline.

use std::sync::Arc;

use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::cache::TranslationCache;
use crate::config::Config;
use crate::core::{BabelEbookError, ProgressCallback, ProgressEvent};
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
pub async fn run_ordered_pipeline(
    book: &mut EpubBook,
    indices: Vec<usize>,
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    progress: Option<&dyn ProgressCallback>,
) -> Result<PipelineResult, BabelEbookError> {
    if indices.is_empty() {
        return Ok(PipelineResult {
            failures: Vec::new(),
            chapters: book.chapters.clone(),
        });
    }

    let semaphore = Arc::new(Semaphore::new(config.concurrency.max(1)));
    let mut futures = FuturesUnordered::new();

    for &index in &indices {
        let href = book.chapters[index].href.clone();
        let content = book.chapters[index].content.clone();
        let semaphore = Arc::clone(&semaphore);
        futures.push(async move {
            emit_progress(
                progress,
                ProgressEvent::ChapterStarted {
                    index,
                    href: href.clone(),
                },
            );
            match acquire_permit(semaphore).await {
                Ok(permit) => {
                    let result = process_document(&content, translator, config, cache, &href).await;
                    drop(permit);
                    (index, result)
                }
                Err(err) => (index, Err(err)),
            }
        });
    }

    let mut results = Vec::with_capacity(indices.len());
    while let Some(item) = futures.next().await {
        results.push(item);
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

    Ok(PipelineResult {
        failures,
        chapters: book.chapters.clone(),
    })
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
        }
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
