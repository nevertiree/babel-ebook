//! Element selection and text-extraction helpers for HTML document processing.

#![allow(clippy::must_use_candidate, clippy::implicit_hasher)]

use std::collections::HashSet;

use kuchiki::NodeRef;

use crate::config::TranslationScope;

/// Tag names whose text content is never translated (code blocks, scripts, etc.).
pub const SKIPPED_ANCESTORS: &[&str] = &["pre", "code", "script", "style"];

/// Return `true` if `text` is worth translating.
///
/// Mirrors the element-text threshold: trimmed text must contain at least 2
/// characters.
pub fn is_translatable_text(text: &str) -> bool {
    text.trim().chars().count() >= 2
}

/// Build a set of raw node pointers for elements matched by
/// `exclude_selectors`.
pub fn build_skip_set(
    doc: &NodeRef,
    exclude_selectors: &[String],
) -> HashSet<*const kuchiki::Node> {
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
pub fn node_ptr(node: &NodeRef) -> *const kuchiki::Node {
    std::ptr::from_ref::<kuchiki::Node>(&**node)
}

/// Return `true` if `node` has an ancestor that is in the excluded subtree set.
pub fn is_inside_excluded_subtree(
    node: &NodeRef,
    skip_set: &HashSet<*const kuchiki::Node>,
) -> bool {
    node.ancestors().any(|n| skip_set.contains(&node_ptr(&n)))
}

/// Return `true` if `node` has a `pre`, `code`, `script`, or `style` ancestor.
pub fn is_inside_skipped_parent(node: &NodeRef) -> bool {
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
pub fn has_translatable_child(node: &NodeRef, translate_tags: &HashSet<&str>) -> bool {
    node.descendants().any(|n| {
        n.as_element().is_some_and(|e| {
            translate_tags.contains(e.name.local.as_ref()) && !is_inside_skipped_parent(&n)
        })
    })
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
pub fn should_translate_element_text(name: &str, scope: &TranslationScope) -> bool {
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
///
/// Only well-known, safe attributes are translated. Unknown attributes are
/// ignored even if they appear in `translate_attributes`, preventing accidental
/// mutation of `data-*`, event handlers, or ARIA properties.
pub fn should_translate_attribute(name: &str, scope: &TranslationScope) -> bool {
    match name {
        "alt" => scope.alt_text,
        "title" => scope.metadata,
        _ => false,
    }
}

/// Extract and normalise the text content of `node`.
///
/// Approximates `BeautifulSoup`'s `get_text(" ", strip=True)`: text from
/// descendant text nodes is stripped and joined with a single space. `<br>`
/// elements are preserved as newlines so that poetry, addresses, and signatures
/// keep their line structure.
pub fn normalize_text(node: &NodeRef) -> String {
    let mut result = String::new();
    let mut needs_space = false;
    for n in node.descendants() {
        if n.as_element()
            .is_some_and(|e| e.name.local.as_ref() == "br")
        {
            result.push('\n');
            needs_space = false;
            continue;
        }
        if let Some(text_node) = n.as_text() {
            let text = text_node.borrow();
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            if needs_space && !result.ends_with('\n') {
                result.push(' ');
            }
            result.push_str(trimmed);
            needs_space = true;
        }
    }
    result.trim().to_string()
}
