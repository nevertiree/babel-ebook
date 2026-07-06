//! `Anthropic` translator provider using the Messages API directly.

use async_trait::async_trait;
use serde_json::json;

use crate::core::BabelEbookError;
use crate::translator::{TranslateContext, Translator};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const DEFAULT_MODEL: &str = "claude-3-5-sonnet-20241022";
const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";

/// Translator using the Anthropic Messages API.
pub struct AnthropicTranslator {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    max_tokens: usize,
    temperature: f32,
}

impl AnthropicTranslator {
    /// Create a new `Anthropic` translator.
    pub fn new(
        api_key: String,
        model: Option<String>,
        base_url: Option<String>,
        max_tokens: usize,
        temperature: f32,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            max_tokens,
            temperature,
        }
    }
}

#[async_trait]
impl Translator for AnthropicTranslator {
    fn name(&self) -> String {
        format!("anthropic:{}", self.model)
    }

    fn max_output_tokens(&self) -> usize {
        self.max_tokens
    }

    async fn health_check(&self) -> Result<(), BabelEbookError> {
        let response = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", DEFAULT_ANTHROPIC_VERSION)
            .send()
            .await
            .map_err(|e| BabelEbookError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format_http_error(status, &body));
        }
        Ok(())
    }

    async fn translate(
        &self,
        text: &str,
        context: &TranslateContext<'_>,
    ) -> Result<String, BabelEbookError> {
        let max_tokens = u32::try_from(self.max_tokens).map_err(|_| {
            BabelEbookError::Configuration(format!(
                "max_tokens {} exceeds u32::MAX",
                self.max_tokens
            ))
        })?;

        let body = json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "temperature": self.temperature,
            "system": context.system_prompt,
            "messages": [{"role": "user", "content": text}],
        });

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", DEFAULT_ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await
            .map_err(|e| BabelEbookError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format_http_error(status, &body));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BabelEbookError::ApiError(e.to_string()))?;
        parse_response(&json)
    }
}

fn format_http_error(status: reqwest::StatusCode, body: &str) -> BabelEbookError {
    let body = body.trim();
    if body.is_empty() {
        BabelEbookError::ApiError(format!("Anthropic HTTP error: {status}"))
    } else {
        BabelEbookError::ApiError(format!("Anthropic HTTP error {status}: {body}"))
    }
}

fn parse_response(json: &serde_json::Value) -> Result<String, BabelEbookError> {
    json["content"]
        .as_array()
        .and_then(|content| content.first())
        .and_then(|c| c["text"].as_str())
        .map(std::string::ToString::to_string)
        .ok_or_else(|| BabelEbookError::ApiError("empty response from Anthropic".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_uses_defaults() {
        let translator = AnthropicTranslator::new("fake-key".into(), None, None, 2000, 0.3);
        assert_eq!(translator.name(), "anthropic:claude-3-5-sonnet-20241022");
        assert_eq!(translator.max_output_tokens(), 2000);
    }

    #[tokio::test]
    async fn max_tokens_exceeds_u32_max_fails_fast() {
        let oversized: usize = u32::MAX as usize + 1;
        let translator = AnthropicTranslator::new("fake-key".into(), None, None, oversized, 0.3);
        let context = TranslateContext {
            system_prompt: "translate to {target_lang}",
            target_lang: "zh-CN",
        };
        let err = translator
            .translate("hello", &context)
            .await
            .expect_err("max_tokens > u32::MAX should fail immediately");

        assert!(
            matches!(err, BabelEbookError::Configuration(_)),
            "expected configuration error, got {err}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("max_tokens") && msg.contains("u32::MAX"),
            "error message should describe the configuration problem: {msg}"
        );
        assert!(
            !matches!(err, BabelEbookError::ApiError(_)),
            "configuration error must not be wrapped as an API failure"
        );
    }

    #[test]
    fn parse_response_extracts_text() {
        let json = json!({
            "content": [{"type": "text", "text": "Bonjour"}],
        });
        assert_eq!(parse_response(&json).unwrap(), "Bonjour");
    }

    #[test]
    fn parse_response_missing_content_returns_error() {
        let cases = [
            json!({}),
            json!({"content": []}),
            json!({"content": [{"type": "text"}]}),
            json!({"content": "unexpected string"}),
        ];
        for case in cases {
            let err = parse_response(&case).expect_err("missing text should fail");
            assert!(
                matches!(err, BabelEbookError::ApiError(_)),
                "expected API error, got {err}"
            );
            assert!(err.to_string().contains("empty response"));
        }
    }

    #[test]
    fn format_http_error_includes_status_and_body() {
        let err = format_http_error(reqwest::StatusCode::BAD_REQUEST, "invalid request");
        let msg = err.to_string();
        assert!(msg.contains("Anthropic HTTP error 400 Bad Request: invalid request"));
    }

    #[test]
    fn format_http_error_omits_body_when_empty() {
        let err = format_http_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "   ");
        let msg = err.to_string();
        assert!(msg.contains("Anthropic HTTP error: 500 Internal Server Error"));
        assert!(!msg.contains("Internal Server Error:"));
    }
}
