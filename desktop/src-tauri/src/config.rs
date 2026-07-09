//! Build `babel_ebook::Config` instances from frontend arguments.

use std::path::PathBuf;

use babel_ebook::{Config, OutputMode, TranslationScope, TranslationStyle};

use crate::args::{TestConnectionArgs, TranslateArgs};

pub fn build_config(args: &TranslateArgs) -> Result<Config, String> {
    // Start from a JSON skeleton so that Config's serde defaults (skip patterns,
    // translate tags, cache dir, etc.) are filled in by the core crate.
    let source = serde_json::to_string(&args.source).map_err(|e| e.to_string())?;
    let output = serde_json::to_string(&args.output).map_err(|e| e.to_string())?;
    let json = format!(r#"{{"source":{source},"output":{output}}}"#);
    let mut config: Config = serde_json::from_str(&json).map_err(|e| e.to_string())?;

    config.provider.clone_from(&args.provider);
    config.api_key = if args.api_key.is_empty() {
        None
    } else {
        Some(args.api_key.clone())
    };
    config.base_url = args.base_url.clone().filter(|url| !url.is_empty());
    config.output_mode = match args.output_mode.as_str() {
        "bilingual" => OutputMode::Bilingual,
        "translation_only" => OutputMode::TranslationOnly,
        "interleaved" => OutputMode::Interleaved,
        other => return Err(format!("invalid output_mode: {other}")),
    };
    config.style = match args.style.as_str() {
        "default" => TranslationStyle::Default,
        "literary" => TranslationStyle::Literary,
        "technical" => TranslationStyle::Technical,
        "academic" => TranslationStyle::Academic,
        other => TranslationStyle::Custom(other.to_string()),
    };
    config.preserve_classes = args.preserve_classes;
    config.exclude_selectors.clone_from(&args.exclude_selectors);
    config
        .translate_attributes
        .clone_from(&args.translate_attributes);
    config.translation_scope = TranslationScope {
        body: args.translate_body,
        metadata: args.translate_metadata,
        toc: args.translate_toc,
        alt_text: args.translate_alt_text,
        image_captions: args.translate_image_captions,
        tables: args.translate_tables,
        footnotes: args.translate_footnotes,
    };
    if !args.translate_code {
        config
            .exclude_selectors
            .extend(["pre".to_string(), "code".to_string()]);
    }
    config.output_font = args.output_font.clone().filter(|f| !f.is_empty());
    config.model.clone_from(&args.model);
    config.concurrency = args.concurrency as usize;
    config.max_input_tokens = args.max_input_tokens as usize;
    config.max_output_tokens = args.max_output_tokens as usize;
    config.cache_dir = PathBuf::from(".babel_ebook_cache");
    config.temperature = args.temperature;
    config.source_lang.clone_from(&args.source_lang);
    config.target_lang.clone_from(&args.target_lang);
    config.dry_run = args.dry_run;
    config.verbose = false;
    config.refine = args.refine;
    config.checkpoint_dir = PathBuf::from(&args.checkpoint_dir);
    config.resume_job_id.clone_from(&args.resume);
    config.system_prompt = args.system_prompt.clone().filter(|s| !s.is_empty());
    if !args.prompts.default.is_empty() {
        config.prompts.default.clone_from(&args.prompts.default);
    }
    if !args.prompts.literary.is_empty() {
        config.prompts.literary.clone_from(&args.prompts.literary);
    }
    if !args.prompts.technical.is_empty() {
        config.prompts.technical.clone_from(&args.prompts.technical);
    }
    if !args.prompts.academic.is_empty() {
        config.prompts.academic.clone_from(&args.prompts.academic);
    }

    Ok(config)
}

pub fn build_test_config(args: &TestConnectionArgs) -> Result<Config, String> {
    let source = serde_json::to_string("input.epub").map_err(|e| e.to_string())?;
    let output = serde_json::to_string("output.epub").map_err(|e| e.to_string())?;
    let json = format!(r#"{{"source":{source},"output":{output}}}"#);
    let mut config: Config = serde_json::from_str(&json).map_err(|e| e.to_string())?;

    config.provider.clone_from(&args.provider);
    config.api_key = if args.api_key.is_empty() {
        None
    } else {
        Some(args.api_key.clone())
    };
    config.base_url = args.base_url.clone().filter(|url| !url.is_empty());
    // Connection health checks do not depend on a specific model.
    config.model = "default".to_string();
    config.dry_run = false;
    config.verbose = false;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use crate::args::{PromptTemplates, TranslateArgs};
    use crate::config::build_config;

    fn sample_translate_args() -> TranslateArgs {
        TranslateArgs {
            source: "input.epub".to_string(),
            output: "output.epub".to_string(),
            provider: "deepseek".to_string(),
            api_key: String::new(),
            model: "deepseek-chat".to_string(),
            concurrency: 1,
            max_input_tokens: 4000,
            max_output_tokens: 2000,
            temperature: 0.3,
            source_lang: "en".to_string(),
            target_lang: "zh-CN".to_string(),
            dry_run: true,
            base_url: None,
            output_mode: "bilingual".to_string(),
            style: "default".to_string(),
            preserve_classes: false,
            exclude_selectors: Vec::new(),
            translate_attributes: Vec::new(),
            translate_body: true,
            translate_metadata: true,
            translate_toc: true,
            translate_alt_text: true,
            translate_image_captions: true,
            translate_tables: true,
            translate_footnotes: true,
            translate_code: false,
            output_font: None,
            system_prompt: None,
            prompts: PromptTemplates::default(),
            refine: false,
            checkpoint_dir: ".babel_ebook_checkpoints".to_string(),
            resume: None,
        }
    }

    #[test]
    fn build_config_propagates_system_prompt() {
        let mut args = sample_translate_args();
        args.system_prompt = Some("custom system prompt".to_string());
        let config = build_config(&args).unwrap();
        assert_eq!(
            config.system_prompt,
            Some("custom system prompt".to_string())
        );
    }

    #[test]
    fn build_config_ignores_empty_system_prompt() {
        let mut args = sample_translate_args();
        args.system_prompt = Some(String::new());
        let config = build_config(&args).unwrap();
        assert_eq!(config.system_prompt, None);
    }

    #[test]
    fn build_config_propagates_prompt_templates() {
        let mut args = sample_translate_args();
        args.prompts.default = "default prompt".to_string();
        args.prompts.literary = "literary prompt".to_string();
        args.prompts.technical = "technical prompt".to_string();
        args.prompts.academic = "academic prompt".to_string();
        let config = build_config(&args).unwrap();
        assert_eq!(config.prompts.default, "default prompt");
        assert_eq!(config.prompts.literary, "literary prompt");
        assert_eq!(config.prompts.technical, "technical prompt");
        assert_eq!(config.prompts.academic, "academic prompt");
    }

    #[test]
    fn build_config_keeps_default_prompts_when_empty() {
        let args = sample_translate_args();
        let config = build_config(&args).unwrap();
        assert!(!config.prompts.default.is_empty());
        assert!(!config.prompts.literary.is_empty());
        assert!(!config.prompts.technical.is_empty());
        assert!(!config.prompts.academic.is_empty());
    }
}
