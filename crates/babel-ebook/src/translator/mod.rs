//! Translator trait and provider implementations.

mod anthropic;
mod deepseek;
mod ollama;
mod openai;
mod registry;

use crate::core::BabelEbookError;
use async_trait::async_trait;

/// Context passed to a translator for a single translation request.
#[derive(Debug, Clone, Copy)]
pub struct TranslateContext<'a> {
    /// The system prompt to use for this request.
    pub system_prompt: &'a str,
    /// The target language code (e.g. "zh-CN", "es").
    pub target_lang: &'a str,
}

/// An LLM-based translation provider.
#[async_trait]
pub trait Translator: Send + Sync {
    /// Provider short name, used for cache keys.
    fn name(&self) -> String;

    /// Maximum number of tokens to request in a single completion.
    fn max_output_tokens(&self) -> usize;

    /// Translate `text` according to the given context.
    async fn translate(
        &self,
        text: &str,
        context: &TranslateContext<'_>,
    ) -> Result<String, BabelEbookError>;

    /// Verify that the provider is reachable and credentials are valid.
    ///
    /// The default implementation succeeds unconditionally; providers should
    /// override it with an inexpensive API call.
    async fn health_check(&self) -> Result<(), BabelEbookError> {
        Ok(())
    }

    /// Return the list of model identifiers available from this provider.
    ///
    /// The default implementation returns an empty list; providers should
    /// override it to query their model API when one is available.
    async fn list_models(&self) -> Result<Vec<String>, BabelEbookError> {
        Ok(Vec::new())
    }
}

pub use registry::get_translator;
