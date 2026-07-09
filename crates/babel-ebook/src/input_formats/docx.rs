//! DOCX document reader.

use std::path::Path;

use docx_rs::read_docx as docx_rs_read;

use crate::core::BabelEbookError;
use crate::epub::{Chapter, EpubBook, EpubMetadata};
use crate::input_formats::html_or_xhtml;

/// Read a DOCX file and convert it to an internal EPUB representation.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed as a DOCX document.
pub fn read_docx(path: &Path) -> Result<EpubBook, BabelEbookError> {
    let bytes = std::fs::read(path)
        .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("read docx: {e}")))?;
    let doc = docx_rs_read(&bytes)
        .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("parse docx: {e}")))?;

    let mut paragraphs: Vec<String> = Vec::new();
    for child in doc.document.children {
        if let docx_rs::DocumentChild::Paragraph(p) = child {
            let text: String = p
                .children
                .iter()
                .filter_map(|c| {
                    if let docx_rs::ParagraphChild::Run(r) = c {
                        Some(
                            r.children
                                .iter()
                                .filter_map(|rc| {
                                    if let docx_rs::RunChild::Text(t) = rc {
                                        Some(t.text.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<String>(),
                        )
                    } else {
                        None
                    }
                })
                .collect();
            if !text.trim().is_empty() {
                paragraphs.push(format!("<p>{}</p>", html_escape(&text)));
            }
        }
    }

    let body = paragraphs.join("\n");
    let html = html_or_xhtml(&body, "Untitled");
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
            href: "chapter.xhtml".into(),
            title: None,
            content: html.into_bytes(),
        }],
        resources: vec![],
    })
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use docx_rs::{Docx, Paragraph, Run};

    use super::read_docx;

    #[test]
    fn docx_to_html_keeps_paragraphs() {
        let dir = tempfile::tempdir().unwrap();
        let path: PathBuf = dir.path().join("test.docx");
        let file = std::fs::File::create(&path).unwrap();
        Docx::new()
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Hello")))
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text("World")))
            .build()
            .pack(file)
            .unwrap();

        let book = read_docx(&path).unwrap();
        let html = String::from_utf8_lossy(&book.chapters[0].content);
        assert!(html.contains("Hello"));
        assert!(html.contains("World"));
    }
}
