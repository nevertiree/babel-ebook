//! Low-level text translation with caching and optional refinement.

use crate::cache::TranslationCache;
use crate::chunking::{count_tokens, split_text_chunks};
use crate::config::TranslationOptions;
use crate::core::{BabelEbookError, CancellationToken, ProgressCallback};
use crate::translator::{TranslateContext, Translator};

use super::progress::emit_chunk_progress;

/// Translate `text`, using caching and chunking as needed.
///
/// Mirrors the Python implementation: checks the cache, splits oversized text
/// into chunks, translates each chunk, caches results, and joins them with a
/// single space after normalising internal newlines.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
pub async fn translate_text(
    text: &str,
    translator: &dyn Translator,
    options: &TranslationOptions,
    cache: &TranslationCache,
    chapter_index: usize,
    chapter_href: &str,
    progress: Option<&dyn ProgressCallback>,
    cancellation: Option<&CancellationToken>,
) -> Result<String, BabelEbookError> {
    // First pass. When refinement is disabled we can return a cached full-text
    // result immediately; otherwise the cached translation still needs to be
    // polished.
    let first_pass = if let Some(cached) = cache.get_async(&translator.name(), text).await {
        if !options.refine {
            return Ok(cached);
        }
        cached
    } else {
        let max_source = options.max_source_tokens();
        let system_prompt = options.system_prompt_for_chapter(chapter_href);
        let target_lang = &options.target_lang;

        let chunks = split_text_chunks(text, max_source);
        let chunk_total = chunks.len();
        let mut translated_parts = Vec::with_capacity(chunk_total);
        for (chunk_index, chunk) in chunks.iter().enumerate() {
            if cancellation.is_some_and(CancellationToken::is_cancelled) {
                return Err(BabelEbookError::Cancelled);
            }
            emit_chunk_progress(
                progress,
                chapter_index,
                chapter_href,
                chunk_index,
                chunk_total,
                false,
            );
            if let Some(cached) = cache.get_async(&translator.name(), chunk).await {
                translated_parts.push(cached);
                emit_chunk_progress(
                    progress,
                    chapter_index,
                    chapter_href,
                    chunk_index,
                    chunk_total,
                    true,
                );
                continue;
            }

            let context = TranslateContext {
                system_prompt: &system_prompt,
                target_lang,
            };
            let result = translator.translate(chunk, &context).await?;
            let tokens = count_tokens(chunk) + count_tokens(&result);
            cache
                .put_async(&translator.name(), chunk, &result, Some(tokens))
                .await;
            translated_parts.push(result);
            emit_chunk_progress(
                progress,
                chapter_index,
                chapter_href,
                chunk_index,
                chunk_total,
                true,
            );
        }

        translated_parts
            .join(" ")
            .replace('\n', " ")
            .trim()
            .to_string()
    };

    if !options.refine {
        return Ok(first_pass);
    }

    // Optional second-pass refinement using a separate cache namespace. Refine
    // progress is not reported as chunks because the number of refine chunks is
    // not known until the first pass completes; keeping chapter progress tied to
    // the first-pass source chunks gives a stable, monotonically increasing bar.
    let max_refine_source = options.max_refine_source_tokens();
    let refine_prompt = options.refine_prompt();
    let target_lang = &options.target_lang;
    let refine_name = format!("{}-refine", translator.name());

    let chunks = split_text_chunks(&first_pass, max_refine_source);
    let mut refined_parts = Vec::with_capacity(chunks.len());
    for chunk in &chunks {
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            return Err(BabelEbookError::Cancelled);
        }
        if let Some(cached) = cache.get_async(&refine_name, chunk).await {
            refined_parts.push(cached);
            continue;
        }

        let context = TranslateContext {
            system_prompt: &refine_prompt,
            target_lang,
        };
        let result = translator.translate(chunk, &context).await?;
        let tokens = count_tokens(chunk) + count_tokens(&result);
        cache
            .put_async(&refine_name, chunk, &result, Some(tokens))
            .await;
        refined_parts.push(result);
    }

    let refined = refined_parts
        .join(" ")
        .replace('\n', " ")
        .trim()
        .to_string();
    Ok(refined)
}
