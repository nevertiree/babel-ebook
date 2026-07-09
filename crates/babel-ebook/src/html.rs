//! HTML document processing and bilingual translation insertion.

use std::collections::HashSet;

use crate::cache::TranslationCache;
use crate::chunking::{count_tokens, split_text_chunks};
use crate::config::{Config, OutputMode, TranslationScope};
use crate::core::BabelEbookError;
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
pub async fn translate_text(
    text: &str,
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    chapter_href: &str,
) -> Result<String, BabelEbookError> {
    // First pass. When refinement is disabled we can return a cached full-text
    // result immediately; otherwise the cached translation still needs to be
    // polished.
    let first_pass = if let Some(cached) = cache.get(&translator.name(), text) {
        if !config.refine {
            return Ok(cached);
        }
        cached
    } else {
        let max_source = config.max_source_tokens();
        let system_prompt = config.system_prompt_for_chapter(chapter_href);
        let target_lang = &config.target_lang;

        let chunks = split_text_chunks(text, max_source);
        let mut translated_parts = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            if let Some(cached) = cache.get(&translator.name(), &chunk) {
                translated_parts.push(cached);
                continue;
            }

            let context = TranslateContext {
                system_prompt: &system_prompt,
                target_lang,
            };
            let result = translator.translate(&chunk, &context).await?;
            let tokens = count_tokens(&chunk) + count_tokens(&result);
            cache.put(&translator.name(), &chunk, &result, Some(tokens));
            translated_parts.push(result);
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

    // Optional second-pass refinement using a separate cache namespace.
    let max_refine_source = config.max_refine_source_tokens();
    let refine_prompt = config.refine_prompt();
    let target_lang = &config.target_lang;
    let refine_name = format!("{}-refine", translator.name());

    let chunks = split_text_chunks(&first_pass, max_refine_source);
    let mut refined_parts = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        if let Some(cached) = cache.get(&refine_name, &chunk) {
            refined_parts.push(cached);
            continue;
        }

        let context = TranslateContext {
            system_prompt: &refine_prompt,
            target_lang,
        };
        let result = translator.translate(&chunk, &context).await?;
        let tokens = count_tokens(&chunk) + count_tokens(&result);
        cache.put(&refine_name, &chunk, &result, Some(tokens));
        refined_parts.push(result);
    }

    let refined = refined_parts
        .join(" ")
        .replace('\n', " ")
        .trim()
        .to_string();
    Ok(refined)
}

/// Translate all translatable elements in an EPUB HTML document and insert
/// translations according to `config.output_mode`.
///
/// The returned future is not `Send` because `kuchiki` uses `Rc` internally.
/// Callers that need a `Send` future should run the work on a local runtime.
#[allow(clippy::future_not_send)]
pub async fn process_document(
    html: &[u8],
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    chapter_href: &str,
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
        translate_element_text_and_attributes(&element, translator, config, cache, chapter_href)
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
async fn translate_element_text_and_attributes(
    element: &kuchiki::NodeDataRef<kuchiki::ElementData>,
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    chapter_href: &str,
) -> Result<(), BabelEbookError> {
    let node = element.as_node();
    let tag_name = element.name.local.as_ref();

    if should_translate_element_text(tag_name, &config.translation_scope) {
        let text = normalize_text(node);
        if is_translatable_text(&text) {
            let translated = translate_text(&text, translator, config, cache, chapter_href).await?;

            if !translated.is_empty() {
                let target_lang = &config.target_lang;
                if tag_name == "li" {
                    insert_li_translation(
                        node,
                        &translated,
                        target_lang,
                        config.output_mode,
                        config.preserve_classes,
                    );
                } else {
                    insert_generic_translation(
                        node,
                        &element.name,
                        &translated,
                        target_lang,
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
                let translated_attr =
                    match translate_text(&value, translator, config, cache, chapter_href).await {
                        Ok(t) => t,
                        Err(err) => {
                            tracing::error!(
                                "Failed to translate attribute {} on <{}>: {err}",
                                attr_name,
                                tag_name
                            );
                            continue;
                        }
                    };
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
            set_lang(node, "en");
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
            set_lang(&original_clone, "en");
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
            set_lang(node, "en");
        }
        OutputMode::TranslationOnly => {
            // Replace the original content with the translated text.
            for child in node.children().collect::<Vec<_>>() {
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
            set_lang(node, "en");
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
