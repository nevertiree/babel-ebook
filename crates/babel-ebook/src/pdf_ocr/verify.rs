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
            "content": "You are an OCR verifier. Compare the provided image region with the extracted text. Return only a valid JSON object with keys: 'text' (corrected text), 'confidence' (0.0-1.0), and 'changed' (boolean). Preserve line breaks and formatting. If the extracted text is correct, return it unchanged with changed=false. Do not include comments or explanatory text in the JSON."
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
            .map_err(|e| BabelEbookError::ApiError(format!("verify request failed: {e}")))?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
            BabelEbookError::ApiError(format!("failed to read verify response: {e}"))
        })?;

        if !status.is_success() {
            return Err(BabelEbookError::ApiError(format!(
                "verify returned {status}: {response_text}"
            )));
        }

        let chat_response: ChatCompletionResponse =
            serde_json::from_str(&response_text).map_err(|e| {
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

        let cleaned = super::strip_json_comments(&content);
        let verified: VerifiedText = serde_json::from_str(&cleaned).map_err(|e| {
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

/// Verify an entire page using an adaptive retry loop.
///
/// Blocks whose confidence is below `threshold`, or whose text looks anomalous,
/// are sent to the verifier. For each such block, the verifier is called up to
/// `max_attempts` times with progressively larger cropped regions (controlled by
/// `scale_factors`). The best result is kept.
pub async fn verify_page_with_retry(
    backend: &dyn VerifyBackend,
    full_image: &[u8],
    mime_type: &str,
    page: &mut OcrPageResult,
    threshold: f32,
    max_attempts: usize,
    scale_factors: &[f32],
) -> Result<(), BabelEbookError> {
    for block in &mut page.blocks {
        let anomaly = text_anomaly_score(&block.text);
        let needs_verify = block.confidence < threshold || anomaly > 0.3;
        if !needs_verify {
            continue;
        }

        let mut best_text = block.text.clone();
        let mut best_confidence = block.confidence;
        let mut best_anomaly = anomaly;

        let factors: Vec<f32> = scale_factors.iter().copied().take(max_attempts).collect();
        if factors.is_empty() {
            continue;
        }

        for scale in &factors {
            let image_to_verify = block.bbox.map_or_else(
                || full_image.to_vec(),
                |bbox| {
                    crop_and_scale_image(full_image, bbox, *scale)
                        .unwrap_or_else(|_| full_image.to_vec())
                },
            );

            let verified = match backend
                .verify(&image_to_verify, mime_type, &block.text)
                .await
            {
                Ok(v) => v,
                Err(err) => {
                    tracing::warn!(
                        block_type = ?block.block_type,
                        scale = %scale,
                        error = %err,
                        "verify attempt failed"
                    );
                    continue;
                }
            };

            let new_anomaly = text_anomaly_score(&verified.text);
            let is_better = verified.confidence > best_confidence
                || (verified.confidence >= best_confidence && new_anomaly < best_anomaly);

            if is_better {
                best_text = verified.text;
                best_confidence = verified.confidence;
                best_anomaly = new_anomaly;
            }

            // Early stop if the result is good enough and looks normal.
            if best_confidence >= threshold && best_anomaly <= 0.2 {
                break;
            }
        }

        block.text = best_text;
        block.confidence = best_confidence.max(block.confidence);
    }

    page.full_text = page
        .blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(())
}

/// Compute a quick heuristic anomaly score for OCR text.
///
/// Returns a value between 0.0 and 1.0. Higher means more likely to be garbled.
/// This is intentionally cheap and local; it catches obvious garbage without
/// spending extra LLM tokens.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn text_anomaly_score(text: &str) -> f32 {
    if text.is_empty() {
        return 0.0;
    }

    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    if total == 0 {
        return 0.0;
    }

    // Count suspicious characters: replacement chars, control chars, excessive punctuation.
    let suspicious: f32 = chars
        .iter()
        .filter(|c| {
            matches!(c, '\u{FFFD}' | '〓' | '▯' | '□' | '■' | '●' | '▪' | '▫')
                || c.is_control() && !c.is_whitespace()
        })
        .count() as f32;

    // Repeated single character 4+ times (e.g. "????", "aaaa").
    let mut repeated = 0usize;
    let mut run = 1usize;
    for pair in chars.windows(2) {
        if pair[0] == pair[1] {
            run += 1;
        } else {
            if run >= 4 {
                repeated += run;
            }
            run = 1;
        }
    }
    if run >= 4 {
        repeated += run;
    }

    // Excessive punctuation / symbols relative to text.
    let symbol_count = chars
        .iter()
        .filter(|c| {
            c.is_ascii_punctuation()
                || matches!(
                    c,
                    '。' | '，'
                        | '、'
                        | '；'
                        | '：'
                        | '？'
                        | '！'
                        | '“'
                        | '”'
                        | '‘'
                        | '’'
                        | '（'
                        | '）'
                        | '《'
                        | '》'
                        | '—'
                        | '…'
                )
        })
        .count();
    let symbol_ratio = symbol_count as f32 / total as f32;

    let suspicious_ratio = suspicious / total as f32;
    let repeated_ratio = repeated as f32 / total as f32;

    (repeated_ratio.mul_add(0.8, suspicious_ratio * 1.5) + symbol_ratio.clamp(0.0, 0.5))
        .clamp(0.0, 1.0)
}

/// Crop a region from `image_bytes` according to `bbox` and scale the crop
/// relative to the original bounding box.
///
/// A scale of 1.0 returns the exact bounding box. Larger scales expand the
/// region equally in all directions, clamped to the image edges, and then
/// resize the result to the scaled dimensions. This gives the verifier a
/// magnified view of the text block while preserving surrounding context.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn crop_and_scale_image(
    image_bytes: &[u8],
    bbox: BoundingBox,
    scale: f32,
) -> Result<Vec<u8>, BabelEbookError> {
    let img = image::load_from_memory(image_bytes).map_err(|e| {
        BabelEbookError::Anyhow(anyhow::anyhow!("failed to decode image for cropping: {e}"))
    })?;

    let (img_width, img_height) = (img.width(), img.height());

    let new_w = (bbox.w as f32 * scale).round().max(1.0) as u32;
    let new_h = (bbox.h as f32 * scale).round().max(1.0) as u32;
    let x = bbox
        .x
        .saturating_sub((new_w - bbox.w) / 2)
        .min(img_width - 1);
    let y = bbox
        .y
        .saturating_sub((new_h - bbox.h) / 2)
        .min(img_height - 1);
    let w = new_w.min(img_width - x);
    let h = new_h.min(img_height - y);

    let cropped = img.crop_imm(x, y, w, h);
    let mut out = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| {
            BabelEbookError::Anyhow(anyhow::anyhow!("failed to encode cropped image: {e}"))
        })?;
    Ok(out.into_inner())
}

/// Legacy single-pass page verifier kept for callers that do not need retries.
#[allow(dead_code)]
pub async fn verify_page(
    backend: &dyn VerifyBackend,
    full_image: &[u8],
    mime_type: &str,
    page: &mut OcrPageResult,
    threshold: f32,
) -> Result<(), BabelEbookError> {
    verify_page_with_retry(backend, full_image, mime_type, page, threshold, 1, &[1.0]).await
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
