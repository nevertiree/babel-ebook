//! Qwen-VL-OCR backend for PDF page OCR.
//!
//! Uses the DashScope OpenAI-compatible endpoint. The model `qwen-vl-ocr` is
//! optimised for Chinese and multilingual scanned documents.

use serde::Deserialize;

use crate::core::BabelEbookError;
use crate::pdf_ocr::backend::{BlockType, BoundingBox, OcrBackend, OcrPageResult, TextBlock};

const DEFAULT_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";
const DEFAULT_MODEL: &str = "qwen-vl-ocr";

/// Qwen-VL-OCR backend configuration.
#[derive(Debug, Clone)]
pub struct QwenOcrConfig {
    /// API key for DashScope.
    pub api_key: String,
    /// Optional custom base URL.
    pub base_url: Option<String>,
    /// Model name.
    pub model: String,
}

impl QwenOcrConfig {
    /// Create a config from an API key, using the defaults for base URL and model.
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: None,
            model: DEFAULT_MODEL.to_string(),
        }
    }
}

/// Qwen-VL-OCR backend.
pub struct QwenOcrBackend {
    client: reqwest::Client,
    config: QwenOcrConfig,
}

impl QwenOcrBackend {
    /// Create a new backend from the given config.
    #[must_use]
    pub fn new(config: QwenOcrConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }
}

#[async_trait::async_trait]
impl OcrBackend for QwenOcrBackend {
    async fn extract(
        &self,
        image_bytes: &[u8],
        mime_type: &str,
    ) -> Result<OcrPageResult, BabelEbookError> {
        let base64_image = encode_base64(image_bytes);
        let data_url = format!("data:{mime_type};base64,{base64_image}");

        let plain_text_mode = self.config.model.to_lowercase().contains("deepseek-ocr");

        let (system_message, user_message, response_format) = if plain_text_mode {
            let system = serde_json::json!({
                "role": "system",
                "content": "You are a strict OCR engine. Extract only the text that actually appears in the provided page image.\n\nRules:\n1. Output the text exactly as it appears, preserving paragraphs with blank lines between them.\n2. Do not output page numbers, running headers, or footers.\n3. Do not include instructions, examples, or explanations.\n4. Use Markdown tables for tabular content when possible.\n5. Keep diagram labels short and on their own lines."
            });
            let user = serde_json::json!({
                "role": "user",
                "content": [
                    { "type": "image_url", "image_url": { "url": data_url } },
                    { "type": "text", "text": "Extract all text from this page image." }
                ]
            });
            (system, user, serde_json::json!({ "type": "text" }))
        } else {
            let system = serde_json::json!({
                "role": "system",
                "content": "You are a strict OCR engine. Your job is to extract only the text that actually appears in the provided page image.\n\nOutput format: return exactly one valid JSON object with a top-level 'blocks' array. Each block must have:\n- 'text': the exact text visible in the image (string)\n- 'confidence': 0.0-1.0\n- 'bbox': [x, y, w, h] in pixels (array of 4 integers)\n- 'block_type': one of heading, subheading, paragraph, caption, table_cell, other\n\nRules:\n1. Only output text that is visually present in the image.\n2. Do not output any instructions, examples, system messages, or task descriptions.\n3. Do not output page numbers, running headers, or footers as body text.\n4. Do not include comments, markdown, or explanatory notes in the JSON.\n5. For diagrams or figures with no readable sentences, use block_type 'other' and keep text minimal.\n6. For tables, output each cell as a separate table_cell block with accurate bbox, in reading order (left-to-right, top-to-bottom). If the table is complex or has many cells, you may output the whole table as a single markdown table inside a paragraph block instead.\n\nExample of valid output:\n{\"blocks\":[{\"text\":\"Sample heading\",\"confidence\":0.98,\"bbox\":[100,50,200,30],\"block_type\":\"heading\"}]}"
            });
            let user = serde_json::json!({
                "role": "user",
                "content": [
                    { "type": "image_url", "image_url": { "url": data_url } },
                    { "type": "text", "text": "Extract all text from this page image as structured JSON." }
                ]
            });
            (system, user, serde_json::json!({ "type": "json_object" }))
        };

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": vec![system_message, user_message],
            "temperature": 0.0,
            "max_tokens": 4096,
            "response_format": response_format
        });

        let base_url = self.config.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL);
        let url = if base_url.ends_with("/chat/completions") {
            base_url.to_string()
        } else {
            format!("{base_url}/chat/completions")
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| BabelEbookError::ApiError(format!("qwen-vl-ocr request failed: {e}")))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| BabelEbookError::ApiError(format!("failed to read response: {e}")))?;

        if !status.is_success() {
            return Err(BabelEbookError::ApiError(format!(
                "qwen-vl-ocr returned {status}: {response_text}"
            )));
        }

        let chat_response: ChatCompletionResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                BabelEbookError::ApiError(format!(
                    "failed to parse qwen-vl-ocr response: {e}. Body: {response_text}"
                ))
            })?;

        let content = chat_response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| BabelEbookError::ApiError("empty response from qwen-vl-ocr".into()))?;

        if plain_text_mode {
            let blocks = parse_plain_text_ocr(&content);
            let full_text = blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");
            Ok(OcrPageResult {
                page_number: 0,
                blocks,
                full_text,
            })
        } else {
            parse_ocr_json(&content)
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
fn parse_ocr_json(content: &str) -> Result<OcrPageResult, BabelEbookError> {
    let cleaned = super::strip_json_comments(content);

    let blocks: Vec<TextBlock> = if let Ok(raw) = serde_json::from_str::<RawOcrResponse>(&cleaned) {
        raw.blocks
            .into_iter()
            .map(|b| {
                let inferred = infer_json_block_type(&b.text, b.bbox);
                // If the model supplied a structural type, trust it; otherwise use
                // our inference. Paragraphs that look like headings/captions are
                // upgraded so sections and figure/table labels are not lost.
                let block_type = match b.block_type {
                    Some(BlockType::Paragraph) | None => inferred,
                    Some(ty) => ty,
                };
                TextBlock {
                    text: b.text,
                    confidence: b.confidence.clamp(0.0, 1.0),
                    bbox: b.bbox,
                    block_type,
                }
            })
            .collect()
    } else {
        tracing::debug!(content = %content, "OCR response was not structured JSON; falling back to plain-text parsing");
        parse_plain_text_ocr(content)
    };

    let full_text = blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(OcrPageResult {
        page_number: 0,
        blocks,
        full_text,
    })
}

/// Fallback parser for vision models that return plain text instead of the
/// requested JSON structure. Splits the text into blocks, merges line-broken
/// paragraphs, and guesses block types from simple heuristics.
fn parse_plain_text_ocr(content: &str) -> Vec<TextBlock> {
    let mut blocks = Vec::new();
    let mut pending = String::new();

    let lines: Vec<&str> = content.lines().map(str::trim).collect();
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            flush_plain_text_block(&mut blocks, &mut pending);
            continue;
        }

        let next = lines.get(i + 1).copied().unwrap_or("");
        let is_heading = is_plain_text_heading(line);
        let is_caption = line.starts_with('图') || line.starts_with('表');

        // Always break before structural lines (headings, captions) so they
        // start their own block and can split chapters correctly.
        if (is_heading || is_caption || line.starts_with("## ")) && !pending.is_empty() {
            flush_plain_text_block(&mut blocks, &mut pending);
        }

        let pending_is_short = pending.trim().chars().count() <= 20;
        let line_is_short = line.chars().count() <= 20;
        let should_break = if pending.is_empty() {
            true
        } else {
            // Break the paragraph if the previous accumulated text ends a sentence,
            // or if the next line clearly starts a new sentence/structural element.
            let prev_ends = pending
                .trim_end()
                .ends_with(['。', '！', '？', '.', '!', '?', '"']);
            let next_starts_new = next.is_empty()
                || next.starts_with('#')
                || is_plain_text_heading(next)
                || next.starts_with('图')
                || next.starts_with('表')
                || next.starts_with("Fig")
                || next.starts_with("Table");
            // Keep short diagram labels on separate lines instead of gluing them.
            let looks_like_labels = pending_is_short && line_is_short;
            prev_ends || next_starts_new || looks_like_labels
        };

        if should_break && !pending.is_empty() {
            flush_plain_text_block(&mut blocks, &mut pending);
        }

        if !pending.is_empty() {
            pending.push(' ');
        }
        pending.push_str(line);
    }
    flush_plain_text_block(&mut blocks, &mut pending);

    // Filter out artifact-only blocks such as bare bracketed numbers.
    blocks.retain(|b| !is_ocr_artifact(&b.text));
    blocks
}

fn flush_plain_text_block(blocks: &mut Vec<TextBlock>, text: &mut String) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    let block_type = infer_plain_text_block_type(trimmed);
    blocks.push(TextBlock {
        text: trimmed.to_string(),
        block_type,
        confidence: 0.8,
        bbox: None,
    });
    text.clear();
}

fn is_plain_text_heading(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    // Numbered headings like "1. ", "1.1 ", "1金融", "1.1金融".
    let prefix_len = line
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .count();
    if prefix_len == 0 {
        return false;
    }
    let rest = line.chars().skip(prefix_len).collect::<String>();
    if rest.is_empty() {
        return false;
    }
    // Either a space, or the rest starts with a Chinese character.
    rest.starts_with(' ') || rest.starts_with('\u{3000}') || rest.chars().next().is_some_and(is_cjk)
}

fn is_cjk(c: char) -> bool {
    (0x4E00..=0x9FFF).contains(&(c as u32))
}

fn is_ocr_artifact(text: &str) -> bool {
    let trimmed = text.trim();
    // Standalone bracketed numbers like "[1]", "[1.1  ]".
    trimmed.len() <= 10 && trimmed.starts_with('[') && trimmed.ends_with(']')
}

fn infer_plain_text_block_type(text: &str) -> BlockType {
    let trimmed = text.trim();
    if trimmed.starts_with('#') {
        return BlockType::Heading;
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
            || rest.chars().next().is_some_and(is_cjk)
        {
            return BlockType::Heading;
        }
    }
    if trimmed.starts_with('图') || trimmed.starts_with('表') || trimmed.starts_with("Fig") {
        return BlockType::Caption;
    }
    BlockType::Paragraph
}

/// Infer block type for structured JSON responses that may omit the
/// `block_type` field.
///
/// Falls back to the same heuristics used for plain-text OCR, but also treats
/// multi-line diagram label clusters and math-heavy fragments as `Other` so
/// they can be cropped and embedded as images when a bounding box is present.
fn infer_json_block_type(text: &str, bbox: Option<BoundingBox>) -> BlockType {
    let base = infer_plain_text_block_type(text);
    if base != BlockType::Paragraph {
        return base;
    }

    // Without geometry we cannot safely embed anything.
    let Some(_) = bbox else {
        return BlockType::Paragraph;
    };

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return BlockType::Paragraph;
    }

    // Captions should stay as text even if the model forgot the block type.
    if trimmed.starts_with('图') || trimmed.starts_with('表') || trimmed.starts_with("Fig") {
        return BlockType::Caption;
    }

    // Diagrams/figures usually have no sentence terminator and consist of short
    // labels, percentages, Greek letters, or math symbols.
    let lines: Vec<&str> = trimmed
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let has_math_or_symbol = trimmed.chars().any(|c| {
        matches!(
            c,
            'μ' | 'σ' | 'δ' | 'α' | 'β' | 'γ' | 'θ' | 'λ' | 'π' | 'ω' | '∞' | '∑' | '∫'
        ) || c == '%'
    });
    let avg_line_len = trimmed.len() / lines.len().max(1);
    let no_sentence_end = !trimmed
        .chars()
        .last()
        .is_some_and(|c| matches!(c, '。' | '！' | '？' | '.' | '!' | '?' | '"' | '”'));
    let short_label_cluster = lines.len() >= 3 && avg_line_len <= 20;

    if no_sentence_end && (has_math_or_symbol || short_label_cluster) {
        return BlockType::Other;
    }

    BlockType::Paragraph
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
struct RawOcrResponse {
    #[serde(default)]
    blocks: Vec<RawTextBlock>,
}

#[derive(Debug, Deserialize)]
struct RawTextBlock {
    text: String,
    #[serde(default)]
    confidence: f32,
    #[serde(
        default,
        deserialize_with = "crate::pdf_ocr::deserialize_bbox_flexible"
    )]
    bbox: Option<BoundingBox>,
    #[serde(default)]
    block_type: Option<BlockType>,
}

fn encode_base64(input: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(input)
}
