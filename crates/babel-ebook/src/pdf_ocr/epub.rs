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
    let mut sorted: Vec<&TextBlock> = cells.to_vec();
    sorted.sort_by_key(|b| b.bbox.map_or(0, |bbox| bbox.y));

    let row_threshold = cells
        .iter()
        .filter_map(|b| b.bbox.map(|bbox| bbox.h / 2))
        .min()
        .unwrap_or(10)
        .max(5);

    let mut rows: Vec<Vec<&TextBlock>> = Vec::new();
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

    // Fallback: if every cell landed in a single row, try to infer a grid from
    // repeating x-coordinate cycles. This handles models that emit cells with
    // poor y coordinates.
    if rows.len() == 1 && cells.len() > 4 {
        if let Some(grid_rows) = infer_grid_by_x_coords(cells) {
            rows = grid_rows;
        }
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

/// Try to infer rows from a flat list of cells by detecting a repeating cycle
/// in their x coordinates. Returns rows in reading order when possible.
fn infer_grid_by_x_coords<'a>(cells: &'a [&'a TextBlock]) -> Option<Vec<Vec<&'a TextBlock>>> {
    let xs: Vec<u32> = cells
        .iter()
        .map(|b| b.bbox.map_or(0, |bbox| bbox.x))
        .collect();
    if xs.len() < 6 {
        return None;
    }

    // Find the smallest cycle length (number of columns) that explains the data.
    for col_count in 2..=xs.len() / 2 {
        if !xs.len().is_multiple_of(col_count) {
            continue;
        }
        // Check if positions within each column are roughly stable.
        let mut columns_ok = true;
        for col in 0..col_count {
            let values: Vec<u32> = xs.iter().skip(col).step_by(col_count).copied().collect();
            if values.len() < 2 {
                columns_ok = false;
                break;
            }
            let avg = values.iter().sum::<u32>() / u32::try_from(values.len()).unwrap_or(1);
            let max_dev = values.iter().map(|v| v.abs_diff(avg)).max().unwrap_or(0);
            // Allow 15% of page width or 50px, whichever is larger.
            let threshold = avg / 6 + 50;
            if max_dev > threshold {
                columns_ok = false;
                break;
            }
        }
        if columns_ok {
            let mut rows: Vec<Vec<&'a TextBlock>> = Vec::new();
            for chunk in cells.chunks(col_count) {
                rows.push(chunk.to_vec());
            }
            return Some(rows);
        }
    }

    None
}

fn block_to_html(block: &TextBlock) -> String {
    let escaped = html_escape(&block.text);
    match block.block_type {
        BlockType::Heading => format!("<h1>{escaped}</h1>"),
        BlockType::Subheading => format!("<h2>{escaped}</h2>"),
        BlockType::Caption => format!("<figcaption>{escaped}</figcaption>"),
        BlockType::TableCell => format!("<td>{escaped}</td>"),
        BlockType::Other => {
            // Diagram labels and similar fragments are grouped with newlines;
            // render them in a figure/pre block to preserve layout.
            if escaped.lines().count() > 1 {
                format!("<figure><pre>{escaped}</pre></figure>")
            } else {
                format!("<p>{escaped}</p>")
            }
        }
        BlockType::Paragraph => {
            // Render embedded Markdown tables as HTML tables.
            if let Some(table_html) = render_markdown_table(&block.text) {
                return table_html;
            }
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

/// Render a Markdown table embedded in a paragraph as an HTML table.
/// Returns `None` if the text does not contain a Markdown table.
fn render_markdown_table(text: &str) -> Option<String> {
    // Some models concatenate Markdown rows with spaces; normalize to one row
    // per line first.
    let normalized = normalize_markdown_table(text);
    let lines: Vec<&str> = normalized.lines().collect();
    let mut table_lines: Vec<&str> = Vec::new();
    let mut in_table = false;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            in_table = true;
            table_lines.push(trimmed);
        } else if in_table {
            break;
        }
    }

    if table_lines.len() < 2 {
        return None;
    }

    // The second line should be the separator line (e.g. |---|---|).
    let separator = table_lines[1];
    let is_separator = separator
        .split('|')
        .skip(1)
        .take_while(|s| !s.is_empty())
        .all(|s| s.trim().chars().all(|c| c == '-' || c == ':' || c == ' '));
    if !is_separator {
        return None;
    }

    let header_cells = split_markdown_row(table_lines[0]);
    let mut html = String::from("<table>\n  <thead>\n    <tr>");
    for cell in header_cells {
        let escaped = html_escape(cell.trim());
        let _ = write!(html, "<th>{escaped}</th>");
    }
    html.push_str("</tr>\n  </thead>\n  <tbody>\n");

    for row in table_lines.iter().skip(2) {
        html.push_str("    <tr>");
        for cell in split_markdown_row(row) {
            let escaped = html_escape(cell.trim());
            let _ = write!(html, "<td>{escaped}</td>");
        }
        html.push_str("</tr>\n");
    }
    html.push_str("  </tbody>\n</table>");
    Some(html)
}

/// Some vision models emit Markdown tables with rows concatenated on a single
/// line separated by spaces. Split them into one row per line.
fn normalize_markdown_table(text: &str) -> String {
    // Match sequences like "| a | b | | c | d |" and split after every closing
    // pipe that is followed by a space and another pipe.
    let mut result = String::new();
    let mut row = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        row.push(chars[i]);
        if chars[i] == '|' && i + 2 < chars.len() && chars[i + 1] == ' ' && chars[i + 2] == '|' {
            result.push_str(row.trim());
            result.push('\n');
            row.clear();
        }
        i += 1;
    }
    if !row.is_empty() {
        result.push_str(row.trim());
    }
    result
}

fn split_markdown_row(row: &str) -> Vec<&str> {
    row.trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .collect()
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

    #[test]
    fn renders_markdown_table_in_paragraph() {
        let pages = vec![OcrPageResult {
            page_number: 1,
            blocks: vec![TextBlock {
                text: "| Name | Value |\n|------|-------|\n| A | 1 |\n| B | 2 |".into(),
                block_type: BlockType::Paragraph,
                ..Default::default()
            }],
            full_text: String::new(),
        }];
        let book = build_epub("Test", &pages);
        let content = String::from_utf8_lossy(&book.chapters[0].content);
        assert!(content.contains("<table>"));
        assert!(content.contains("<th>Name</th>"));
        assert!(content.contains("<td>1</td>"));
    }

    #[test]
    fn renders_inline_markdown_table_in_paragraph() {
        let pages = vec![OcrPageResult {
            page_number: 1,
            blocks: vec![TextBlock {
                text: "| A | B | |---|---| | 1 | 2 | | 3 | 4 |".into(),
                block_type: BlockType::Paragraph,
                ..Default::default()
            }],
            full_text: String::new(),
        }];
        let book = build_epub("Test", &pages);
        let content = String::from_utf8_lossy(&book.chapters[0].content);
        assert!(content.contains("<table>"));
        assert!(content.contains("<td>3</td>"));
        assert!(content.contains("<td>4</td>"));
    }

    #[test]
    fn renders_multi_line_other_as_figure_pre() {
        let pages = vec![OcrPageResult {
            page_number: 1,
            blocks: vec![TextBlock {
                text: "μ-3σ\nμ\nμ+3σ".into(),
                block_type: BlockType::Other,
                ..Default::default()
            }],
            full_text: String::new(),
        }];
        let book = build_epub("Test", &pages);
        let content = String::from_utf8_lossy(&book.chapters[0].content);
        assert!(content.contains("<figure><pre>"));
        assert!(content.contains("μ-3σ"));
        assert!(content.contains("μ+3σ"));
    }
}
