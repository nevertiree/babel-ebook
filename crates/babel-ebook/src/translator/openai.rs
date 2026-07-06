//! `OpenAI` / OpenAI-compatible translator provider.

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use async_trait::async_trait;

use crate::core::BabelEbookError;
use crate::translator::{TranslateContext, Translator};

const DEFAULT_MODEL: &str = "gpt-4o-mini";

/// Translator using the `OpenAI` API or an OpenAI-compatible endpoint.
pub struct OpenAiTranslator {
    client: Client<OpenAIConfig>,
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
            client: Client::with_config(config),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            max_tokens,
            temperature,
        }
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

        let system_message = ChatCompletionRequestSystemMessageArgs::default()
            .content(context.system_prompt)
            .build()
            .map_err(|e| BabelEbookError::ApiError(e.to_string()))?
            .into();
        let user_message = ChatCompletionRequestUserMessageArgs::default()
            .content(text)
            .build()
            .map_err(|e| BabelEbookError::ApiError(e.to_string()))?
            .into();

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(vec![system_message, user_message])
            .max_tokens(max_tokens)
            .temperature(self.temperature)
            .build()
            .map_err(|e| BabelEbookError::ApiError(e.to_string()))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| BabelEbookError::ApiError(e.to_string()))?;

        response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| BabelEbookError::ApiError("empty response from OpenAI".into()))
    }
}
