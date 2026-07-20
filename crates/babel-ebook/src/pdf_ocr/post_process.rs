//! Post-processing heuristics for OCR page results.
//!
//! Vision models often emit noise such as page numbers, running headers, footers,
//! prompt leakage, or diagram text. This module cleans those artifacts before the
//! results are assembled into an EPUB.

use crate::pdf_ocr::backend::{BlockType, OcrPageResult, TextBlock};

/// Clean a set of pages by removing common OCR artifacts.
pub fn clean_pages(pages: &mut [OcrPageResult]) {
    if pages.is_empty() {
        return;
    }

    remove_prompt_leakage(pages);

    let repeated = find_repeated_headers(pages);
    for page in pages.iter_mut() {
        page.blocks.retain(|block| {
            if is_page_number(&block.text) {
                return false;
            }
            if repeated.contains(&normalize_header(&block.text)) {
                return false;
            }
            if contains_repeated_header(&block.text, &repeated) {
                return false;
            }
            true
        });
    }

    group_diagram_labels(pages);
    rebuild_full_text(pages);
}

/// Remove blocks that are dominated by a known repeated header, even if the
/// model merged the header with a few extra characters.
fn contains_repeated_header(text: &str, repeated: &std::collections::HashSet<String>) -> bool {
    let normalized = normalize_header(text);
    for header in repeated {
        if normalized.starts_with(header) {
            // Allow only a small amount of extra trailing content.
            return normalized.len() <= header.len() + 10;
        }
    }
    false
}

/// Remove blocks that contain fragments of the system prompt or task description.
fn remove_prompt_leakage(pages: &mut [OcrPageResult]) {
    for page in pages.iter_mut() {
        page.blocks.retain(|block| !is_prompt_leakage(&block.text));
    }
}

/// Detect text that looks like leaked instructions or example content rather
/// than real document content.
fn is_prompt_leakage(text: &str) -> bool {
    let lower = text.to_lowercase();
    let markers = [
        "epub output should",
        "preserve heading hierarchy",
        "generate a table of contents",
        "ocr confidence is low",
        "verification backend",
        "review low-confidence",
        "extract all text from the image",
        "return only a valid json",
        "top-level 'blocks' array",
        "each block must have",
        "you are a strict ocr engine",
        "you are an ocr engine",
        "your job is to extract",
        "output format:",
        "rules:",
        "example of valid output",
        "sample document",
        "testing pdf ocr",
        "rendered as page images",
        "vision - language model",
        "vision-language model",
        "layout analysis",
        "reading order",
        "chapter segmentation",
        "babelbook",
        "babel-ebook",
        "test document",
        "## chapter",
        "chapter 2: structure",
        "sample heading",
    ];
    if markers.iter().any(|m| lower.contains(m)) {
        return true;
    }

    // Example chapter titles like "Chapter 1: Origin" or "Chapter 2: Structure".
    let trimmed = lower.trim();
    trimmed.starts_with("chapter ")
        && trimmed
            .strip_prefix("chapter ")
            .and_then(|rest| rest.chars().next())
            .is_some_and(|c| c.is_ascii_digit())
}

/// Detect standalone numeric strings that are likely page numbers.
fn is_page_number(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Pure digits, 1-4 digits long, not part of a larger number/date.
    if trimmed.len() <= 4 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    // Page ranges like "890 - 905." or "890-905".
    if trimmed.len() <= 15
        && trimmed.split('-').map(str::trim).all(|part| {
            part.trim_end_matches('.')
                .chars()
                .all(|c| c.is_ascii_digit())
        })
        && trimmed.contains('-')
    {
        return true;
    }
    false
}

/// Find short text strings that appear on many pages and are likely headers or
/// footers. Returns a set of normalized (whitespace-stripped) strings.
fn find_repeated_headers(pages: &[OcrPageResult]) -> std::collections::HashSet<String> {
    let page_count = pages.len();
    if page_count < 3 {
        return std::collections::HashSet::new();
    }

    let threshold = page_count / 5;
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for page in pages {
        let mut seen_on_page = std::collections::HashSet::new();
        for block in &page.blocks {
            let trimmed = block.text.trim();
            if trimmed.is_empty() || trimmed.len() > 80 {
                continue;
            }
            seen_on_page.insert(normalize_header(trimmed));
        }
        for text in seen_on_page {
            *counts.entry(text).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .filter(|(_, count)| *count >= threshold)
        .map(|(text, _)| text)
        .collect()
}

/// Normalize a header candidate by removing whitespace so that
/// "金融 IT 运维新探索" and "金融IT运维新探索" are treated as the same string.
fn normalize_header(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Group consecutive short text fragments that look like diagram labels into
/// a single `other` block. This prevents figures from being rendered as a
/// scattered series of one-line paragraphs.
fn group_diagram_labels(pages: &mut [OcrPageResult]) {
    for page in pages.iter_mut() {
        let mut grouped: Vec<TextBlock> = Vec::new();
        let mut run: Vec<TextBlock> = Vec::new();

        for block in page.blocks.drain(..) {
            if is_diagram_label(&block) {
                run.push(block);
                continue;
            }

            if !run.is_empty() {
                grouped.push(flush_diagram_run(&mut run));
            }
            grouped.push(block);
        }

        if !run.is_empty() {
            grouped.push(flush_diagram_run(&mut run));
        }

        page.blocks = grouped;
    }
}

/// A block is treated as a diagram label if it is short, contains no sentence
/// terminator, is not a structural element, and contains at least one non-CJK
/// character (Latin, digit, or symbol). This avoids grouping normal short
/// Chinese phrases that happen to cross a page boundary.
fn is_diagram_label(block: &TextBlock) -> bool {
    // Headings and captions should keep their semantic type; don't turn them
    // into diagram label clusters.
    if matches!(
        block.block_type,
        BlockType::Heading | BlockType::Subheading | BlockType::Caption
    ) {
        return false;
    }
    if !matches!(block.block_type, BlockType::Paragraph | BlockType::Other) {
        return false;
    }
    let text = block.text.trim();
    if text.is_empty() || text.len() > 25 {
        return false;
    }
    // Avoid grouping real sentences that happen to be short.
    if text.ends_with('。') || text.ends_with('！') || text.ends_with('？') {
        return false;
    }
    if text.ends_with('.') || text.ends_with('!') || text.ends_with('?') {
        return false;
    }
    // Don't swallow section headers or figure/table captions that the OCR
    // backend mis-typed as paragraphs.
    if looks_like_heading(text) || looks_like_caption(text) {
        return false;
    }
    // Diagram labels are either non-CJK (e.g. "μ-3σ", "2.15%") or very short
    // pure Chinese fragments (e.g. "监控", "日志", "北斗").
    let has_non_cjk = text.chars().any(|c| !is_cjk_or_fullwidth(c));
    let short_pure_cjk = text.chars().count() <= 6 && text.chars().all(is_cjk_or_fullwidth);
    has_non_cjk || short_pure_cjk
}

fn looks_like_heading(text: &str) -> bool {
    let trimmed = text.trim_start();
    if trimmed.starts_with('#') {
        return true;
    }
    // Numbered headings like "1.", "1.1", "1金融", "1.1金融" or Chinese "第1章".
    let prefix_len = trimmed
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .count();
    if prefix_len > 0 {
        let rest = trimmed.chars().skip(prefix_len).collect::<String>();
        if rest.starts_with(' ')
            || rest.starts_with('\u{3000}')
            || rest.chars().next().is_some_and(is_cjk_or_fullwidth)
        {
            return true;
        }
    }
    trimmed.starts_with('第') && trimmed.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
}

fn looks_like_caption(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with('图')
        || trimmed.starts_with('表')
        || trimmed.starts_with("Fig")
        || trimmed.starts_with("Table")
}

fn is_cjk_or_fullwidth(c: char) -> bool {
    (0x4E00..=0x9FFF).contains(&(c as u32))
        || (0x3040..=0x309F).contains(&(c as u32))
        || (0x30A0..=0x30FF).contains(&(c as u32))
        || (0xFF00..=0xFFEF).contains(&(c as u32))
        || (0x3000..=0x303F).contains(&(c as u32))
}

/// Flush a run of diagram labels into one `other` block, preserving the text
/// in a layout-friendly form.
fn flush_diagram_run(run: &mut Vec<TextBlock>) -> TextBlock {
    let first = run.first().cloned().unwrap_or_default();
    let text = run
        .iter()
        .map(|b| b.text.trim())
        .collect::<Vec<_>>()
        .join("\n");
    run.clear();
    TextBlock {
        text,
        block_type: BlockType::Other,
        bbox: first.bbox,
        confidence: first.confidence,
    }
}

fn rebuild_full_text(pages: &mut [OcrPageResult]) {
    for page in pages.iter_mut() {
        page.full_text = page
            .blocks
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf_ocr::backend::{BlockType, TextBlock};

    fn block(text: &str, block_type: BlockType) -> TextBlock {
        TextBlock {
            text: text.to_string(),
            block_type,
            ..Default::default()
        }
    }

    #[test]
    fn removes_prompt_leakage() {
        let mut pages = vec![OcrPageResult {
            page_number: 1,
            blocks: vec![
                block("Real content", BlockType::Paragraph),
                block(
                    "The EPUB output should preserve heading hierarchy.",
                    BlockType::Paragraph,
                ),
            ],
            full_text: String::new(),
        }];
        clean_pages(&mut pages);
        assert_eq!(pages[0].blocks.len(), 1);
        assert_eq!(pages[0].blocks[0].text, "Real content");
    }

    #[test]
    fn removes_page_numbers() {
        let mut pages = vec![OcrPageResult {
            page_number: 1,
            blocks: vec![
                block("Introduction", BlockType::Paragraph),
                block("229", BlockType::Paragraph),
            ],
            full_text: String::new(),
        }];
        clean_pages(&mut pages);
        assert!(!pages[0].blocks.iter().any(|b| b.text == "229"));
    }

    #[test]
    fn removes_repeated_headers() {
        let mut pages: Vec<OcrPageResult> = (0..5)
            .map(|i| OcrPageResult {
                page_number: i + 1,
                blocks: vec![
                    block("Journal Header", BlockType::Paragraph),
                    block(&format!("Content {}", i + 1), BlockType::Paragraph),
                ],
                full_text: String::new(),
            })
            .collect();
        clean_pages(&mut pages);
        for page in &pages {
            assert!(!page.blocks.iter().any(|b| b.text == "Journal Header"));
        }
    }

    #[test]
    fn removes_repeated_heading_headers() {
        let mut pages: Vec<OcrPageResult> = (0..5)
            .map(|i| OcrPageResult {
                page_number: i + 1,
                blocks: vec![
                    block("Running Title", BlockType::Heading),
                    block(&format!("Content {}", i + 1), BlockType::Paragraph),
                ],
                full_text: String::new(),
            })
            .collect();
        clean_pages(&mut pages);
        for page in &pages {
            assert!(!page.blocks.iter().any(|b| b.text == "Running Title"));
        }
    }

    #[test]
    fn removes_repeated_headers_with_whitespace_variants() {
        let mut pages: Vec<OcrPageResult> = (0..5)
            .map(|i| OcrPageResult {
                page_number: i + 1,
                blocks: vec![
                    block(
                        if i % 2 == 0 {
                            "Journal Header"
                        } else {
                            "Journal  Header"
                        },
                        BlockType::Paragraph,
                    ),
                    block(&format!("Content {}", i + 1), BlockType::Paragraph),
                ],
                full_text: String::new(),
            })
            .collect();
        clean_pages(&mut pages);
        for page in &pages {
            assert!(!page.blocks.iter().any(|b| b.text.contains("Journal")));
        }
    }

    #[test]
    fn groups_short_diagram_labels() {
        let mut pages = vec![OcrPageResult {
            page_number: 1,
            blocks: vec![
                block("μ-3σ", BlockType::Paragraph),
                block("μ", BlockType::Paragraph),
                block("μ+3σ", BlockType::Paragraph),
                block("This is a real sentence.", BlockType::Paragraph),
            ],
            full_text: String::new(),
        }];
        clean_pages(&mut pages);
        assert_eq!(pages[0].blocks.len(), 2);
        assert_eq!(pages[0].blocks[0].block_type, BlockType::Other);
        assert!(pages[0].blocks[0].text.contains("μ-3σ"));
        assert!(pages[0].blocks[0].text.contains("μ+3σ"));
    }
}
