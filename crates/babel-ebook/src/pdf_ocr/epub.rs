//! Assemble OCR page results into a valid `EpubBook`.

use std::fmt::Write;

use crate::epub::{Chapter, EpubBook, EpubMetadata};
use crate::input_formats::html_or_xhtml;
use crate::pdf_ocr::backend::{BlockType, OcrPageResult, TextBlock};

/// Convert page-level OCR results into an `EpubBook`.
///
/// Headings detected by the OCR backend are used as chapter boundaries: each
/// time a major heading is encountered, the current chapter is closed and a new
/// one begins. This preserves the document structure in the resulting EPUB.
#[must_use]
pub fn build_epub(title: &str, pages: &[OcrPageResult]) -> EpubBook {
    let mut chapters: Vec<Chapter> = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_body: Vec<String> = Vec::new();

    for page in pages {
        let mut pending_cells: Vec<&TextBlock> = Vec::new();

        for block in &page.blocks {
            if block.block_type == BlockType::TableCell {
                pending_cells.push(block);
                continue;
            }

            if !pending_cells.is_empty() {
                current_body.push(render_table(&pending_cells));
                pending_cells.clear();
            }

            if block.block_type == BlockType::Heading {
                // Close the previous chapter.
                if !current_body.is_empty() {
                    chapters.push(finish_chapter(
                        current_title.as_deref(),
                        &current_body,
                        chapters.len(),
                    ));
                    current_body.clear();
                }
                current_title = Some(block.text.clone());
                continue;
            }

            current_body.push(block_to_html(block));
        }

        if !pending_cells.is_empty() {
            current_body.push(render_table(&pending_cells));
        }
    }

    // Close the final chapter.
    if !current_body.is_empty() || current_title.is_some() {
        chapters.push(finish_chapter(
            current_title.as_deref(),
            &current_body,
            chapters.len(),
        ));
    }

    // If no chapters were created, create a single chapter with all text so the
    // output is still valid.
    if chapters.is_empty() {
        let all_text: Vec<String> = pages
            .iter()
            .flat_map(|p| p.blocks.iter().map(block_to_html))
            .collect();
        chapters.push(finish_chapter(Some(title), &all_text, 0));
    }

    // Ensure every chapter has a title for the table of contents.
    for (i, chapter) in chapters.iter_mut().enumerate() {
        if chapter.title.is_none() {
            chapter.title = Some(format!("Chapter {}", i + 1));
        }
    }

    EpubBook {
        metadata: EpubMetadata {
            title: Some(title.to_string()),
            language: None,
            identifier: None,
        },
        chapters,
        resources: vec![],
    }
}

/// Render a sequence of table cells as an HTML table, inferring rows from the
/// vertical positions of the cells.
fn render_table(cells: &[&TextBlock]) -> String {
    if cells.is_empty() {
        return String::new();
    }

    // Group cells into rows by similar y coordinate. A simple heuristic: cells
    // whose y values differ by no more than half the median cell height belong
    // to the same row.
    let mut sorted: Vec<&&TextBlock> = cells.iter().collect();
    sorted.sort_by_key(|b| b.bbox.map_or(0, |bbox| bbox.y));

    let row_threshold = cells
        .iter()
        .filter_map(|b| b.bbox.map(|bbox| bbox.h / 2))
        .min()
        .unwrap_or(10)
        .max(5);

    let mut rows: Vec<Vec<&&TextBlock>> = Vec::new();
    for cell in sorted {
        let y = cell.bbox.map_or(0, |bbox| bbox.y);
        if let Some(row) = rows.iter_mut().find(|r| {
            let row_y = r[0].bbox.map_or(0, |bbox| bbox.y);
            y.abs_diff(row_y) <= row_threshold
        }) {
            row.push(cell);
        } else {
            rows.push(vec![cell]);
        }
    }

    // Within each row, sort cells left-to-right.
    for row in &mut rows {
        row.sort_by_key(|b| b.bbox.map_or(0, |bbox| bbox.x));
    }

    let mut html = String::from("<table>\n");
    for row in rows {
        html.push_str("  <tr>");
        for cell in row {
            let escaped = html_escape(&cell.text);
            let _ = write!(html, "<td>{escaped}</td>");
        }
        html.push_str("</tr>\n");
    }
    html.push_str("</table>");
    html
}

fn block_to_html(block: &TextBlock) -> String {
    let escaped = html_escape(&block.text);
    match block.block_type {
        BlockType::Heading => format!("<h1>{escaped}</h1>"),
        BlockType::Subheading => format!("<h2>{escaped}</h2>"),
        BlockType::Caption => format!("<figcaption>{escaped}</figcaption>"),
        BlockType::TableCell => format!("<td>{escaped}</td>"),
        BlockType::Paragraph | BlockType::Other => {
            // Preserve line breaks as separate paragraphs.
            escaped
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| format!("<p>{l}</p>"))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

fn finish_chapter(title: Option<&str>, body: &[String], index: usize) -> Chapter {
    let chapter_title = title.unwrap_or("Chapter");
    let body_html = body.join("\n");
    let html = html_or_xhtml(&body_html, chapter_title);
    Chapter {
        href: format!("chapter{:03}.xhtml", index + 1),
        title: Some(chapter_title.to_string()),
        content: html.into_bytes(),
    }
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
    use super::*;
    use crate::pdf_ocr::backend::{BlockType, TextBlock};

    #[test]
    fn headings_split_chapters() {
        let pages = vec![OcrPageResult {
            page_number: 1,
            blocks: vec![
                TextBlock {
                    text: "Introduction".into(),
                    block_type: BlockType::Heading,
                    ..Default::default()
                },
                TextBlock {
                    text: "First paragraph.".into(),
                    block_type: BlockType::Paragraph,
                    ..Default::default()
                },
                TextBlock {
                    text: "Chapter One".into(),
                    block_type: BlockType::Heading,
                    ..Default::default()
                },
                TextBlock {
                    text: "Second paragraph.".into(),
                    block_type: BlockType::Paragraph,
                    ..Default::default()
                },
            ],
            full_text: String::new(),
        }];

        let book = build_epub("Test Book", &pages);
        assert_eq!(book.chapters.len(), 2);
        let content0 = String::from_utf8_lossy(&book.chapters[0].content);
        assert!(content0.contains("First paragraph."));
        let content1 = String::from_utf8_lossy(&book.chapters[1].content);
        assert!(content1.contains("Second paragraph."));
    }
}
