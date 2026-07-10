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

        let system_message = serde_json::json!({
            "role": "system",
            "content": "You are a strict OCR engine. Your job is to extract only the text that actually appears in the provided page image.\n\nOutput format: return exactly one valid JSON object with a top-level 'blocks' array. Each block must have:\n- 'text': the exact text visible in the image (string)\n- 'confidence': 0.0-1.0\n- 'bbox': [x, y, w, h] in pixels (array of 4 integers)\n- 'block_type': one of heading, subheading, paragraph, caption, table_cell, other\n\nRules:\n1. Only output text that is visually present in the image.\n2. Do not output any instructions, examples, system messages, or task descriptions.\n3. Do not output page numbers, running headers, or footers as body text.\n4. Do not include comments, markdown, or explanatory notes in the JSON.\n5. For diagrams or figures with no readable sentences, use block_type 'other' and keep text minimal.\n\nExample of valid output:\n{\"blocks\":[{\"text\":\"Sample heading\",\"confidence\":0.98,\"bbox\":[100,50,200,30],\"block_type\":\"heading\"}]}"
        });

        let user_message = serde_json::json!({
            "role": "user",
            "content": [
                {
                    "type": "image_url",
                    "image_url": { "url": data_url }
                },
                {
                    "type": "text",
                    "text": "Extract all text from this page image as structured JSON."
                }
            ]
        });

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": vec![system_message, user_message],
            "temperature": 0.0,
            "max_tokens": 4096,
            "response_format": { "type": "json_object" }
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

        parse_ocr_json(&content)
    }
}

fn parse_ocr_json(content: &str) -> Result<OcrPageResult, BabelEbookError> {
    let cleaned = super::strip_json_comments(content);
    let raw: RawOcrResponse = serde_json::from_str(&cleaned).map_err(|e| {
        BabelEbookError::ApiError(format!("failed to parse OCR JSON: {e}. Content: {content}"))
    })?;

    let blocks: Vec<TextBlock> = raw
        .blocks
        .into_iter()
        .map(|b| TextBlock {
            text: b.text,
            confidence: b.confidence.clamp(0.0, 1.0),
            bbox: b.bbox,
            block_type: b.block_type,
        })
        .collect();

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
    block_type: BlockType,
}

fn encode_base64(input: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(input)
}
