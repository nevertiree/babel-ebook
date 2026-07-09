//! Argument types passed from the Tauri frontend to the Rust backend.

use serde::{Deserialize, Serialize};

/// Configurable prompt templates passed from the frontend.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub struct PromptTemplates {
    pub default: String,
    pub literary: String,
    pub technical: String,
    pub academic: String,
}

/// Optional values injected via environment variables for automated E2E tests.
#[cfg(not(test))]
#[derive(Deserialize, serde::Serialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub struct E2EArgs {
    pub source: Option<String>,
    pub output: Option<String>,
    pub api_key: Option<String>,
    pub dry_run: Option<bool>,
    pub ui_language: Option<String>,
}

/// Arguments passed from the frontend to start a translation.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::struct_excessive_bools)]
pub struct TranslateArgs {
    pub source: String,
    pub output: String,
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub concurrency: u32,
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub temperature: f32,
    pub source_lang: String,
    pub target_lang: String,
    pub dry_run: bool,
    pub base_url: Option<String>,
    pub output_mode: String,
    pub style: String,
    pub preserve_classes: bool,
    pub exclude_selectors: Vec<String>,
    pub translate_attributes: Vec<String>,
    pub translate_body: bool,
    pub translate_metadata: bool,
    pub translate_toc: bool,
    pub translate_alt_text: bool,
    pub translate_image_captions: bool,
    pub translate_tables: bool,
    pub translate_footnotes: bool,
    pub translate_code: bool,
    pub output_font: Option<String>,
    /// Optional custom system prompt override. When provided, it replaces the
    /// style-based prompt entirely.
    pub system_prompt: Option<String>,
    /// Configurable prompt templates for each translation style.
    pub prompts: PromptTemplates,
    /// If true, refine an existing translation instead of translating from scratch.
    pub refine: bool,
    /// Directory where translation checkpoints are stored.
    pub checkpoint_dir: String,
    /// Optional job ID to resume a previously interrupted translation.
    pub resume: Option<String>,
}

/// Arguments for testing a provider connection without running a full translation.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct TestConnectionArgs {
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
}
