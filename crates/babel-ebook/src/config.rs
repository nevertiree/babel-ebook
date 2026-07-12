//! Runtime configuration for the translator.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::core::BabelEbookError;

/// How the translated EPUB should present source and target text.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// Side-by-side or interleaved bilingual output.
    #[default]
    Bilingual,
    /// Only the translated text is included.
    TranslationOnly,
    /// Original and translation paragraphs alternate.
    Interleaved,
}

impl OutputMode {
    /// Return a stable, snake-case string representation for use in hashes and
    /// persistence. This does not rely on the `Debug` derive, so renaming the
    /// enum variants will not break existing checkpoints.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bilingual => "bilingual",
            Self::TranslationOnly => "translation_only",
            Self::Interleaved => "interleaved",
        }
    }
}

/// Which parts of an EPUB document should be translated.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct TranslationScope {
    /// Translate body text.
    #[serde(default = "true_bool")]
    pub body: bool,
    /// Translate metadata such as titles and descriptions.
    #[serde(default = "true_bool")]
    pub metadata: bool,
    /// Translate the table of contents.
    #[serde(default = "true_bool")]
    pub toc: bool,
    /// Translate alternative text for images.
    #[serde(default = "true_bool")]
    pub alt_text: bool,
    /// Translate figure captions.
    #[serde(default = "true_bool")]
    pub image_captions: bool,
    /// Translate table cell contents.
    #[serde(default = "true_bool")]
    pub tables: bool,
    /// Translate footnotes and endnotes.
    #[serde(default = "true_bool")]
    pub footnotes: bool,
}

impl Default for TranslationScope {
    fn default() -> Self {
        Self {
            body: true,
            metadata: true,
            toc: true,
            alt_text: true,
            image_captions: true,
            tables: true,
            footnotes: true,
        }
    }
}

const fn true_bool() -> bool {
    true
}

/// Desired style or register for the translation output.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationStyle {
    /// No specific style; use the model's default.
    #[default]
    Default,
    /// Literary, flowing prose suitable for fiction.
    Literary,
    /// Technical, precise language suitable for manuals or specs.
    Technical,
    /// Academic, formal language suitable for papers.
    Academic,
    /// A user-defined style description.
    Custom(String),
}

/// A single glossary entry mapping a term to its preferred translation.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GlossaryEntry {
    /// Source term in the original language.
    pub term: String,
    /// Preferred translation in the target language.
    pub translation: String,
    /// Optional context or note for when to use this translation.
    pub context: Option<String>,
}

/// Runtime configuration for the babel-ebook translation pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Config {
    /// Path to the source EPUB file.
    pub source: PathBuf,
    /// Path to the output EPUB file.
    pub output: PathBuf,
    /// Translation provider short name (e.g. `deepseek`).
    #[serde(default = "default_provider")]
    pub provider: String,
    /// API key for the selected provider.
    pub api_key: Option<String>,
    /// Custom base URL for the provider's API.
    pub base_url: Option<String>,
    /// Model name to use with the provider.
    #[serde(default = "default_model")]
    pub model: String,
    /// Maximum number of concurrent translation requests.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Maximum input tokens per API request.
    #[serde(default = "default_max_input_tokens")]
    pub max_input_tokens: usize,
    /// Maximum output tokens per API request.
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: usize,
    /// Directory where translation cache entries are stored.
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    /// Directory where translation checkpoint files are stored.
    #[serde(default = "default_checkpoint_dir")]
    pub checkpoint_dir: PathBuf,
    /// Optional job id to resume an existing translation.
    #[serde(default)]
    pub resume_job_id: Option<String>,
    /// Sampling temperature for the LLM.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Source language for the translation.
    #[serde(default = "default_source_lang")]
    pub source_lang: String,
    /// Target language for the translation.
    #[serde(default = "default_target_lang")]
    pub target_lang: String,
    /// Document name patterns that should not be translated.
    #[serde(default = "default_skip_doc_patterns")]
    pub skip_doc_patterns: Vec<String>,
    /// HTML tag names whose text content should be translated.
    #[serde(default = "default_translate_tags")]
    pub translate_tags: Vec<String>,
    /// Optional custom system prompt; the default prompt is used if `None`.
    pub system_prompt: Option<String>,
    /// If `true`, only estimate token usage without calling the API.
    #[serde(default)]
    pub dry_run: bool,
    /// If `true`, enable verbose logging.
    #[serde(default)]
    pub verbose: bool,
    /// Optional provider-specific configuration.
    #[serde(default)]
    #[allow(clippy::struct_field_names)]
    pub provider_config: Option<ProviderConfig>,
    /// Preset provider configurations keyed by provider short name.
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    /// How the translated EPUB should present source and target text.
    #[serde(default)]
    pub output_mode: OutputMode,
    /// Which parts of an EPUB document should be translated.
    #[serde(default)]
    pub translation_scope: TranslationScope,
    /// Desired style or register for the translation output.
    #[serde(default)]
    pub style: TranslationStyle,
    /// Per-chapter custom system prompts keyed by chapter href.
    #[serde(default)]
    pub chapter_prompts: HashMap<String, String>,
    /// Configurable prompt templates for each translation style.
    #[serde(default)]
    pub prompts: PromptTemplates,
    /// Glossary of terms with preferred translations.
    #[serde(default)]
    pub glossary: Vec<GlossaryEntry>,
    /// CSS selectors matching elements whose text should not be translated.
    #[serde(default)]
    pub exclude_selectors: Vec<String>,
    /// HTML attributes whose values should be translated.
    #[serde(default)]
    pub translate_attributes: Vec<String>,
    /// Whether to preserve original CSS classes when rewriting HTML.
    #[serde(default)]
    pub preserve_classes: bool,
    /// Optional font-family CSS injected into every translated XHTML document.
    #[serde(default)]
    pub output_font: Option<String>,
    /// If `true`, refine an existing translation instead of translating from scratch.
    #[serde(default)]
    pub refine: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source: PathBuf::new(),
            output: PathBuf::new(),
            provider: default_provider(),
            api_key: None,
            base_url: None,
            model: default_model(),
            concurrency: default_concurrency(),
            max_input_tokens: default_max_input_tokens(),
            max_output_tokens: default_max_output_tokens(),
            cache_dir: default_cache_dir(),
            checkpoint_dir: default_checkpoint_dir(),
            resume_job_id: None,
            temperature: default_temperature(),
            source_lang: default_source_lang(),
            target_lang: default_target_lang(),
            skip_doc_patterns: default_skip_doc_patterns(),
            translate_tags: default_translate_tags(),
            system_prompt: None,
            dry_run: false,
            verbose: false,
            provider_config: None,
            providers: HashMap::default(),
            output_mode: OutputMode::default(),
            translation_scope: TranslationScope::default(),
            style: TranslationStyle::default(),
            chapter_prompts: HashMap::default(),
            prompts: PromptTemplates::default(),
            glossary: Vec::default(),
            exclude_selectors: Vec::default(),
            translate_attributes: Vec::default(),
            preserve_classes: false,
            output_font: None,
            refine: false,
        }
    }
}

/// Translation-only options extracted from [`Config`].
///
/// Grouping these fields makes it easier to pass the subset of configuration
/// that HTML translation needs without dragging the full `Config` object
/// (paths, API keys, provider settings, etc.) through the document pipeline.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TranslationOptions {
    /// Source language for the translation.
    pub source_lang: String,
    /// Target language for the translation.
    pub target_lang: String,
    /// How the translated EPUB should present source and target text.
    pub output_mode: OutputMode,
    /// Which parts of an EPUB document should be translated.
    pub translation_scope: TranslationScope,
    /// Desired style or register for the translation output.
    pub style: TranslationStyle,
    /// Optional custom system prompt; the default prompt is used if `None`.
    pub system_prompt: Option<String>,
    /// Configurable prompt templates for each translation style.
    pub prompts: PromptTemplates,
    /// Glossary of terms with preferred translations.
    pub glossary: Vec<GlossaryEntry>,
    /// CSS selectors matching elements whose text should not be translated.
    pub exclude_selectors: Vec<String>,
    /// HTML attributes whose values should be translated.
    pub translate_attributes: Vec<String>,
    /// HTML tag names whose text content should be translated.
    pub translate_tags: Vec<String>,
    /// Whether to preserve original CSS classes when rewriting HTML.
    pub preserve_classes: bool,
    /// Optional font-family CSS injected into every translated XHTML document.
    pub output_font: Option<String>,
    /// If `true`, refine an existing translation instead of translating from scratch.
    pub refine: bool,
    /// Maximum input tokens per API request.
    pub max_input_tokens: usize,
    /// Maximum output tokens per API request.
    pub max_output_tokens: usize,
    /// Sampling temperature for the LLM.
    pub temperature: f32,
    /// Per-chapter custom system prompts keyed by chapter href.
    pub chapter_prompts: HashMap<String, String>,
}

/// Configurable prompt templates for each translation style.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PromptTemplates {
    /// Template used for the default translation style.
    #[serde(default = "default_prompt_default")]
    pub default: String,
    /// Template used for the literary translation style.
    #[serde(default = "default_prompt_literary")]
    pub literary: String,
    /// Template used for the technical translation style.
    #[serde(default = "default_prompt_technical")]
    pub technical: String,
    /// Template used for the academic translation style.
    #[serde(default = "default_prompt_academic")]
    pub academic: String,
    /// Template used for the optional refine pass.
    #[serde(default = "default_prompt_refine")]
    pub refine: String,
}

impl Default for PromptTemplates {
    fn default() -> Self {
        Self {
            default: default_prompt_default(),
            literary: default_prompt_literary(),
            technical: default_prompt_technical(),
            academic: default_prompt_academic(),
            refine: default_prompt_refine(),
        }
    }
}

fn default_prompt_default() -> String {
    include_str!("../prompts/default.md").into()
}

fn default_prompt_literary() -> String {
    include_str!("../prompts/literary.md").into()
}

fn default_prompt_technical() -> String {
    include_str!("../prompts/technical.md").into()
}

fn default_prompt_academic() -> String {
    include_str!("../prompts/academic.md").into()
}

fn default_prompt_refine() -> String {
    include_str!("../prompts/refine.md").into()
}

/// Provider-specific configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ProviderConfig {
    /// Provider short name.
    pub name: String,
    /// API key for this provider.
    pub api_key: Option<String>,
    /// Custom base URL for the provider's API.
    pub base_url: Option<String>,
    /// Default model name for this provider.
    pub default_model: String,
    /// Maximum output tokens per request.
    pub max_tokens: usize,
    /// Sampling temperature.
    pub temperature: f32,
    /// Extra provider-specific options.
    pub extra: Option<serde_json::Value>,
}

/// Return the conventional environment variable name for a provider's API key.
///
/// Hyphens in the provider name are converted to underscores so that names
/// such as `openai-compatible` map to valid environment variable names like
/// `OPENAI_COMPATIBLE_API_KEY`.
#[must_use]
pub fn provider_env_var(provider: &str) -> String {
    format!("{}_API_KEY", provider.to_uppercase().replace('-', "_"))
}

impl ProviderConfig {
    /// Return a default configuration for a known provider.
    #[must_use]
    pub fn for_provider(name: &str) -> Self {
        match name {
            "anthropic" => Self {
                name: "anthropic".into(),
                api_key: None,
                base_url: Some("https://api.anthropic.com".into()),
                default_model: "claude-3-5-sonnet-20241022".into(),
                max_tokens: 2000,
                temperature: 0.3,
                extra: None,
            },
            "deepseek" => Self {
                name: "deepseek".into(),
                api_key: None,
                base_url: Some("https://api.deepseek.com".into()),
                default_model: "deepseek-chat".into(),
                max_tokens: 2000,
                temperature: 0.3,
                extra: None,
            },
            "ollama" => Self {
                name: "ollama".into(),
                api_key: None,
                base_url: None,
                default_model: "llama3".into(),
                max_tokens: 2000,
                temperature: 0.3,
                extra: None,
            },
            _ => Self {
                name: name.into(),
                api_key: None,
                base_url: None,
                default_model: "unknown".into(),
                max_tokens: 2000,
                temperature: 0.3,
                extra: None,
            },
        }
    }
}

/// Recognised translation provider short names.
pub const KNOWN_PROVIDERS: &[&str] = &[
    "anthropic",
    "deepseek",
    "ollama",
    "openai",
    "openai-compatible",
];
const SUPPORTED_LOCALES: &[&str] = &["en", "es", "ja", "ko", "ru", "zh-CN"];

fn validate_non_empty_str(value: &str, name: &str) -> Result<(), BabelEbookError> {
    if value.trim().is_empty() {
        return Err(BabelEbookError::Configuration(format!(
            "{name} cannot be empty"
        )));
    }
    Ok(())
}

fn validate_non_empty_path(path: &Path, name: &str) -> Result<(), BabelEbookError> {
    if path.as_os_str().is_empty() {
        return Err(BabelEbookError::Configuration(format!(
            "{name} cannot be empty"
        )));
    }
    Ok(())
}

fn validate_url(url: &str, name: &str) -> Result<(), BabelEbookError> {
    let parsed = Url::parse(url)
        .map_err(|e| BabelEbookError::Configuration(format!("{name} must be a valid URL: {e}")))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(BabelEbookError::Configuration(format!(
            "{name} must use http or https scheme: {url}"
        )));
    }
    Ok(())
}

/// Validate that a file path's parent directory already exists and is a directory.
///
/// The current directory (empty parent or `.`) is accepted without an existence
/// check because the caller may run from a directory that is created later.
fn validate_output_parent(path: &Path, name: &str) -> Result<(), BabelEbookError> {
    if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() || parent == Path::new(".") {
            return Ok(());
        }
        if !parent.exists() {
            return Err(BabelEbookError::Configuration(format!(
                "{name} parent directory does not exist: {}",
                parent.display()
            )));
        }
        if !parent.is_dir() {
            return Err(BabelEbookError::Configuration(format!(
                "{name} parent is not a directory: {}",
                parent.display()
            )));
        }
    }
    Ok(())
}

/// Validate a cache directory path.
///
/// If the directory already exists it must be a directory. If it does not
/// exist, its parent must exist and be a directory so it can be created later.
fn validate_cache_dir(path: &Path, name: &str) -> Result<(), BabelEbookError> {
    if path.exists() {
        if !path.is_dir() {
            return Err(BabelEbookError::Configuration(format!(
                "{name} is not a directory: {}",
                path.display()
            )));
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && parent != Path::new(".") {
            if !parent.exists() {
                return Err(BabelEbookError::Configuration(format!(
                    "{name} parent directory does not exist: {}",
                    parent.display()
                )));
            }
            if !parent.is_dir() {
                return Err(BabelEbookError::Configuration(format!(
                    "{name} parent is not a directory: {}",
                    parent.display()
                )));
            }
        }
    }
    Ok(())
}

fn validate_locale(value: &str, name: &str) -> Result<(), BabelEbookError> {
    validate_non_empty_str(value, name)?;
    if value != "auto" && !SUPPORTED_LOCALES.contains(&value) {
        return Err(BabelEbookError::Configuration(format!(
            "unsupported {name}: {value}"
        )));
    }
    Ok(())
}

fn validate_exclude_selectors(items: &[String]) -> Result<(), BabelEbookError> {
    for item in items {
        if item.trim().is_empty() {
            return Err(BabelEbookError::Configuration(
                "exclude_selectors cannot contain empty entries".into(),
            ));
        }
        if item.chars().any(|c| c.is_whitespace() || ";{}".contains(c)) {
            return Err(BabelEbookError::Configuration(format!(
                "exclude_selectors contains invalid value: {item}"
            )));
        }
    }
    Ok(())
}

fn validate_translate_attributes(items: &[String]) -> Result<(), BabelEbookError> {
    for item in items {
        if item.trim().is_empty() {
            return Err(BabelEbookError::Configuration(
                "translate_attributes cannot contain empty entries".into(),
            ));
        }
        if !item.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(BabelEbookError::Configuration(format!(
                "translate_attributes contains invalid value: {item}"
            )));
        }
    }
    Ok(())
}

fn validate_translate_tags(items: &[String]) -> Result<(), BabelEbookError> {
    for item in items {
        if item.trim().is_empty() {
            return Err(BabelEbookError::Configuration(
                "translate_tags cannot contain empty entries".into(),
            ));
        }
        if !item
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            return Err(BabelEbookError::Configuration(format!(
                "translate_tags contains invalid value: {item}"
            )));
        }
    }
    Ok(())
}

impl Config {
    /// Load a `Config` from a JSON file at the given path.
    ///
    /// # Errors
    ///
    /// Returns `BabelEbookError::Configuration` if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, BabelEbookError> {
        let contents = fs::read_to_string(path).map_err(|err| {
            BabelEbookError::Configuration(format!(
                "failed to load config {}: {err}",
                path.display()
            ))
        })?;
        let config: Self = serde_json::from_str(&contents).map_err(|err| {
            BabelEbookError::Configuration(format!(
                "failed to load config {}: {err}",
                path.display()
            ))
        })?;
        Ok(config)
    }

    /// Validate the configuration values.
    ///
    /// # Errors
    ///
    /// Returns `BabelEbookError::Configuration` if any required value is missing,
    /// out of range, or malformed.
    pub fn validate(&self) -> Result<(), BabelEbookError> {
        // source
        validate_non_empty_path(&self.source, "source")?;
        if !self.dry_run {
            let meta = std::fs::metadata(&self.source).map_err(|e| {
                BabelEbookError::Configuration(format!(
                    "source file does not exist: {} ({e})",
                    self.source.display()
                ))
            })?;
            if !meta.is_file() {
                return Err(BabelEbookError::Configuration(format!(
                    "source is not a file: {}",
                    self.source.display()
                )));
            }
        }

        // output
        validate_non_empty_path(&self.output, "output")?;
        validate_output_parent(&self.output, "output")?;

        // cache_dir
        validate_non_empty_path(&self.cache_dir, "cache_dir")?;
        validate_cache_dir(&self.cache_dir, "cache_dir")?;

        // checkpoint_dir
        validate_non_empty_path(&self.checkpoint_dir, "checkpoint_dir")?;
        validate_cache_dir(&self.checkpoint_dir, "checkpoint_dir")?;

        // provider
        let provider = self.provider.to_ascii_lowercase();
        if !KNOWN_PROVIDERS.contains(&provider.as_str()) {
            return Err(BabelEbookError::Configuration(format!(
                "unknown provider: {}",
                self.provider
            )));
        }

        // model
        validate_non_empty_str(&self.model, "model")?;

        // base_url
        if let Some(url) = &self.base_url {
            validate_url(url, "base_url")?;
        }
        if provider == "openai-compatible"
            && self.base_url.as_ref().is_none_or(|u| u.trim().is_empty())
        {
            return Err(BabelEbookError::Configuration(
                "base_url is required for provider openai-compatible".into(),
            ));
        }

        // api_key
        if !self.dry_run && provider != "ollama" {
            let key = self.api_key.as_deref().unwrap_or("");
            validate_non_empty_str(key, "api_key")?;
        }

        // numeric ranges
        if self.concurrency == 0 {
            return Err(BabelEbookError::Configuration(
                "concurrency must be greater than 0".into(),
            ));
        }
        if self.max_input_tokens == 0 {
            return Err(BabelEbookError::Configuration(
                "max_input_tokens must be greater than 0".into(),
            ));
        }
        if self.max_output_tokens == 0 {
            return Err(BabelEbookError::Configuration(
                "max_output_tokens must be greater than 0".into(),
            ));
        }
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(BabelEbookError::Configuration(
                "temperature must be between 0.0 and 2.0".into(),
            ));
        }

        // locales
        validate_locale(&self.source_lang, "source_lang")?;
        validate_locale(&self.target_lang, "target_lang")?;

        // lists
        validate_exclude_selectors(&self.exclude_selectors)?;
        validate_translate_attributes(&self.translate_attributes)?;
        validate_translate_tags(&self.translate_tags)?;

        // output_font
        if let Some(font) = &self.output_font {
            validate_non_empty_str(font, "output_font")?;
            if font.chars().any(|c| ";{}".contains(c)) {
                return Err(BabelEbookError::Configuration(format!(
                    "output_font contains invalid CSS characters: {font}"
                )));
            }
        }

        Ok(())
    }

    /// Return a copy of the translation-related subset of this configuration.
    #[must_use]
    pub fn translation_options(&self) -> TranslationOptions {
        TranslationOptions {
            source_lang: self.source_lang.clone(),
            target_lang: self.target_lang.clone(),
            output_mode: self.output_mode,
            translation_scope: self.translation_scope.clone(),
            style: self.style.clone(),
            system_prompt: self.system_prompt.clone(),
            prompts: self.prompts.clone(),
            glossary: self.glossary.clone(),
            exclude_selectors: self.exclude_selectors.clone(),
            translate_attributes: self.translate_attributes.clone(),
            translate_tags: self.translate_tags.clone(),
            preserve_classes: self.preserve_classes,
            output_font: self.output_font.clone(),
            refine: self.refine,
            max_input_tokens: self.max_input_tokens,
            max_output_tokens: self.max_output_tokens,
            temperature: self.temperature,
            chapter_prompts: self.chapter_prompts.clone(),
        }
    }

    /// Return the configured system prompt, or the default prompt localised to
    /// `target_lang`.
    #[must_use]
    pub fn system_prompt(&self) -> String {
        self.translation_options().system_prompt()
    }

    /// Return the system prompt configured for a specific chapter, falling back
    /// to the global system prompt when no chapter-specific prompt exists.
    #[must_use]
    pub fn system_prompt_for_chapter(&self, href: &str) -> String {
        self.translation_options().system_prompt_for_chapter(href)
    }

    /// Return the configured refine prompt localised to the source/target
    /// language.
    #[must_use]
    pub fn refine_prompt(&self) -> String {
        self.translation_options().refine_prompt()
    }

    /// Maximum source text tokens for a refine-pass API call.
    ///
    /// Reserves tokens for the refine prompt plus a safety margin and keeps the
    /// expected output within the configured limit.
    #[must_use]
    pub fn max_refine_source_tokens(&self) -> usize {
        self.translation_options().max_refine_source_tokens()
    }

    /// Maximum source text tokens per API call.
    ///
    /// Reserves tokens for the system prompt and keeps the expected output
    /// within the configured limit.
    #[must_use]
    pub fn max_source_tokens(&self) -> usize {
        self.translation_options().max_source_tokens()
    }
}

impl TranslationOptions {
    fn style_prompt(&self) -> String {
        match &self.style {
            TranslationStyle::Default => self.prompts.default.clone(),
            TranslationStyle::Literary => self.prompts.literary.clone(),
            TranslationStyle::Technical => self.prompts.technical.clone(),
            TranslationStyle::Academic => self.prompts.academic.clone(),
            TranslationStyle::Custom(name) => std::fs::read_to_string(format!("prompts/{name}.md"))
                .unwrap_or_else(|_| self.prompts.default.clone()),
        }
    }

    fn glossary_prompt(&self) -> String {
        if self.glossary.is_empty() {
            return String::new();
        }
        let lines: Vec<String> = self
            .glossary
            .iter()
            .map(|e| format!("- {} => {}", e.term, e.translation))
            .collect();
        format!("\nUse the following glossary:\n{}\n", lines.join("\n"))
    }

    /// Return the configured system prompt, or the default prompt localised to
    /// `target_lang`.
    #[must_use]
    #[allow(clippy::literal_string_with_formatting_args)]
    pub fn system_prompt(&self) -> String {
        self.system_prompt.clone().unwrap_or_else(|| {
            let source_lang = if self.source_lang == "auto" {
                "the original language".to_string()
            } else {
                self.source_lang.clone()
            };
            self.style_prompt()
                .replace("{source_lang}", &source_lang)
                .replace("{target_lang}", &self.target_lang)
                + &self.glossary_prompt()
        })
    }

    /// Return the system prompt configured for a specific chapter, falling back
    /// to the global system prompt when no chapter-specific prompt exists.
    #[must_use]
    pub fn system_prompt_for_chapter(&self, href: &str) -> String {
        self.chapter_prompts
            .get(href)
            .cloned()
            .unwrap_or_else(|| self.system_prompt())
    }

    /// Return the configured refine prompt localised to the source/target
    /// language.
    #[must_use]
    #[allow(clippy::literal_string_with_formatting_args)]
    pub fn refine_prompt(&self) -> String {
        let source_lang = if self.source_lang == "auto" {
            "the original language".to_string()
        } else {
            self.source_lang.clone()
        };
        self.prompts
            .refine
            .clone()
            .replace("{source_lang}", &source_lang)
            .replace("{target_lang}", &self.target_lang)
            + &self.glossary_prompt()
    }

    /// Maximum source text tokens for a refine-pass API call.
    ///
    /// Reserves tokens for the refine prompt plus a safety margin and keeps the
    /// expected output within the configured limit.
    #[must_use]
    pub fn max_refine_source_tokens(&self) -> usize {
        let prompt_tokens = crate::chunking::count_tokens(&self.refine_prompt()) + 100;
        let input = self.max_input_tokens.saturating_sub(prompt_tokens);
        let output = self.max_output_tokens.saturating_sub(200);
        input.min(output).max(1)
    }

    /// Maximum source text tokens per API call.
    ///
    /// Reserves tokens for the system prompt and keeps the expected output
    /// within the configured limit.
    #[must_use]
    pub fn max_source_tokens(&self) -> usize {
        let prompt_tokens = crate::chunking::count_tokens(&self.system_prompt()) + 50;
        let input = self.max_input_tokens.saturating_sub(prompt_tokens);
        let output = self.max_output_tokens.saturating_sub(200);
        input.min(output).max(1)
    }
}

fn default_provider() -> String {
    "deepseek".into()
}

fn default_model() -> String {
    "deepseek-chat".into()
}

const fn default_concurrency() -> usize {
    3
}

const fn default_max_input_tokens() -> usize {
    4000
}

const fn default_max_output_tokens() -> usize {
    2000
}

fn default_cache_dir() -> PathBuf {
    PathBuf::from(".babel_ebook_cache")
}

fn default_checkpoint_dir() -> PathBuf {
    PathBuf::from(".babel_ebook_checkpoints")
}

const fn default_temperature() -> f32 {
    0.3
}

fn default_source_lang() -> String {
    "en".into()
}

fn default_target_lang() -> String {
    "zh-CN".into()
}

fn default_skip_doc_patterns() -> Vec<String> {
    vec![
        "cover".into(),
        "titlepage".into(),
        "copyright".into(),
        "dedication".into(),
        "colophon".into(),
    ]
}

fn default_translate_tags() -> Vec<String> {
    vec![
        "p".into(),
        "h1".into(),
        "h2".into(),
        "h3".into(),
        "h4".into(),
        "h5".into(),
        "h6".into(),
        "li".into(),
        "figcaption".into(),
        "dt".into(),
        "dd".into(),
        "td".into(),
        "th".into(),
    ]
}
