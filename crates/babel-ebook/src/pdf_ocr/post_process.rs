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
            true
        });
    }

    group_diagram_labels(pages);
    merge_paragraphs_across_pages(pages);
    rebuild_full_text(pages);
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
    if !matches!(
        block.block_type,
        BlockType::Paragraph | BlockType::Other | BlockType::Caption
    ) {
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
    // Require at least one non-CJK character to distinguish diagram labels like
    // "μ-3σ" or "2.15%" from ordinary short Chinese phrases.
    text.chars().any(|c| !is_cjk_or_fullwidth(c))
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

/// Merge a paragraph that ends mid-sentence at the bottom of one page with the
/// first paragraph of the next page when the continuation looks plausible.
fn merge_paragraphs_across_pages(pages: &mut [OcrPageResult]) {
    for i in 0..pages.len().saturating_sub(1) {
        let (prev, next) = pages.split_at_mut(i + 1);
        let prev_page = prev.last_mut().unwrap();
        let next_page = next.first_mut().unwrap();

        let Some(last) = prev_page.blocks.last_mut() else {
            continue;
        };
        if last.block_type != BlockType::Paragraph || ends_sentence(&last.text) {
            continue;
        }

        let first_idx = next_page
            .blocks
            .iter()
            .position(|b| b.block_type == BlockType::Paragraph);
        let Some(first_idx) = first_idx else {
            continue;
        };

        let first = &next_page.blocks[first_idx];
        // Only merge if the next paragraph does not itself start a new sentence.
        if starts_with_sentence_fragment(&first.text) {
            let first_text = next_page.blocks.remove(first_idx).text;
            last.text.push_str(&first_text);
        }
    }
}

fn ends_sentence(text: &str) -> bool {
    let trimmed = text.trim_end();
    trimmed.ends_with('。')
        || trimmed.ends_with('！')
        || trimmed.ends_with('？')
        || trimmed.ends_with('.')
        || trimmed.ends_with('!')
        || trimmed.ends_with('?')
        || trimmed.ends_with('”')
        || trimmed.ends_with('"')
}

/// Heuristic: the text looks like a continuation of the previous sentence
/// rather than the start of a new one.
fn starts_with_sentence_fragment(text: &str) -> bool {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return false;
    }
    let first_char = trimmed.chars().next().unwrap();
    // Starts with lowercase Latin or a number.
    if first_char.is_ascii_lowercase() || first_char.is_ascii_digit() {
        return true;
    }
    // Starts with punctuation that implies continuation.
    let continuations = ["，", "、", "；", ",", ";", " "];
    if continuations.iter().any(|c| trimmed.starts_with(*c)) {
        return true;
    }
    // Chinese text: treat as a continuation unless it starts with a strong
    // sentence or list marker.
    if (0x4E00..=0x9FFF).contains(&(first_char as u32)) {
        let new_sentence_markers = [
            "首先",
            "其次",
            "再次",
            "最后",
            "此外",
            "同时",
            "因此",
            "所以",
            "但是",
            "然而",
            "综上",
            "总之",
            "例如",
            "比如",
            "一方面",
            "另一方面",
            "第一",
            "第二",
            "第三",
            "第",
            "一是",
            "二是",
            "三是",
            "其一",
            "其二",
            "其三",
        ];
        return !new_sentence_markers.iter().any(|m| trimmed.starts_with(m));
    }
    false
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

    #[test]
    fn merges_paragraphs_across_pages() {
        let mut pages = vec![
            OcrPageResult {
                page_number: 1,
                blocks: vec![block("该算法通过数据", BlockType::Paragraph)],
                full_text: String::new(),
            },
            OcrPageResult {
                page_number: 2,
                blocks: vec![block("预处理、计算基线、实时检测", BlockType::Paragraph)],
                full_text: String::new(),
            },
        ];
        clean_pages(&mut pages);
        assert_eq!(pages[0].blocks.len(), 1);
        assert_eq!(pages[1].blocks.len(), 0);
        assert_eq!(
            pages[0].blocks[0].text,
            "该算法通过数据预处理、计算基线、实时检测"
        );
    }
}
