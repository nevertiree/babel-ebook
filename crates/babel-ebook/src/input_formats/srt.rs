//! SRT subtitle reader.

use std::path::Path;

use crate::core::BabelEbookError;
use crate::epub::{Chapter, EpubBook, EpubMetadata};
use crate::input_formats::html_or_xhtml;

/// A single SRT subtitle entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrtEntry {
    /// Subtitle index number.
    pub index: usize,
    /// Start timestamp as it appears in the file.
    pub start: String,
    /// End timestamp as it appears in the file.
    pub end: String,
    /// Subtitle text, preserving internal line breaks.
    pub text: String,
}

/// Read an SRT file and convert it into the internal `EpubBook` representation.
///
/// The subtitle entries are wrapped in a minimal XHTML chapter so the rest of
/// the pipeline can translate them like any other document.
pub fn read_srt(path: &Path) -> Result<EpubBook, BabelEbookError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("read srt: {e}")))?;
    let entries = parse_srt_entries(&text)?;
    let html = srt_to_html(&entries);
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

/// Parse the raw text of an SRT file into a list of entries.
///
/// Blocks are separated by blank lines. Each block must contain an index, a
/// time line (`start --> end`), and at least one line of subtitle text. Blocks
/// with fewer than three lines are skipped.
pub fn parse_srt_entries(text: &str) -> Result<Vec<SrtEntry>, BabelEbookError> {
    let mut entries = Vec::new();
    // Normalize CRLF to LF first, then collapse any stray carriage returns
    // into line breaks so Windows SRT files split into blocks correctly.
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let blocks: Vec<&str> = normalized.trim().split("\n\n").collect();
    for block in blocks {
        let lines: Vec<&str> = block.lines().collect();
        if lines.len() < 3 {
            continue;
        }
        let index: usize = lines[0]
            .parse()
            .map_err(|_| BabelEbookError::Configuration("invalid SRT index".into()))?;
        let (start, end) = parse_time_line(lines[1])?;
        let text = lines[2..].join("\n");
        entries.push(SrtEntry {
            index,
            start,
            end,
            text,
        });
    }
    Ok(entries)
}

fn parse_time_line(line: &str) -> Result<(String, String), BabelEbookError> {
    let parts: Vec<&str> = line.split(" --> ").collect();
    if parts.len() != 2 {
        return Err(BabelEbookError::Configuration(format!(
            "invalid SRT time line: {line}"
        )));
    }
    Ok((parts[0].trim().into(), parts[1].trim().into()))
}

fn srt_to_html(entries: &[SrtEntry]) -> String {
    let body = entries
        .iter()
        .map(|e| {
            format!(
                r#"<div class="srt-entry" data-start="{}" data-end="{}"><p>{}</p></div>"#,
                html_escape(&e.start),
                html_escape(&e.end),
                html_escape(&e.text)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    html_or_xhtml(&body, "Untitled")
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_srt() {
        let srt = r"1
00:00:01,000 --> 00:00:04,000
Hello world

2
00:00:05,000 --> 00:00:07,000
Second line
";
        let entries = parse_srt_entries(srt).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "Hello world");
        assert_eq!(entries[1].start, "00:00:05,000");
    }

    #[test]
    fn parse_crlf_srt() {
        let srt = "1\r\n00:00:01,000 --> 00:00:04,000\r\nHello world\r\n\r\n2\r\n00:00:05,000 --> 00:00:07,000\r\nSecond line\r\n";
        let entries = parse_srt_entries(srt).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].index, 1);
        assert_eq!(entries[0].text, "Hello world");
        assert_eq!(entries[1].index, 2);
        assert_eq!(entries[1].start, "00:00:05,000");
    }

    #[test]
    fn parse_multiline_srt() {
        let srt = r"1
00:00:01,000 --> 00:00:04,000
Line one
Line two
";
        let entries = parse_srt_entries(srt).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "Line one\nLine two");
    }

    #[test]
    fn parse_srt_skips_malformed_blocks() {
        let srt = r"1
00:00:01,000 --> 00:00:04,000
Hello world

bad block

2
00:00:05,000 --> 00:00:07,000
Second line
";
        let entries = parse_srt_entries(srt).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].index, 2);
    }

    #[test]
    fn parse_srt_rejects_invalid_time_line() {
        let srt = r"1
00:00:01,000 00:00:04,000
Hello world
";
        let result = parse_srt_entries(srt);
        assert!(matches!(result, Err(BabelEbookError::Configuration(_))));
    }

    #[test]
    fn read_srt_produces_xhtml_chapter() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let path = temp_dir.path().join("sample.srt");
        std::fs::write(&path, "1\n00:00:01,000 --> 00:00:04,000\nHello world\n")
            .expect("write srt");

        let book = read_srt(&path).expect("read srt");
        assert_eq!(book.chapters.len(), 1);
        let content = String::from_utf8_lossy(&book.chapters[0].content);
        assert!(content.contains(r#"data-start="00:00:01,000""#));
        assert!(content.contains("<p>Hello world</p>"));
        assert!(content.contains("<html"));
    }

    #[test]
    fn srt_html_escapes_text() {
        let srt = r#"1
00:00:01,000 --> 00:00:04,000
A < B & C "D"
"#;
        let entries = parse_srt_entries(srt).unwrap();
        let html = srt_to_html(&entries);
        assert!(html.contains("&lt;"));
        assert!(html.contains("&amp;"));
        assert!(html.contains("&quot;"));
        assert!(!html.contains("A < B"));
    }
}
