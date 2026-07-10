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
    /// Template used for the optional refine pass.
    #[serde(default)]
    pub refine: String,
}

/// Optional values injected via environment variables for automated E2E tests.
#[cfg(not(test))]
#[derive(Deserialize, serde::Serialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub struct E2EArgs {
    pub source: Option<String>,
    pub output: Option<String>,
    pub checkpoint_dir: Option<String>,
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
    /// If true, run a second refinement pass over the first-pass translation.
    #[serde(default)]
    pub refine: bool,
    /// Directory where translation checkpoints are stored.
    #[serde(default)]
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

/// Arguments passed from the frontend to convert a scanned PDF to EPUB.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PdfToEpubArgs {
    /// Path to the source PDF file.
    pub pdf_path: String,
    /// Path where the output EPUB should be written.
    pub output_path: String,
    /// Optional title for the generated EPUB.
    pub title: Option<String>,
    /// API key for the OCR provider.
    pub ocr_api_key: String,
    /// Optional custom base URL for the OCR provider.
    pub ocr_base_url: Option<String>,
    /// Optional model name for the OCR provider.
    pub ocr_model: Option<String>,
    /// Number of pages to OCR concurrently.
    #[serde(default = "default_ocr_concurrency")]
    pub ocr_concurrency: usize,
    /// Optional API key for the verifier provider.
    pub verify_api_key: Option<String>,
    /// Optional base URL for the verifier provider.
    pub verify_base_url: Option<String>,
    /// Optional model name for the verifier provider.
    pub verify_model: Option<String>,
    /// If true, skip the LLM verification pass.
    #[serde(default)]
    pub no_verify: bool,
    /// Rendering resolution in DPI.
    #[serde(default = "default_dpi")]
    pub dpi: u32,
    /// Confidence threshold below which a block is verified.
    #[serde(default = "default_verify_threshold")]
    pub verify_threshold: f32,
    /// Maximum number of verify attempts for a low-confidence block.
    #[serde(default = "default_verify_max_attempts")]
    pub verify_max_attempts: usize,
    /// Scale factors for verify retry crops.
    #[serde(default = "default_verify_scale_factors")]
    pub verify_scale_factors: Vec<f32>,
}

const fn default_dpi() -> u32 {
    200
}

const fn default_verify_threshold() -> f32 {
    0.7
}

const fn default_verify_max_attempts() -> usize {
    3
}

fn default_verify_scale_factors() -> Vec<f32> {
    vec![1.0, 2.0, 3.0]
}

const fn default_ocr_concurrency() -> usize {
    3
}
