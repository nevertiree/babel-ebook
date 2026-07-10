//! Post-processing heuristics for OCR page results.
//!
//! Vision models often emit noise such as page numbers, running headers, footers,
//! prompt leakage, or diagram text. This module cleans those artifacts before the
//! results are assembled into an EPUB.

use crate::pdf_ocr::backend::OcrPageResult;

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
            true
        });
    }
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
    false
}

/// Find short text strings that appear on many pages and are likely headers or
/// footers. Returns a set of normalized (whitespace-stripped) strings.
fn find_repeated_headers(pages: &[OcrPageResult]) -> std::collections::HashSet<String> {
    let page_count = pages.len();
    if page_count < 3 {
        return std::collections::HashSet::new();
    }

    let threshold = page_count / 4;
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
}
