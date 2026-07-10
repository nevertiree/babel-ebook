//! LLM-based verification and correction of OCR output.

use serde::{Deserialize, Serialize};

use crate::core::BabelEbookError;
use crate::pdf_ocr::backend::{BoundingBox, OcrPageResult};

/// An LLM backend that can verify extracted text against an image.
#[async_trait::async_trait]
pub trait VerifyBackend: Send + Sync {
    /// Ask the model to compare `text` against `image_bytes` and return a
    /// corrected version together with a confidence score.
    async fn verify(
        &self,
        image_bytes: &[u8],
        mime_type: &str,
        text: &str,
    ) -> Result<VerifiedText, BabelEbookError>;
}

/// Result of verifying a text block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedText {
    /// Corrected text.
    pub text: String,
    /// Confidence that the text matches the image, 0.0–1.0.
    pub confidence: f32,
    /// True if the model changed the input text.
    pub changed: bool,
}

/// Configuration for the OpenAI-compatible verification backend.
#[derive(Debug, Clone)]
pub struct OpenAiVerifyConfig {
    /// API key.
    pub api_key: String,
    /// Base URL, e.g. `https://api.openai.com/v1`.
    pub base_url: String,
    /// Model name.
    pub model: String,
}

/// Verifier using any OpenAI-compatible chat completions endpoint.
pub struct OpenAiVerifyBackend {
    client: reqwest::Client,
    config: OpenAiVerifyConfig,
}

impl OpenAiVerifyBackend {
    /// Create a new verifier from config.
    #[must_use]
    pub fn new(config: OpenAiVerifyConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }
}

#[async_trait::async_trait]
impl VerifyBackend for OpenAiVerifyBackend {
    async fn verify(
        &self,
        image_bytes: &[u8],
        mime_type: &str,
        text: &str,
    ) -> Result<VerifiedText, BabelEbookError> {
        let base64_image = encode_base64(image_bytes);
        let data_url = format!("data:{mime_type};base64,{base64_image}");

        let system_message = serde_json::json!({
            "role": "system",
            "content": "You are an OCR verifier. Compare the provided image region with the extracted text. Return only a JSON object with keys: 'text' (corrected text), 'confidence' (0.0-1.0), and 'changed' (boolean). Preserve line breaks and formatting. If the extracted text is correct, return it unchanged with changed=false."
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
                    "text": format!("Extracted text to verify:\n---\n{text}\n---\nReturn the corrected text as JSON.")
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

        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| BabelEbookError::ApiError(format!("verify request failed: {e}")))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| BabelEbookError::ApiError(format!("failed to read verify response: {e}")))?;

        if !status.is_success() {
            return Err(BabelEbookError::ApiError(format!(
                "verify returned {status}: {response_text}"
            )));
        }

        let chat_response: ChatCompletionResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                BabelEbookError::ApiError(format!(
                    "failed to parse verify response: {e}. Body: {response_text}"
                ))
            })?;

        let content = chat_response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| BabelEbookError::ApiError("empty verify response".into()))?;

        let verified: VerifiedText = serde_json::from_str(&content).map_err(|e| {
            BabelEbookError::ApiError(format!(
                "failed to parse verify JSON: {e}. Content: {content}"
            ))
        })?;

        Ok(VerifiedText {
            confidence: verified.confidence.clamp(0.0, 1.0),
            ..verified
        })
    }
}

/// Verify an entire page.
///
/// Blocks whose confidence is below `threshold` are sent to the verifier.
/// If a block has a bounding box, the region is cropped and the cropped image
/// is sent; otherwise the full page image is used.
pub async fn verify_page(
    backend: &dyn VerifyBackend,
    full_image: &[u8],
    mime_type: &str,
    page: &mut OcrPageResult,
    threshold: f32,
) -> Result<(), BabelEbookError> {
    for block in &mut page.blocks {
        if block.confidence >= threshold {
            continue;
        }

        let image_to_verify = block
            .bbox
            .map_or_else(|| full_image.to_vec(), |bbox| crop_image(full_image, bbox).unwrap_or_else(|_| full_image.to_vec()));

        let verified = backend
            .verify(&image_to_verify, mime_type, &block.text)
            .await?;

        if verified.changed {
            block.text = verified.text;
        }
        block.confidence = verified.confidence.max(block.confidence);
    }

    page.full_text = page
        .blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(())
}

fn crop_image(image_bytes: &[u8], bbox: BoundingBox) -> Result<Vec<u8>, BabelEbookError> {
    let img = image::load_from_memory(image_bytes).map_err(|e| {
        BabelEbookError::Anyhow(anyhow::anyhow!("failed to decode image for cropping: {e}"))
    })?;

    let cropped = img.crop_imm(bbox.x, bbox.y, bbox.w, bbox.h);
    let mut out = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("failed to encode cropped image: {e}")))?;
    Ok(out.into_inner())
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

fn encode_base64(input: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(input)
}
