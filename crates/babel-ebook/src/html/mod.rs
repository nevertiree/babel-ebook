//! HTML document processing and bilingual translation insertion.

use std::collections::HashSet;

use kuchiki::traits::TendrilSink;
use kuchiki::{Attribute, ExpandedName, NodeRef};
use markup5ever::{namespace_url, ns, QualName};

use crate::cache::TranslationCache;
use crate::chunking::split_text_chunks;
use crate::config::TranslationOptions;
use crate::core::{BabelEbookError, CancellationToken, ProgressCallback};
use crate::translator::Translator;

use insertion::{insert_generic_translation, insert_li_translation};
use progress::ChapterChunkAdapter;
use selection::{
    build_skip_set, has_translatable_child, is_inside_excluded_subtree, is_inside_skipped_parent,
    is_translatable_text, node_ptr, normalize_text, should_translate_attribute,
    should_translate_element_text,
};

mod insertion;
mod progress;
mod selection;
mod translation;

pub use translation::translate_text;

/// Translate all translatable elements in an EPUB HTML document and insert
/// translations according to `options.output_mode`.
///
/// The returned future is not `Send` because `kuchiki` uses `Rc` internally.
/// Callers that need a `Send` future should run the work on a local runtime.
#[allow(clippy::future_not_send)]
#[allow(clippy::too_many_arguments)]
pub async fn process_document(
    html: &[u8],
    translator: &dyn Translator,
    options: &TranslationOptions,
    cache: &TranslationCache,
    chapter_index: usize,
    chapter_href: &str,
    progress: Option<&dyn ProgressCallback>,
    cancellation: Option<&CancellationToken>,
) -> Result<Vec<u8>, BabelEbookError> {
    let html_str = std::str::from_utf8(html)
        .map_err(|e| BabelEbookError::Configuration(format!("invalid UTF-8 HTML: {e}")))?;
    let doc = kuchiki::parse_html().one(html_str);

    if let Some(font) = &options.output_font {
        inject_font_style(&doc, font);
    }

    let translate_tags: HashSet<&str> = options.translate_tags.iter().map(String::as_str).collect();
    if translate_tags.is_empty() {
        return Ok(doc.to_string().into_bytes());
    }

    // Set of raw node pointers used only for identity comparison during this
    // single synchronous pass of `process_document`. Each pointer is derived from
    // a live `NodeRef` and remains valid as long as the document exists. The set
    // must not be stored across `await` points; `*const kuchiki::Node` is
    // `!Send`/`!Sync`, so it is only used in the synchronous setup below.
    let skip_set: HashSet<*const kuchiki::Node> = build_skip_set(&doc, &options.exclude_selectors);

    let selector = options.translate_tags.join(", ");
    let elements: Vec<kuchiki::NodeDataRef<kuchiki::ElementData>> = match doc.select(&selector) {
        Ok(iter) => iter.collect(),
        Err(()) => return Ok(doc.to_string().into_bytes()),
    };

    // Use a chapter-global chunk counter so that progress does not reset between
    // elements. The total is the number of first-pass source chunks across all
    // translatable text blocks in this chapter.
    let total_chunks = count_translatable_chunks(&elements, &skip_set, &translate_tags, options);
    let adapter = progress.map(|p| {
        ChapterChunkAdapter::new(
            Some(p),
            chapter_index,
            chapter_href.to_string(),
            total_chunks,
            cancellation,
        )
    });
    let chapter_progress = adapter
        .as_ref()
        .map(|a| a as &dyn crate::core::ProgressCallback);
    // The adapter holds the token so that process_document can forward it
    // through the progress-callback path; fall back to the direct parameter
    // for callers that do not supply a progress callback.
    let chapter_cancellation = adapter
        .as_ref()
        .and_then(ChapterChunkAdapter::cancellation)
        .or(cancellation);

    for element in elements {
        let node = element.as_node();
        if skip_set.contains(&node_ptr(node)) || is_inside_excluded_subtree(node, &skip_set) {
            continue;
        }
        if is_inside_skipped_parent(node) {
            continue;
        }
        if has_translatable_child(node, &translate_tags) {
            continue;
        }
        translate_element_text_and_attributes(
            &element,
            translator,
            options,
            cache,
            chapter_index,
            chapter_href,
            chapter_progress,
            chapter_cancellation,
        )
        .await?;
    }

    Ok(doc.to_string().into_bytes())
}

/// Count how many first-pass source chunks a chapter will produce.
///
/// This mirrors the translation decisions made by `process_document` so that the
/// chapter-level chunk total is known before translation starts.
fn count_translatable_chunks(
    elements: &[kuchiki::NodeDataRef<kuchiki::ElementData>],
    skip_set: &HashSet<*const kuchiki::Node>,
    translate_tags: &HashSet<&str>,
    options: &TranslationOptions,
) -> usize {
    let max_source = options.max_source_tokens();
    let mut total = 0usize;
    for element in elements {
        let node = element.as_node();
        if skip_set.contains(&node_ptr(node)) || is_inside_excluded_subtree(node, skip_set) {
            continue;
        }
        if is_inside_skipped_parent(node) {
            continue;
        }
        if has_translatable_child(node, translate_tags) {
            continue;
        }

        let tag_name = element.name.local.as_ref();
        if should_translate_element_text(tag_name, &options.translation_scope) {
            let text = normalize_text(node);
            if is_translatable_text(&text) {
                total += split_text_chunks(&text, max_source).len();
            }
        }

        for attr_name in &options.translate_attributes {
            if !should_translate_attribute(attr_name, &options.translation_scope) {
                continue;
            }
            if let Some(value) = element.attributes.borrow().get(attr_name.as_str()) {
                if is_translatable_text(value) {
                    total += split_text_chunks(value, max_source).len();
                }
            }
        }
    }
    total
}

/// Translate the text and configured attributes of a single element.
#[allow(clippy::future_not_send)]
#[allow(clippy::too_many_arguments)]
async fn translate_element_text_and_attributes(
    element: &kuchiki::NodeDataRef<kuchiki::ElementData>,
    translator: &dyn Translator,
    options: &TranslationOptions,
    cache: &TranslationCache,
    chapter_index: usize,
    chapter_href: &str,
    progress: Option<&dyn ProgressCallback>,
    cancellation: Option<&CancellationToken>,
) -> Result<(), BabelEbookError> {
    let node = element.as_node();
    let tag_name = element.name.local.as_ref();

    if should_translate_element_text(tag_name, &options.translation_scope) {
        let text = normalize_text(node);
        if is_translatable_text(&text) {
            let translated = translate_text(
                &text,
                translator,
                options,
                cache,
                chapter_index,
                chapter_href,
                progress,
                cancellation,
            )
            .await
            .map_err(|err| {
                tracing::error!("Failed to translate element <{}>: {err}", tag_name);
                err
            })?;

            if !translated.is_empty() {
                let target_lang = &options.target_lang;
                let source_lang = if options.source_lang == "auto" {
                    "en"
                } else {
                    options.source_lang.as_str()
                };
                if tag_name == "li" {
                    insert_li_translation(
                        node,
                        &translated,
                        target_lang,
                        source_lang,
                        options.output_mode,
                        options.preserve_classes,
                    );
                } else {
                    insert_generic_translation(
                        node,
                        &element.name,
                        &translated,
                        target_lang,
                        source_lang,
                        options.output_mode,
                        options.preserve_classes,
                    );
                }
            }
        }
    }

    for attr in &options.translate_attributes {
        let attr_name = attr.as_str();
        if !should_translate_attribute(attr_name, &options.translation_scope) {
            continue;
        }
        let attr_value = element.attributes.borrow().get(attr_name).map(String::from);
        if let Some(value) = attr_value {
            if is_translatable_text(&value) {
                let translated_attr = translate_text(
                    &value,
                    translator,
                    options,
                    cache,
                    chapter_index,
                    chapter_href,
                    progress,
                    cancellation,
                )
                .await
                .map_err(|err| {
                    tracing::error!(
                        "Failed to translate attribute {} on <{}>: {err}",
                        attr_name,
                        tag_name
                    );
                    err
                })?;
                element
                    .attributes
                    .borrow_mut()
                    .insert(attr_name, translated_attr);
            }
        }
    }

    Ok(())
}

/// Inject a `<style>` element setting `font-family` on `<body>` into the document
/// `<head>`. If no `<head>` exists, the style is appended to the root element.
fn inject_font_style(doc: &NodeRef, font: &str) {
    let css = format!("body {{ font-family: {font}; }}\n");
    let style = NodeRef::new_element(
        QualName::new(None, ns!(html), "style".into()),
        Vec::<(ExpandedName, Attribute)>::new(),
    );
    style.append(NodeRef::new_text(css));

    if let Ok(head) = doc.select_first("head") {
        head.as_node().append(style);
        return;
    }

    if let Ok(html) = doc.select_first("html") {
        html.as_node().prepend(style);
    }
}
