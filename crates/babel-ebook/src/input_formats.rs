//! Built-in readers for non-EPUB ebook formats.

use std::path::Path;

use crate::core::BabelEbookError;
use crate::epub::{Chapter, EpubBook, EpubMetadata};
use crate::escape::html_escape;

pub mod docx;
pub mod srt;

/// Supported input ebook formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
pub enum InputFormat {
    /// EPUB 2/3.
    Epub,
    /// MOBI / AZW / AZW3 (Palm-based Kindle formats).
    Mobi,
    /// Plain text.
    Text,
    /// SubRip subtitle format.
    Srt,
    /// Microsoft Word Open XML document.
    Docx,
}

impl InputFormat {
    /// Detect the format from a file extension.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .and_then(|ext| match ext.as_str() {
                "epub" => Some(Self::Epub),
                "mobi" | "azw" | "azw3" | "prc" => Some(Self::Mobi),
                "txt" | "text" => Some(Self::Text),
                "srt" => Some(Self::Srt),
                "docx" => Some(Self::Docx),
                _ => None,
            })
    }

    /// Human-readable name for error messages.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Epub => "EPUB",
            Self::Mobi => "MOBI/AZW",
            Self::Text => "Plain text",
            Self::Srt => "SRT subtitles",
            Self::Docx => "DOCX",
        }
    }
}

/// Read an ebook from disk, dispatching to the appropriate built-in reader.
///
/// # Errors
///
/// Returns an error if the file extension is unsupported or if the underlying
/// reader fails to parse the file.
pub fn read_input_book(path: &Path) -> Result<EpubBook, BabelEbookError> {
    let format = InputFormat::from_path(path).ok_or_else(|| {
        BabelEbookError::Configuration(format!(
            "unsupported input format: {}. Supported formats: epub, mobi, azw, azw3, prc, txt, srt, docx",
            path.display()
        ))
    })?;

    match format {
        InputFormat::Epub => crate::epub::read_epub(path),
        InputFormat::Mobi => read_mobi(path),
        InputFormat::Text => read_text(path),
        InputFormat::Srt => srt::read_srt(path),
        InputFormat::Docx => docx::read_docx(path),
    }
}

fn read_mobi(path: &Path) -> Result<EpubBook, BabelEbookError> {
    let mobi = mobi::Mobi::from_path(path)
        .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("failed to parse MOBI file: {e}")))?;

    let title = Some(mobi.title());
    let language = mobi_language_to_code(mobi.language());

    let text = mobi.content_as_string().map_err(|e| {
        BabelEbookError::Anyhow(anyhow::anyhow!("failed to read MOBI content: {e}"))
    })?;
    let html = html_or_xhtml(&text, title.as_deref().unwrap_or("Untitled"));

    Ok(EpubBook {
        metadata: EpubMetadata {
            title,
            language,
            identifier: None,
        },
        chapters: vec![Chapter {
            href: "chapter.xhtml".to_string(),
            title: None,
            content: html.into_bytes(),
        }],
        resources: vec![],
    })
}

/// Convert a MOBI language enum variant into a BCP-47 style code.
fn mobi_language_to_code(lang: mobi::headers::Language) -> Option<String> {
    use mobi::headers::Language;
    let code = match lang {
        Language::Neutral | Language::Unknown => return None,
        Language::English => "en".to_string(),
        Language::Chinese => "zh".to_string(),
        Language::Japanese => "ja".to_string(),
        Language::Korean => "ko".to_string(),
        Language::Spanish => "es".to_string(),
        Language::French => "fr".to_string(),
        Language::German => "de".to_string(),
        Language::Russian => "ru".to_string(),
        Language::Portuguese => "pt".to_string(),
        Language::Italian => "it".to_string(),
        Language::Dutch => "nl".to_string(),
        Language::Polish => "pl".to_string(),
        Language::Turkish => "tr".to_string(),
        Language::Arabic => "ar".to_string(),
        Language::Hindi => "hi".to_string(),
        Language::Greek => "el".to_string(),
        Language::Czech => "cs".to_string(),
        Language::Danish => "da".to_string(),
        Language::Finnish => "fi".to_string(),
        Language::Hungarian => "hu".to_string(),
        Language::Norwegian => "no".to_string(),
        Language::Romanian => "ro".to_string(),
        Language::Slovak => "sk".to_string(),
        Language::Swedish => "sv".to_string(),
        Language::Thai => "th".to_string(),
        Language::Ukrainian => "uk".to_string(),
        Language::Vietnamese => "vi".to_string(),
        Language::Hebrew => "he".to_string(),
        Language::Indonesian => "id".to_string(),
        Language::Malay => "ms".to_string(),
        _ => format!("{lang:?}").to_lowercase(),
    };
    Some(code)
}

/// Wrap raw text or existing HTML in a minimal valid XHTML document.
#[must_use]
pub fn html_or_xhtml(text: &str, title: &str) -> String {
    let trimmed = text.trim_start();
    if trimmed.starts_with('<') && trimmed.to_lowercase().starts_with("<html") {
        // Already looks like a complete HTML document; reuse it as chapter content.
        if trimmed.starts_with("<?xml") {
            return text.to_string();
        }
        return format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{text}");
    }

    if trimmed.starts_with('<') {
        // HTML fragment without <html>: wrap it in a body.
        return format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
  <title>{title}</title>
  <meta charset="UTF-8"/>
</head>
<body>
{text}
</body>
</html>"#,
            title = html_escape(title),
            text = text
        );
    }

    wrap_text_as_xhtml(text, title)
}

fn read_text(path: &Path) -> Result<EpubBook, BabelEbookError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("failed to read text file: {e}")))?;

    let html = wrap_text_as_xhtml(&text, "Untitled");

    Ok(EpubBook {
        metadata: EpubMetadata {
            title: Some(
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string(),
            ),
            language: None,
            identifier: None,
        },
        chapters: vec![Chapter {
            href: "chapter.xhtml".to_string(),
            title: None,
            content: html.into_bytes(),
        }],
        resources: vec![],
    })
}

fn wrap_text_as_xhtml(text: &str, title: &str) -> String {
    let escaped = html_escape(text);
    let paragraphs: Vec<String> = escaped
        .lines()
        .map(|line| format!("<p>{line}</p>"))
        .collect();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
  <title>{title}</title>
  <meta charset="UTF-8"/>
</head>
<body>
{body}
</body>
</html>"#,
        body = paragraphs.join("\n")
    )
}

/// Return a list of supported input file extensions for UI filters.
#[must_use]
pub const fn supported_extensions() -> &'static [&'static str] {
    &["epub", "mobi", "azw", "azw3", "prc", "txt", "srt", "docx"]
}
