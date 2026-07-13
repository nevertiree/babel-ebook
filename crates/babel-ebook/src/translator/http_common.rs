//! Shared HTTP helpers for translator providers.
//!
//! This module centralises the duplicated pieces of the provider translators:
//! reqwest client construction, timeout values, retry logic with exponential
//! backoff, JSON response parsing, and HTTP error formatting. Provider-specific
//! files keep only their base URLs, request body shapes, and response extraction
//! paths.

use crate::core::BabelEbookError;
use async_openai::config::{Config, OpenAIConfig};
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, CreateChatCompletionRequest,
};
use async_openai::Client;
use reqwest::StatusCode;
use serde_json::Value;
use std::time::Duration;

/// Timeout for a single translation request.
pub const TRANSLATE_TIMEOUT: Duration = Duration::from_secs(300);
/// Timeout for inexpensive metadata calls such as health checks and model lists.
pub const META_TIMEOUT: Duration = Duration::from_secs(10);
/// Number of retries applied consistently across all HTTP providers.
pub const MAX_RETRIES: u32 = 3;

/// Build the shared `reqwest::Client` used for provider metadata calls.
pub fn build_reqwest_client() -> reqwest::Client {
    reqwest::Client::new()
}

/// Format an HTTP error response in a consistent way across providers.
pub fn format_http_error(provider: &str, status: StatusCode, body: &str) -> BabelEbookError {
    let body = body.trim();
    if body.is_empty() {
        BabelEbookError::ApiError(format!("{provider} HTTP error: {status}"))
    } else {
        BabelEbookError::ApiError(format!("{provider} HTTP error {status}: {body}"))
    }
}

/// Execute an async operation, retrying on failure with exponential backoff.
///
/// The `operation` closure is called fresh for each attempt so that request
/// bodies and futures can be reconstructed. After `MAX_RETRIES` failed attempts
/// the last error is returned wrapped in a provider-scoped message.
pub async fn with_retry<F, Fut, T>(
    provider: &str,
    operation_name: &str,
    operation: F,
) -> Result<T, BabelEbookError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, BabelEbookError>>,
{
    let mut last_error = None;

    for attempt in 0..=MAX_RETRIES {
        match operation().await {
            Ok(value) => return Ok(value),
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
        "{provider} {operation_name} failed after {MAX_RETRIES} retries: {last_error}"
    )))
}

/// Extract a list of model identifiers from a JSON array.
///
/// `array_field` names the top-level field holding the array (e.g. `"data"` or
/// `"models"`) and `id_field` names the field containing the model id (e.g.
/// `"id"` or `"name"`).
pub fn parse_model_list(json: &Value, array_field: &str, id_field: &str) -> Vec<String> {
    json[array_field]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m[id_field].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Translate a chunk using an OpenAI-compatible chat completion endpoint.
///
/// This helper builds the standard system/user message request, applies the
/// translate timeout, retries on failure, and extracts the first choice content.
pub async fn openai_compatible_translate(
    client: &Client<OpenAIConfig>,
    model: &str,
    system_prompt: &str,
    text: &str,
    max_tokens: u32,
    temperature: f32,
    provider_name: &str,
) -> Result<String, BabelEbookError> {
    let request = CreateChatCompletionRequest {
        model: model.to_string(),
        messages: vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: ChatCompletionRequestSystemMessageContent::Text(system_prompt.to_string()),
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(text.to_string()),
                name: None,
            }),
        ],
        max_tokens: Some(max_tokens),
        temperature: Some(temperature),
        ..Default::default()
    };

    with_retry(provider_name, "API", || {
        let request = request.clone();
        async move {
            let response = tokio::time::timeout(TRANSLATE_TIMEOUT, client.chat().create(request))
                .await
                .map_err(|_| {
                    BabelEbookError::ApiError(format!("{provider_name} request timed out"))
                })?
                .map_err(|e| BabelEbookError::ApiError(e.to_string()))?;

            let content = response
                .choices
                .into_iter()
                .next()
                .and_then(|choice| choice.message.content)
                .unwrap_or_default()
                .trim()
                .to_string();

            if content.is_empty() {
                return Err(BabelEbookError::ApiError(format!(
                    "{provider_name} API returned empty content"
                )));
            }

            Ok(content)
        }
    })
    .await
}

/// Perform a lightweight health check against an OpenAI-compatible `/models`
/// endpoint.
pub async fn openai_compatible_health_check(
    config: &OpenAIConfig,
    provider_name: &str,
) -> Result<(), BabelEbookError> {
    let client = build_reqwest_client();
    let url = config.url("/models");
    let response = client
        .get(&url)
        .headers(config.headers())
        .timeout(META_TIMEOUT)
        .send()
        .await
        .map_err(|e| BabelEbookError::ApiError(e.to_string()))?;

    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        let body = response.text().await.unwrap_or_default();
        Err(format_http_error(provider_name, status, &body))
    }
}

/// List models from an OpenAI-compatible `/models` endpoint.
pub async fn openai_compatible_list_models(
    config: &OpenAIConfig,
    provider_name: &str,
) -> Result<Vec<String>, BabelEbookError> {
    let client = build_reqwest_client();
    let url = config.url("/models");
    let response = client
        .get(&url)
        .headers(config.headers())
        .timeout(META_TIMEOUT)
        .send()
        .await
        .map_err(|e| BabelEbookError::ApiError(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format_http_error(provider_name, status, &body));
    }

    let json: Value = response.json().await.map_err(|e| {
        BabelEbookError::ApiError(format!("failed to parse {provider_name} models: {e}"))
    })?;

    Ok(parse_model_list(&json, "data", "id"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_http_error_includes_body() {
        let err = format_http_error("TestProvider", StatusCode::BAD_REQUEST, "bad request");
        let msg = err.to_string();
        assert!(msg.contains("TestProvider HTTP error 400 Bad Request: bad request"));
    }

    #[test]
    fn format_http_error_omits_body_when_empty() {
        let err = format_http_error("TestProvider", StatusCode::INTERNAL_SERVER_ERROR, "   ");
        let msg = err.to_string();
        assert!(msg.contains("TestProvider HTTP error: 500 Internal Server Error"));
        assert!(!msg.contains("Internal Server Error:"));
    }

    #[test]
    fn parse_model_list_extracts_ids() {
        let json = serde_json::json!({
            "data": [
                {"id": "model-a"},
                {"id": "model-b"},
            ]
        });
        assert_eq!(
            parse_model_list(&json, "data", "id"),
            vec!["model-a", "model-b"]
        );
    }

    #[test]
    fn parse_model_list_uses_custom_fields() {
        let json = serde_json::json!({
            "models": [
                {"name": "llama3"},
                {"name": "qwen2"},
            ]
        });
        assert_eq!(
            parse_model_list(&json, "models", "name"),
            vec!["llama3", "qwen2"]
        );
    }

    #[test]
    fn parse_model_list_returns_empty_on_bad_shape() {
        let cases = [
            serde_json::json!({}),
            serde_json::json!({"data": "not-an-array"}),
            serde_json::json!({"data": [{"name": "missing-id"}]}),
        ];
        for case in cases {
            assert!(parse_model_list(&case, "data", "id").is_empty());
        }
    }

    #[tokio::test]
    async fn with_retry_succeeds_without_retries() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let result = with_retry("Provider", "op", {
            let counter = counter.clone();
            move || {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok::<_, BabelEbookError>("done")
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), "done");
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn with_retry_retries_then_succeeds() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let result = with_retry("Provider", "op", {
            let counter = counter.clone();
            move || {
                let counter = counter.clone();
                async move {
                    let attempt = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if attempt < 2 {
                        Err(BabelEbookError::ApiError("transient".into()))
                    } else {
                        Ok::<_, BabelEbookError>("recovered")
                    }
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), "recovered");
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn with_retry_gives_up_after_max_retries() {
        let result = with_retry("Provider", "op", || async {
            Err::<(), _>(BabelEbookError::ApiError("always fails".into()))
        })
        .await;
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Provider op failed after 3 retries"));
        assert!(msg.contains("always fails"));
    }
}
