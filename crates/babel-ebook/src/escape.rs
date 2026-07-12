//! Shared text escaping utilities.

/// Escape text for XML attribute or element content.
///
/// Escapes `&`, `<`, `>`, `"`, and `'` to their predefined XML entities. This
/// is sufficient for both XML serialization in EPUB package documents and for
/// HTML content generated from plain text input formats.
#[must_use]
pub fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Escape text for HTML element or attribute content.
///
/// This is currently an alias for [`xml_escape`] because the five characters
/// that must be escaped are the same for both HTML and XML text.
#[must_use]
pub fn html_escape(text: &str) -> String {
    xml_escape(text)
}
