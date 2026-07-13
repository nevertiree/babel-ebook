//! LLM-based structural refinement of OCR page results.
//!
//! Vision models used for OCR often return fragmented paragraphs, mis-classified
//! headings, or tables rendered as loose cells. This module sends the raw OCR
//! output (together with the original page image) back to an LLM and asks it to
//! clean and restructure the blocks. The refinement can be run for multiple
//! rounds until the structure stabilises.

use serde::Deserialize;

use crate::core::BabelEbookError;
use crate::pdf_ocr::backend::{BlockType, OcrPageResult, TextBlock};

/// A backend that can refine a raw OCR page result into a cleaner structural
/// representation.
#[async_trait::async_trait]
pub trait RefineBackend: Send + Sync {
    /// Refine `current` using the original page image and an optional previous
    /// refinement result for the same page.
    async fn refine(
        &self,
        image_bytes: &[u8],
        mime_type: &str,
        current: &OcrPageResult,
        previous: Option<&OcrPageResult>,
        round: usize,
    ) -> Result<OcrPageResult, BabelEbookError>;
}

/// Configuration for the OpenAI-compatible refinement backend.
#[derive(Debug, Clone)]
pub struct OpenAiRefineConfig {
    /// API key.
    pub api_key: String,
    /// Base URL, e.g. `https://api.openai.com/v1`.
    pub base_url: String,
    /// Model name.
    pub model: String,
    /// Maximum tokens allowed in the refinement response.
    pub max_tokens: u32,
    /// Whether to include the page image in the refinement prompt. Text-only
    /// refinement is cheaper and works well when the raw OCR already contains
    /// the visible text; enable images only when geometry or layout is critical.
    pub include_image: bool,
}

impl Default for OpenAiRefineConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: String::from("https://api.openai.com/v1"),
            model: String::from("gpt-4o"),
            max_tokens: 4096,
            include_image: false,
        }
    }
}

/// Refinement backend using any OpenAI-compatible chat completions endpoint.
pub struct OpenAiRefineBackend {
    client: reqwest::Client,
    config: OpenAiRefineConfig,
}

impl OpenAiRefineBackend {
    /// Create a new backend from config.
    #[must_use]
    pub fn new(config: OpenAiRefineConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }
}

#[async_trait::async_trait]
impl RefineBackend for OpenAiRefineBackend {
    async fn refine(
        &self,
        image_bytes: &[u8],
        mime_type: &str,
        current: &OcrPageResult,
        previous: Option<&OcrPageResult>,
        round: usize,
    ) -> Result<OcrPageResult, BabelEbookError> {
        let (image_context, image_url) = if self.config.include_image {
            let base64_image = encode_base64(image_bytes);
            let data_url = format!("data:{mime_type};base64,{base64_image}");
            (
                "You are given the rendered page image and the raw structured OCR output for that page.".to_string(),
                Some(data_url),
            )
        } else {
            (
                "You are given the raw structured OCR output for a scanned page.".to_string(),
                None,
            )
        };

        let system_message = serde_json::json!({
            "role": "system",
            "content": format!("You are an OCR post-processor. {}\n\nYour task is to clean and restructure the OCR output.\n\nReturn exactly one valid JSON object with a top-level 'blocks' array. Each block must have:\n- 'text': the cleaned text visible in the page (string)\n- 'block_type': one of heading, subheading, paragraph, caption, table_cell, other\n\nDo not include confidence, bbox, or any other top-level fields besides 'blocks'.\n\nRules:\n1. Only keep text that is actually visible. Remove page numbers, running headers, footers, leaked prompts, examples, and instructions.\n2. Merge fragmented paragraphs into single, coherent paragraph blocks. If a paragraph was broken across multiple blocks, join them into one block. In particular, never leave a single CJK character (e.g. '最', '后', '空', '数') as its own paragraph; merge it with the surrounding text to reconstruct the original sentence. Do not break a sentence in the middle of a word or CJK character. Preserve the original sentence boundaries and do not invent content.\n3. Convert fragmented tables into clean Markdown tables inside a single paragraph block when possible. If the table structure is complex and cell positions are clear, you may emit each cell as a table_cell block instead.\n4. Section headings: preserve the exact heading text including the leading number. Top-level chapter headings look like '1 标题', '2 标题' or Chinese '第1章 标题' and must be block_type 'heading'. Nested subheadings look like '1.1 标题', '1.1.1 标题' or '4.4 标题' and must be block_type 'subheading'. Never strip the leading number from a heading.\n5. Captions starting with '图', '表', 'Fig', or 'Table' must be caption blocks. A table title such as '表1 ...' is a caption, not a heading.\n6. Diagram labels, formulas, flowcharts, and isolated non-sentence fragments should be 'other'.\n7. Author names, abstracts, and keywords should be paragraph blocks, not headings.\n8. Do not wrap the output in markdown code fences. Return raw JSON only.", image_context)
        });

        let previous_hint = previous.map_or_else(String::new, |p| {
            format!(
                "\nThis is refinement round {}. The previous round produced {} blocks. Use it as a reference but improve any remaining structural errors.",
                round,
                p.blocks.len()
            )
        });

        let user_text = format!(
            "Raw OCR output (JSON):\n{}\n{}\n\nReturn the cleaned, restructured OCR output as raw JSON.",
            serde_json::to_string(&current).unwrap_or_default(),
            previous_hint
        );

        let user_content: Vec<serde_json::Value> = image_url.map_or_else(
            || vec![serde_json::json!({ "type": "text", "text": user_text })],
            |url| {
                vec![
                    serde_json::json!({ "type": "image_url", "image_url": { "url": url } }),
                    serde_json::json!({ "type": "text", "text": user_text }),
                ]
            },
        );

        let user_message = serde_json::json!({
            "role": "user",
            "content": user_content
        });

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": vec![system_message, user_message],
            "temperature": 0.0,
            "max_tokens": self.config.max_tokens,
            "response_format": { "type": "json_object" }
        });

        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| BabelEbookError::ApiError(format!("refine request failed: {e}")))?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
            BabelEbookError::ApiError(format!("failed to read refine response: {e}"))
        })?;

        if !status.is_success() {
            return Err(BabelEbookError::ApiError(format!(
                "refine returned {status}: {response_text}"
            )));
        }

        let content = extract_response_text(&response_text)
            .ok_or_else(|| BabelEbookError::ApiError("empty refine response".into()))?;

        parse_refined_json(&content, current)
    }
}

/// Parse the JSON returned by the refinement LLM and map it back onto the
/// original page, preserving original bounding boxes where blocks can be
/// matched.
fn parse_refined_json(
    content: &str,
    original: &OcrPageResult,
) -> Result<OcrPageResult, BabelEbookError> {
    let cleaned = super::strip_json_comments(content);
    let cleaned = strip_markdown_fences(&cleaned);

    let raw: RawRefineResponse = parse_refine_response(&cleaned).map_err(|e| {
        BabelEbookError::ApiError(format!(
            "failed to parse refined JSON: {e}. Content: {content}"
        ))
    })?;

    let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut blocks = Vec::with_capacity(raw.blocks.len());

    for refined in raw.blocks {
        let text = refined.text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        let block_type = refined.block_type.unwrap_or(BlockType::Paragraph);

        let matched = find_original_blocks(&text, original, &mut used);
        let bbox = matched
            .iter()
            .filter_map(|idx| original.blocks[*idx].bbox)
            .reduce(union_bbox);
        let confidence = matched
            .iter()
            .map(|idx| original.blocks[*idx].confidence)
            .fold(0.0, f32::max)
            .max(0.8);

        blocks.push(TextBlock {
            text,
            confidence,
            bbox,
            block_type,
        });
    }

    let full_text = blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(OcrPageResult {
        page_number: original.page_number,
        blocks,
        full_text,
    })
}

/// Try to match a refined text block to one or more consecutive original blocks.
fn find_original_blocks(
    text: &str,
    original: &OcrPageResult,
    used: &mut std::collections::HashSet<usize>,
) -> Vec<usize> {
    let normalized_refined = normalize_match_text(text);

    // First try a single exact match.
    for (i, block) in original.blocks.iter().enumerate() {
        if used.contains(&i) {
            continue;
        }
        if normalize_match_text(&block.text) == normalized_refined {
            used.insert(i);
            return vec![i];
        }
    }

    // Then try to match a run of consecutive unused original blocks whose
    // concatenated text equals the refined text.
    for start in 0..original.blocks.len() {
        let mut concatenated = String::new();
        let mut run = Vec::new();
        for i in start..original.blocks.len() {
            if used.contains(&i) {
                break;
            }
            if !concatenated.is_empty() {
                concatenated.push('\n');
            }
            concatenated.push_str(&original.blocks[i].text);
            run.push(i);
            if normalize_match_text(&concatenated) == normalized_refined {
                for idx in &run {
                    used.insert(*idx);
                }
                return run;
            }
        }
    }

    // Fallback: some refined blocks (especially diagrams or merged tables) may
    // not match exactly because the model reordered or rewrote the text. Try
    // to find a contiguous run of unused original blocks whose combined text
    // closely contains the refined text.
    for start in 0..original.blocks.len() {
        let mut concatenated = String::new();
        let mut run = Vec::new();
        for i in start..original.blocks.len() {
            if used.contains(&i) {
                break;
            }
            if !concatenated.is_empty() {
                concatenated.push('\n');
            }
            concatenated.push_str(&original.blocks[i].text);
            run.push(i);
            if is_close_match(&normalized_refined, &normalize_match_text(&concatenated)) {
                for idx in &run {
                    used.insert(*idx);
                }
                return run;
            }
        }
    }

    Vec::new()
}

/// Decide whether two normalized texts are close enough to be considered the
/// same block after model rewriting.
fn is_close_match(refined: &str, original: &str) -> bool {
    if refined.is_empty() || original.is_empty() {
        return false;
    }
    let refined_len = refined.len();
    let original_len = original.len();
    let longer = refined_len.max(original_len);
    let shorter = refined_len.min(original_len);
    // Require at least 80% length overlap and one to contain the other.
    if shorter * 10 < longer * 8 {
        return false;
    }
    refined.contains(original) || original.contains(refined)
}

fn normalize_match_text(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn union_bbox(
    first: crate::pdf_ocr::backend::BoundingBox,
    second: crate::pdf_ocr::backend::BoundingBox,
) -> crate::pdf_ocr::backend::BoundingBox {
    let left = first.x.min(second.x);
    let top = first.y.min(second.y);
    let right = (first.x + first.w).max(second.x + second.w);
    let bottom = (first.y + first.h).max(second.y + second.h);
    crate::pdf_ocr::backend::BoundingBox {
        x: left,
        y: top,
        w: right - left,
        h: bottom - top,
    }
}

fn strip_markdown_fences(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with("```") {
        let without_start = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_start();
        if let Some(end) = without_start.rfind("```") {
            return without_start[..end].trim().to_string();
        }
        return without_start.trim().to_string();
    }
    trimmed.to_string()
}

/// Parse the refinement response, tolerating models that return the JSON
/// object directly, wrap it in a JSON string, or embed it inside extra text.
fn parse_refine_response(cleaned: &str) -> Result<RawRefineResponse, serde_json::Error> {
    // Most common case: the model returns the object directly.
    if let Ok(raw) = serde_json::from_str::<RawRefineResponse>(cleaned) {
        return Ok(raw);
    }

    // Some models return a JSON-encoded string containing the object.
    if let Ok(inner) = serde_json::from_str::<String>(cleaned) {
        if let Ok(raw) = serde_json::from_str::<RawRefineResponse>(&inner) {
            return Ok(raw);
        }
    }

    // Fall back to extracting the first top-level JSON object.
    if let Some(start) = cleaned.find('{') {
        let rest = &cleaned[start..];
        // Find the matching closing brace by tracking brace depth.
        let mut depth = 0i32;
        let mut end = None;
        for (i, c) in rest.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i + c.len_utf8());
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(e) = end {
            return serde_json::from_str::<RawRefineResponse>(&rest[..e]);
        }
    }

    serde_json::from_str::<RawRefineResponse>(cleaned)
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawRefineResponse {
    #[serde(default)]
    blocks: Vec<RawRefineBlock>,
}

#[derive(Debug, Deserialize)]
struct RawRefineBlock {
    text: String,
    #[serde(default)]
    block_type: Option<BlockType>,
}

/// Try to extract the generated text from either an OpenAI chat-completion
/// response or a plain completion-style response such as the one returned by
/// some DashScope models.
fn extract_response_text(response_text: &str) -> Option<String> {
    if let Ok(chat) = serde_json::from_str::<ChatCompletionResponse>(response_text) {
        return chat
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content);
    }

    if let Ok(completion) = serde_json::from_str::<CompletionResponse>(response_text) {
        return completion.text;
    }

    None
}

#[derive(Debug, Deserialize)]
struct CompletionResponse {
    text: Option<String>,
}

fn encode_base64(input: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(input)
}

/// Refine every page up to `max_rounds` times, stopping early when the block
/// structure stops changing.
pub async fn refine_pages(
    backend: &dyn RefineBackend,
    rendered: &[std::path::PathBuf],
    pages: &mut [OcrPageResult],
    max_rounds: usize,
) -> Result<(), BabelEbookError> {
    if max_rounds == 0 {
        return Ok(());
    }

    for (page_idx, page) in pages.iter_mut().enumerate() {
        let Some(image_path) = rendered.get(page_idx) else {
            continue;
        };
        let image_bytes = std::fs::read(image_path).map_err(|e| {
            BabelEbookError::Anyhow(anyhow::anyhow!(
                "failed to read rendered page image {}: {e}",
                image_path.display()
            ))
        })?;

        let mut previous: Option<OcrPageResult> = None;
        for round in 1..=max_rounds {
            let refined = backend
                .refine(&image_bytes, "image/png", page, previous.as_ref(), round)
                .await?;

            let stable = previous.as_ref().is_some_and(|p| blocks_equal(p, &refined));

            *page = refined.clone();
            if stable {
                tracing::info!(
                    page = page.page_number,
                    round = round,
                    "OCR refinement stabilised"
                );
                break;
            }
            previous = Some(refined);
        }
    }

    Ok(())
}

fn blocks_equal(a: &OcrPageResult, b: &OcrPageResult) -> bool {
    if a.blocks.len() != b.blocks.len() {
        return false;
    }
    a.blocks
        .iter()
        .zip(b.blocks.iter())
        .all(|(x, y)| x.text == y.text && x.block_type == y.block_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf_ocr::backend::{BlockType, TextBlock};

    #[test]
    fn strips_markdown_code_fence() {
        let input = "```json\n{\"blocks\":[]}\n```";
        assert_eq!(strip_markdown_fences(input), "{\"blocks\":[]}");
    }

    #[test]
    fn parses_refined_json_and_matches_bbox() {
        let original = OcrPageResult {
            page_number: 1,
            blocks: vec![TextBlock {
                text: "1.1  Details".into(),
                block_type: BlockType::Paragraph,
                bbox: Some(crate::pdf_ocr::backend::BoundingBox {
                    x: 10,
                    y: 20,
                    w: 100,
                    h: 30,
                }),
                confidence: 0.9,
            }],
            full_text: String::new(),
        };

        let content = r#"{"blocks":[{"text":"1.1 Details","block_type":"subheading"}]}"#;
        let refined = parse_refined_json(content, &original).unwrap();
        assert_eq!(refined.blocks.len(), 1);
        assert_eq!(refined.blocks[0].block_type, BlockType::Subheading);
        assert!(refined.blocks[0].bbox.is_some());
    }
}
