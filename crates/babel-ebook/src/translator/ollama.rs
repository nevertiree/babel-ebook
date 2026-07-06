//! `Ollama` local translator provider.

use async_trait::async_trait;
use serde_json::json;

use crate::core::BabelEbookError;
use crate::translator::{TranslateContext, Translator};

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
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        })
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
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| BabelEbookError::ApiError(format!("Ollama request failed: {e}")))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(BabelEbookError::ApiError(format!("Ollama error: {body}")));
        }
        Ok(())
    }

    async fn translate(
        &self,
        text: &str,
        context: &TranslateContext<'_>,
    ) -> Result<String, BabelEbookError> {
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": context.system_prompt},
                {"role": "user", "content": text},
            ],
            "stream": false,
        });

        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| BabelEbookError::ApiError(format!("Ollama request failed: {e}")))?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(BabelEbookError::ApiError(format!(
                "Ollama error: {err_text}"
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            BabelEbookError::ApiError(format!("failed to parse Ollama response: {e}"))
        })?;
        json["message"]
            .get("content")
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
            .ok_or_else(|| BabelEbookError::ApiError("empty response from Ollama".into()))
    }
}
