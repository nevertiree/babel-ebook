//! `OpenAI` / OpenAI-compatible translator provider.

use crate::core::BabelEbookError;
use crate::translator::http_common::{
    openai_compatible_health_check, openai_compatible_list_models, openai_compatible_translate,
};
use crate::translator::{TranslateContext, Translator};
use async_openai::config::OpenAIConfig;
use async_trait::async_trait;

const DEFAULT_MODEL: &str = "gpt-4o-mini";

/// Translator using the `OpenAI` API or an OpenAI-compatible endpoint.
pub struct OpenAiTranslator {
    client: async_openai::Client<OpenAIConfig>,
    model: String,
    max_tokens: usize,
    temperature: f32,
}

impl OpenAiTranslator {
    /// Create a new `OpenAI` translator.
    pub fn new(
        api_key: String,
        model: Option<String>,
        base_url: Option<String>,
        max_tokens: usize,
        temperature: f32,
    ) -> Self {
        let mut config = OpenAIConfig::default().with_api_key(api_key);
        if let Some(url) = base_url {
            config = config.with_api_base(url);
        }
        Self {
            client: async_openai::Client::with_config(config),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            max_tokens,
            temperature,
        }
    }

    fn config(&self) -> &OpenAIConfig {
        self.client.config()
    }
}

#[async_trait]
impl Translator for OpenAiTranslator {
    fn name(&self) -> String {
        format!("openai:{}", self.model)
    }

    fn max_output_tokens(&self) -> usize {
        self.max_tokens
    }

    async fn health_check(&self) -> Result<(), BabelEbookError> {
        openai_compatible_health_check(self.config(), "OpenAI").await
    }

    async fn list_models(&self) -> Result<Vec<String>, BabelEbookError> {
        openai_compatible_list_models(self.config(), "OpenAI").await
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

        openai_compatible_translate(
            &self.client,
            &self.model,
            context.system_prompt,
            text,
            max_tokens,
            self.temperature,
            "OpenAI",
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_uses_defaults() {
        let translator = OpenAiTranslator::new("fake-key".into(), None, None, 2000, 0.3);
        assert_eq!(translator.name(), "openai:gpt-4o-mini");
        assert_eq!(translator.max_output_tokens(), 2000);
    }

    #[tokio::test]
    async fn list_models_returns_api_error_for_unreachable_endpoint() {
        let translator = OpenAiTranslator::new(
            "fake-key".to_string(),
            None,
            Some("http://localhost:0".to_string()),
            2000,
            0.3,
        );
        let err = translator.list_models().await.unwrap_err();
        assert!(matches!(err, BabelEbookError::ApiError(_)));
    }

    #[tokio::test]
    async fn max_tokens_exceeds_u32_max_fails_fast() {
        let oversized: usize = u32::MAX as usize + 1;
        let translator = OpenAiTranslator::new("fake-key".into(), None, None, oversized, 0.3);
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
}
