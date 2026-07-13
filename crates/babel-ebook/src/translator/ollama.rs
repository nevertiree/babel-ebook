//! `Ollama` local translator provider.

use crate::core::BabelEbookError;
use crate::translator::http_common::{
    build_reqwest_client, format_http_error, parse_model_list, with_retry, META_TIMEOUT,
    TRANSLATE_TIMEOUT,
};
use crate::translator::{TranslateContext, Translator};
use async_trait::async_trait;
use serde_json::{json, Value};

const DEFAULT_BASE_URL: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "llama3";

/// Translator using a local `Ollama` instance.
pub struct OllamaTranslator {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaTranslator {
    /// Create a new `Ollama` translator.
    #[allow(clippy::unnecessary_wraps)]
    pub fn new(
        _api_key: String,
        model: Option<String>,
        base_url: Option<String>,
    ) -> Result<Self, BabelEbookError> {
        Ok(Self {
            client: build_reqwest_client(),
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        })
    }

    fn tags_url(&self) -> String {
        format!("{}/api/tags", self.base_url)
    }

    fn chat_url(&self) -> String {
        format!("{}/api/chat", self.base_url)
    }
}

#[async_trait]
impl Translator for OllamaTranslator {
    fn name(&self) -> String {
        format!("ollama:{}", self.model)
    }

    fn max_output_tokens(&self) -> usize {
        0 // Ollama does not use this parameter in the same way
    }

    async fn health_check(&self) -> Result<(), BabelEbookError> {
        let response = self
            .client
            .get(self.tags_url())
            .timeout(META_TIMEOUT)
            .send()
            .await
            .map_err(|e| BabelEbookError::ApiError(format!("Ollama request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format_http_error("Ollama", status, &body));
        }
        Ok(())
    }

    async fn list_models(&self) -> Result<Vec<String>, BabelEbookError> {
        let response = self
            .client
            .get(self.tags_url())
            .timeout(META_TIMEOUT)
            .send()
            .await
            .map_err(|e| BabelEbookError::ApiError(format!("Ollama list models failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format_http_error("Ollama", status, &body));
        }

        let json: Value = response.json().await.map_err(|e| {
            BabelEbookError::ApiError(format!("failed to parse Ollama models: {e}"))
        })?;

        Ok(parse_model_list(&json, "models", "name"))
    }

    async fn translate(
        &self,
        text: &str,
        context: &TranslateContext<'_>,
    ) -> Result<String, BabelEbookError> {
        with_retry("Ollama", "API", || {
            let body = json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": context.system_prompt},
                    {"role": "user", "content": text},
                ],
                "stream": false,
            });
            let request = self
                .client
                .post(self.chat_url())
                .json(&body)
                .timeout(TRANSLATE_TIMEOUT);

            async move {
                let response = request.send().await.map_err(|e| {
                    BabelEbookError::ApiError(format!("Ollama request failed: {e}"))
                })?;

                let status = response.status();
                if !status.is_success() {
                    let err_text = response.text().await.unwrap_or_default();
                    return Err(format_http_error("Ollama", status, &err_text));
                }

                let json: Value = response.json().await.map_err(|e| {
                    BabelEbookError::ApiError(format!("failed to parse Ollama response: {e}"))
                })?;
                parse_ollama_response(&json)
            }
        })
        .await
    }
}

fn parse_ollama_response(json: &Value) -> Result<String, BabelEbookError> {
    json["message"]
        .get("content")
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string)
        .ok_or_else(|| BabelEbookError::ApiError("empty response from Ollama".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_uses_defaults() {
        let translator = OllamaTranslator::new(String::new(), None, None).unwrap();
        assert_eq!(translator.name(), "ollama:llama3");
        assert_eq!(translator.max_output_tokens(), 0);
    }

    #[test]
    fn list_models_parses_local_models() {
        let json = serde_json::json!({
            "models": [
                {"name": "llama3.2:latest"},
                {"name": "qwen2:latest"},
            ]
        });
        assert_eq!(
            parse_model_list(&json, "models", "name"),
            vec!["llama3.2:latest", "qwen2:latest"]
        );
    }

    #[test]
    fn parse_response_extracts_content() {
        let json = json!({"message": {"role": "assistant", "content": "Hola"}});
        assert_eq!(parse_ollama_response(&json).unwrap(), "Hola");
    }

    #[test]
    fn parse_response_missing_content_returns_error() {
        let cases = [
            json!({}),
            json!({"message": {}}),
            json!({"message": {"content": 123}}),
        ];
        for case in cases {
            let err = parse_ollama_response(&case).expect_err("missing content should fail");
            assert!(matches!(err, BabelEbookError::ApiError(_)));
            assert!(err.to_string().contains("empty response"));
        }
    }

    #[tokio::test]
    async fn list_models_returns_api_error_for_unreachable_endpoint() {
        let translator =
            OllamaTranslator::new(String::new(), None, Some("http://localhost:0".to_string()))
                .unwrap();
        let err = translator.list_models().await.unwrap_err();
        assert!(matches!(err, BabelEbookError::ApiError(_)));
    }
}
