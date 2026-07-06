//! `DeepSeek` / OpenAI-compatible translator provider.

use crate::core::BabelEbookError;
use crate::translator::{TranslateContext, Translator};
use anyhow::anyhow;
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, CreateChatCompletionRequest,
};
use async_openai::Client;
use async_trait::async_trait;
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";
const DEFAULT_MODEL: &str = "deepseek-chat";
const MAX_RETRIES: u32 = 3;

/// Translator using the `DeepSeek` API.
pub struct DeepSeekTranslator {
    client: Client<OpenAIConfig>,
    model: String,
    max_tokens: usize,
    temperature: f32,
}

impl DeepSeekTranslator {
    /// Create a new `DeepSeek` translator.
    pub fn new(
        api_key: String,
        model: Option<String>,
        base_url: Option<String>,
        max_tokens: usize,
        temperature: f32,
    ) -> Self {
        let model = model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let base_url = base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);
        Self {
            client: Client::with_config(config),
            model,
            max_tokens,
            temperature,
        }
    }

    async fn try_translate(
        &self,
        text: &str,
        system_prompt: &str,
        max_tokens: u32,
    ) -> Result<String, BabelEbookError> {
        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![
                ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                    content: ChatCompletionRequestSystemMessageContent::Text(
                        system_prompt.to_string(),
                    ),
                    name: None,
                }),
                ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                    content: ChatCompletionRequestUserMessageContent::Text(text.to_string()),
                    name: None,
                }),
            ],
            max_tokens: Some(max_tokens),
            temperature: Some(self.temperature),
            ..Default::default()
        };

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| BabelEbookError::Anyhow(e.into()))?;

        let content = response
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .unwrap_or_default()
            .trim()
            .to_string();

        if content.is_empty() {
            return Err(BabelEbookError::Anyhow(anyhow!(
                "DeepSeek API returned empty content"
            )));
        }

        Ok(content)
    }
}

#[async_trait]
impl Translator for DeepSeekTranslator {
    fn name(&self) -> String {
        format!("deepseek:{}", self.model)
    }

    fn max_output_tokens(&self) -> usize {
        self.max_tokens
    }

    async fn health_check(&self) -> Result<(), BabelEbookError> {
        self.client
            .models()
            .list()
            .await
            .map(|_| ())
            .map_err(|e| BabelEbookError::ApiError(e.to_string()))
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

        let mut last_error = None;

        for attempt in 0..=MAX_RETRIES {
            match self
                .try_translate(text, context.system_prompt, max_tokens)
                .await
            {
                Ok(translation) => return Ok(translation),
                Err(e) => {
                    last_error = Some(e);
                    if attempt == MAX_RETRIES {
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                }
            }
        }

        let last_error = last_error.expect("loop always assigns an error before exiting");
        Err(BabelEbookError::ApiError(format!(
            "DeepSeek API failed after {MAX_RETRIES} retries: {last_error}"
        )))
    }
}
