//! HTML document processing and bilingual translation insertion.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::cache::TranslationCache;
use crate::chunking::{count_tokens, split_text_chunks};
use crate::config::{Config, OutputMode, TranslationScope};
use crate::core::{BabelEbookError, CancellationToken};
use crate::translator::{TranslateContext, Translator};
use kuchiki::traits::TendrilSink;
use kuchiki::{Attribute, ExpandedName, NodeRef};
use markup5ever::{namespace_url, ns, QualName};

const SKIPPED_ANCESTORS: &[&str] = &["pre", "code", "script", "style"];

/// Return `true` if `text` is worth translating.
///
/// Mirrors the element-text threshold: trimmed text must contain at least 2
/// characters.
fn is_translatable_text(text: &str) -> bool {
    text.trim().chars().count() >= 2
}

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
    config: &Config,
    cache: &TranslationCache,
    chapter_index: usize,
    chapter_href: &str,
    progress: Option<&dyn crate::core::ProgressCallback>,
    cancellation: Option<&CancellationToken>,
) -> Result<String, BabelEbookError> {
    // First pass. When refinement is disabled we can return a cached full-text
    // result immediately; otherwise the cached translation still needs to be
    // polished.
    let first_pass = if let Some(cached) = cache.get_async(&translator.name(), text).await {
        if !config.refine {
            return Ok(cached);
        }
        cached
    } else {
        let max_source = config.max_source_tokens();
        let system_prompt = config.system_prompt_for_chapter(chapter_href);
        let target_lang = &config.target_lang;

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

    if !config.refine {
        return Ok(first_pass);
    }

    // Optional second-pass refinement using a separate cache namespace. Refine
    // progress is not reported as chunks because the number of refine chunks is
    // not known until the first pass completes; keeping chapter progress tied to
    // the first-pass source chunks gives a stable, monotonically increasing bar.
    let max_refine_source = config.max_refine_source_tokens();
    let refine_prompt = config.refine_prompt();
    let target_lang = &config.target_lang;
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

/// Progress adapter that turns per-text-block chunk events into chapter-global
/// chunk events.
///
/// `process_document` first counts how many first-pass source chunks the whole
/// chapter will produce. While translating, each element calls `translate_text`,
/// which still emits chunk events relative to that single text block. This
/// adapter intercepts those events and rewrites `chunk_index`/`chunk_total` so
/// the progress bar moves smoothly across elements instead of resetting for
/// every paragraph or heading.
struct ChapterChunkAdapter<'a> {
    inner: Option<&'a dyn crate::core::ProgressCallback>,
    chapter_index: usize,
    href: String,
    total_chunks: usize,
    next_chunk: AtomicUsize,
    cancellation: Option<&'a CancellationToken>,
}

impl crate::core::ProgressCallback for ChapterChunkAdapter<'_> {
    fn on_progress(&self, event: crate::core::ProgressEvent) {
        match event {
            crate::core::ProgressEvent::ChunkStarted { .. } => {
                let chunk_index = self.next_chunk.load(Ordering::SeqCst);
                if let Some(inner) = self.inner {
                    inner.on_progress(crate::core::ProgressEvent::ChunkStarted {
                        index: self.chapter_index,
                        href: self.href.clone(),
                        chunk_index,
                        chunk_total: self.total_chunks,
                    });
                }
            }
            crate::core::ProgressEvent::ChunkFinished { .. } => {
                let chunk_index = self.next_chunk.fetch_add(1, Ordering::SeqCst);
                if let Some(inner) = self.inner {
                    inner.on_progress(crate::core::ProgressEvent::ChunkFinished {
                        index: self.chapter_index,
                        href: self.href.clone(),
                        chunk_index,
                        chunk_total: self.total_chunks,
                    });
                }
            }
            other => {
                if let Some(inner) = self.inner {
                    inner.on_progress(other);
                }
            }
        }
    }
}

/// Count how many first-pass source chunks a chapter will produce.
///
/// This mirrors the translation decisions made by `process_document` so that the
/// chapter-level chunk total is known before translation starts.
fn count_translatable_chunks(
    elements: &[kuchiki::NodeDataRef<kuchiki::ElementData>],
    skip_set: &HashSet<*const kuchiki::Node>,
    translate_tags: &HashSet<&str>,
    config: &Config,
) -> usize {
    let max_source = config.max_source_tokens();
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
        if should_translate_element_text(tag_name, &config.translation_scope) {
            let text = normalize_text(node);
            if is_translatable_text(&text) {
                total += split_text_chunks(&text, max_source).len();
            }
        }

        for attr_name in &config.translate_attributes {
            if !should_translate_attribute(attr_name, &config.translation_scope) {
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

/// Translate all translatable elements in an EPUB HTML document and insert
/// translations according to `config.output_mode`.
///
/// The returned future is not `Send` because `kuchiki` uses `Rc` internally.
/// Callers that need a `Send` future should run the work on a local runtime.
#[allow(clippy::future_not_send)]
#[allow(clippy::too_many_arguments)]
pub async fn process_document(
    html: &[u8],
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    chapter_index: usize,
    chapter_href: &str,
    progress: Option<&dyn crate::core::ProgressCallback>,
    cancellation: Option<&CancellationToken>,
) -> Result<Vec<u8>, BabelEbookError> {
    let html_str = std::str::from_utf8(html)
        .map_err(|e| BabelEbookError::Configuration(format!("invalid UTF-8 HTML: {e}")))?;
    let doc = kuchiki::parse_html().one(html_str);

    if let Some(font) = &config.output_font {
        inject_font_style(&doc, font);
    }

    let translate_tags: HashSet<&str> = config.translate_tags.iter().map(String::as_str).collect();
    if translate_tags.is_empty() {
        return Ok(doc.to_string().into_bytes());
    }

    // Set of raw node pointers used only for identity comparison during this
    // single synchronous pass of `process_document`. Each pointer is derived from
    // a live `NodeRef` and remains valid as long as the document exists. The set
    // must not be stored across `await` points; `*const kuchiki::Node` is
    // `!Send`/`!Sync`, so it is only used in the synchronous setup below.
    let skip_set: HashSet<*const kuchiki::Node> = build_skip_set(&doc, &config.exclude_selectors);

    let selector = config.translate_tags.join(", ");
    let elements: Vec<kuchiki::NodeDataRef<kuchiki::ElementData>> = match doc.select(&selector) {
        Ok(iter) => iter.collect(),
        Err(()) => return Ok(doc.to_string().into_bytes()),
    };

    // Use a chapter-global chunk counter so that progress does not reset between
    // elements. The total is the number of first-pass source chunks across all
    // translatable text blocks in this chapter.
    let total_chunks = count_translatable_chunks(&elements, &skip_set, &translate_tags, config);
    let adapter = progress.map(|p| ChapterChunkAdapter {
        inner: Some(p),
        chapter_index,
        href: chapter_href.to_string(),
        total_chunks,
        next_chunk: AtomicUsize::new(0),
        cancellation,
    });
    let chapter_progress = adapter
        .as_ref()
        .map(|a| a as &dyn crate::core::ProgressCallback);
    // The adapter holds the token so that process_document can forward it
    // through the progress-callback path; fall back to the direct parameter
    // for callers that do not supply a progress callback.
    let chapter_cancellation = adapter
        .as_ref()
        .and_then(|a| a.cancellation)
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
            config,
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

/// Build a set of raw node pointers for elements matched by
/// `config.exclude_selectors`.
fn build_skip_set(doc: &NodeRef, exclude_selectors: &[String]) -> HashSet<*const kuchiki::Node> {
    if exclude_selectors.is_empty() {
        return HashSet::new();
    }
    exclude_selectors
        .iter()
        .flat_map(|sel| doc.select(sel).ok().into_iter().flatten())
        .map(|n| node_ptr(n.as_node()))
        .collect()
}

/// Return the raw pointer for a node, used only for identity comparison.
fn node_ptr(node: &NodeRef) -> *const kuchiki::Node {
    std::ptr::from_ref::<kuchiki::Node>(&**node)
}

/// Translate the text and configured attributes of a single element.
#[allow(clippy::future_not_send)]
#[allow(clippy::too_many_arguments)]
async fn translate_element_text_and_attributes(
    element: &kuchiki::NodeDataRef<kuchiki::ElementData>,
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    chapter_index: usize,
    chapter_href: &str,
    progress: Option<&dyn crate::core::ProgressCallback>,
    cancellation: Option<&CancellationToken>,
) -> Result<(), BabelEbookError> {
    let node = element.as_node();
    let tag_name = element.name.local.as_ref();

    if should_translate_element_text(tag_name, &config.translation_scope) {
        let text = normalize_text(node);
        if is_translatable_text(&text) {
            let translated = translate_text(
                &text,
                translator,
                config,
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
                let target_lang = &config.target_lang;
                let source_lang = if config.source_lang == "auto" {
                    "en"
                } else {
                    config.source_lang.as_str()
                };
                if tag_name == "li" {
                    insert_li_translation(
                        node,
                        &translated,
                        target_lang,
                        source_lang,
                        config.output_mode,
                        config.preserve_classes,
                    );
                } else {
                    insert_generic_translation(
                        node,
                        &element.name,
                        &translated,
                        target_lang,
                        source_lang,
                        config.output_mode,
                        config.preserve_classes,
                    );
                }
            }
        }
    }

    for attr in &config.translate_attributes {
        let attr_name = attr.as_str();
        if !should_translate_attribute(attr_name, &config.translation_scope) {
            continue;
        }
        let attr_value = element.attributes.borrow().get(attr_name).map(String::from);
        if let Some(value) = attr_value {
            if is_translatable_text(&value) {
                let translated_attr = translate_text(
                    &value,
                    translator,
                    config,
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

/// Return `true` if `name` is a body-content tag whose text is controlled by
/// `translation_scope.body`.
fn is_body_tag(name: &str) -> bool {
    matches!(
        name,
        "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "figcaption" | "dt" | "dd"
    )
}

/// Return `true` if `name` is a table cell tag whose text is controlled by
/// `translation_scope.tables`.
fn is_table_tag(name: &str) -> bool {
    matches!(name, "td" | "th")
}

/// Return `true` if the text content of an element named `name` should be
/// translated according to `scope`.
fn should_translate_element_text(name: &str, scope: &TranslationScope) -> bool {
    if is_body_tag(name) && !scope.body {
        return false;
    }
    if is_table_tag(name) && !scope.tables {
        return false;
    }
    true
}

/// Return `true` if an attribute named `name` should be translated according to
/// `scope`.
fn should_translate_attribute(name: &str, scope: &TranslationScope) -> bool {
    match name {
        "alt" => scope.alt_text,
        "title" => scope.metadata,
        _ => true,
    }
}

/// Return `true` if `node` has an ancestor that is in the excluded subtree set.
fn is_inside_excluded_subtree(node: &NodeRef, skip_set: &HashSet<*const kuchiki::Node>) -> bool {
    node.ancestors().any(|n| skip_set.contains(&node_ptr(&n)))
}

/// Return `true` if `node` has a `pre`, `code`, `script`, or `style` ancestor.
fn is_inside_skipped_parent(node: &NodeRef) -> bool {
    node.ancestors().any(|n| {
        n.as_element().is_some_and(|e| {
            let name = e.name.local.as_ref();
            SKIPPED_ANCESTORS.contains(&name)
        })
    })
}

/// Return `true` if `node` contains a descendant whose tag is in `translate_tags`.
///
/// Descendants that sit inside skipped parents (`pre`, `code`, `script`,
/// `style`) are ignored, so a translatable tag nested in a code block does not
/// prevent its ancestor from being translated.
fn has_translatable_child(node: &NodeRef, translate_tags: &HashSet<&str>) -> bool {
    node.descendants().any(|n| {
        n.as_element().is_some_and(|e| {
            translate_tags.contains(e.name.local.as_ref()) && !is_inside_skipped_parent(&n)
        })
    })
}

/// Extract and normalise the text content of `node`.
///
/// Approximates `BeautifulSoup`'s `get_text(" ", strip=True)`: text from
/// descendant text nodes is stripped and joined with a single space.
fn normalize_text(node: &NodeRef) -> String {
    let mut parts = Vec::new();
    for n in node.descendants() {
        if let Some(text_node) = n.as_text() {
            let text = text_node.borrow().trim().to_string();
            if !text.is_empty() {
                parts.push(text);
            }
        }
    }
    parts.join(" ").trim().to_string()
}

/// Set the `lang` attribute on `node` to `value`.
fn set_lang(node: &NodeRef, value: &str) {
    if let Some(data) = node.as_element() {
        data.attributes.borrow_mut().insert("lang", value.into());
    }
}

/// Insert a translation for a generic (non-`<li>`) element according to the
/// configured output mode.
fn insert_generic_translation(
    node: &NodeRef,
    name: &QualName,
    translated: &str,
    target_lang: &str,
    source_lang: &str,
    mode: OutputMode,
    preserve_classes: bool,
) {
    let translated_element =
        NodeRef::new_element(QualName::new(None, ns!(html), name.local.clone()), None);
    translated_element.append(NodeRef::new_text(translated));
    if preserve_classes {
        copy_element_attributes(node, &translated_element);
    }
    set_lang(&translated_element, target_lang);

    match mode {
        OutputMode::Bilingual => {
            // Translated element first, original element second.
            node.insert_before(translated_element);
            set_lang(node, source_lang);
        }
        OutputMode::TranslationOnly => {
            // Replace the original element with the translated element.
            node.insert_before(translated_element);
            node.detach();
        }
        OutputMode::Interleaved => {
            // Replace the original element with [original clone, translated].
            let original_clone = clone_subtree(node);
            if preserve_classes {
                copy_element_attributes(node, &original_clone);
            }
            set_lang(&original_clone, source_lang);
            // Insert original_clone first, then translated, then remove the original.
            node.insert_before(original_clone);
            node.insert_before(translated_element);
            node.detach();
        }
    }
}

/// Insert a translation for a `<li>` element while preserving the list item
/// parent structure.
#[allow(unused_variables)]
fn insert_li_translation(
    node: &NodeRef,
    translated: &str,
    target_lang: &str,
    source_lang: &str,
    mode: OutputMode,
    preserve_classes: bool,
) {
    match mode {
        OutputMode::Bilingual => {
            let translated_p =
                NodeRef::new_element(QualName::new(None, ns!(html), "p".into()), None);
            translated_p.append(NodeRef::new_text(translated));
            set_lang(&translated_p, target_lang);
            node.prepend(translated_p);

            if let Some(data) = node.as_element() {
                let mut attrs = data.attributes.borrow_mut();
                let existing = attrs.get("class").map(String::from).unwrap_or_default();
                let new_class = if existing.is_empty() {
                    "bilingual-li".to_string()
                } else {
                    format!("{existing} bilingual-li")
                };
                attrs.insert("class", new_class);
            }
            set_lang(node, source_lang);
        }
        OutputMode::TranslationOnly => {
            // Replace the original content with the translated text.
            while let Some(child) = node.first_child() {
                child.detach();
            }
            node.append(NodeRef::new_text(translated));
            set_lang(node, target_lang);
        }
        OutputMode::Interleaved => {
            // Original content first, then the translated paragraph.
            let translated_p =
                NodeRef::new_element(QualName::new(None, ns!(html), "p".into()), None);
            translated_p.append(NodeRef::new_text(translated));
            set_lang(&translated_p, target_lang);
            node.append(translated_p);
            set_lang(node, source_lang);
        }
    }
}

/// Copy element attributes from `source` to `target`.
///
/// Used when `config.preserve_classes` is enabled so that translated clones
/// keep the original styling hooks.
fn copy_element_attributes(source: &NodeRef, target: &NodeRef) {
    if let (Some(src), Some(tgt)) = (source.as_element(), target.as_element()) {
        let src_attrs = src.attributes.borrow();
        let mut tgt_attrs = tgt.attributes.borrow_mut();
        for (name, attr) in &src_attrs.map {
            tgt_attrs.map.insert(name.clone(), attr.clone());
        }
    }
}

/// Create a deep clone of `node` and its descendants.
fn clone_subtree(node: &NodeRef) -> NodeRef {
    match node.data() {
        kuchiki::NodeData::Element(data) => {
            let attributes: Vec<(ExpandedName, Attribute)> = data
                .attributes
                .borrow()
                .map
                .iter()
                .map(|(name, attr)| {
                    (
                        name.clone(),
                        Attribute {
                            prefix: attr.prefix.clone(),
                            value: attr.value.clone(),
                        },
                    )
                })
                .collect();
            let cloned = NodeRef::new_element(data.name.clone(), attributes);
            for child in node.children() {
                cloned.append(clone_subtree(&child));
            }
            cloned
        }
        kuchiki::NodeData::Text(text) => NodeRef::new_text(text.borrow().clone()),
        kuchiki::NodeData::Comment(text) => NodeRef::new_comment(text.borrow().clone()),
        kuchiki::NodeData::ProcessingInstruction(pi) => {
            let (target, data) = pi.borrow().clone();
            NodeRef::new_processing_instruction(target, data)
        }
        kuchiki::NodeData::Doctype(doctype) => NodeRef::new_doctype(
            doctype.name.clone(),
            doctype.public_id.clone(),
            doctype.system_id.clone(),
        ),
        kuchiki::NodeData::Document(_) => NodeRef::new_document(),
        kuchiki::NodeData::DocumentFragment => {
            let cloned = NodeRef::new(kuchiki::NodeData::DocumentFragment);
            for child in node.children() {
                cloned.append(clone_subtree(&child));
            }
            cloned
        }
    }
}

fn emit_chunk_progress(
    progress: Option<&dyn crate::core::ProgressCallback>,
    index: usize,
    href: &str,
    chunk_index: usize,
    chunk_total: usize,
    finished: bool,
) {
    let event = if finished {
        crate::core::ProgressEvent::ChunkFinished {
            index,
            href: href.to_string(),
            chunk_index,
            chunk_total,
        }
    } else {
        crate::core::ProgressEvent::ChunkStarted {
            index,
            href: href.to_string(),
            chunk_index,
            chunk_total,
        }
    };
    if let Some(p) = progress {
        p.on_progress(event);
    }
}
