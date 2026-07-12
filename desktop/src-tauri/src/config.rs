//! Build `babel_ebook::Config` instances from frontend arguments.

use std::path::PathBuf;

use babel_ebook::{Config, OutputMode, ProviderConfig, TranslationScope, TranslationStyle};

use crate::args::{TestConnectionArgs, TranslateArgs};

pub fn build_config(args: &TranslateArgs) -> Result<Config, String> {
    // Start from typed defaults and override with the frontend arguments. This
    // avoids the fragility of round-tripping through JSON strings and keeps
    // compile-time field checking.
    let mut config = Config {
        source: PathBuf::from(&args.source),
        output: PathBuf::from(&args.output),
        ..Config::default()
    };

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
    // Pass the user-selected model through provider_config so the registry
    // uses it instead of the provider's built-in default.
    config.provider_config = Some(ProviderConfig {
        name: args.provider.clone(),
        api_key: if args.api_key.is_empty() {
            None
        } else {
            Some(args.api_key.clone())
        },
        base_url: args.base_url.clone().filter(|url| !url.is_empty()),
        default_model: args.model.clone(),
        max_tokens: args.max_output_tokens as usize,
        temperature: args.temperature,
        extra: None,
    });
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
    if !args.checkpoint_dir.is_empty() {
        config.checkpoint_dir = PathBuf::from(&args.checkpoint_dir);
    }
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
    if !args.prompts.refine.is_empty() {
        config.prompts.refine.clone_from(&args.prompts.refine);
    }

    Ok(config)
}

pub fn build_test_config(args: &TestConnectionArgs) -> Config {
    let mut config = Config {
        source: PathBuf::from("input.epub"),
        output: PathBuf::from("output.epub"),
        ..Config::default()
    };

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

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::PromptTemplates;

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
        args.prompts.refine = "refine prompt".to_string();
        let config = build_config(&args).unwrap();
        assert_eq!(config.prompts.default, "default prompt");
        assert_eq!(config.prompts.literary, "literary prompt");
        assert_eq!(config.prompts.technical, "technical prompt");
        assert_eq!(config.prompts.academic, "academic prompt");
        assert_eq!(config.prompts.refine, "refine prompt");
    }

    #[test]
    fn build_config_keeps_default_prompts_when_empty() {
        let args = sample_translate_args();
        let config = build_config(&args).unwrap();
        assert!(!config.prompts.default.is_empty());
        assert!(!config.prompts.literary.is_empty());
        assert!(!config.prompts.technical.is_empty());
        assert!(!config.prompts.academic.is_empty());
        assert!(!config.prompts.refine.is_empty());
    }

    #[test]
    fn build_config_maps_output_mode_and_style() {
        let mut args = sample_translate_args();
        args.output_mode = "translation_only".to_string();
        args.style = "literary".to_string();
        let config = build_config(&args).unwrap();
        assert_eq!(config.output_mode, babel_ebook::OutputMode::TranslationOnly);
        assert!(matches!(
            config.style,
            babel_ebook::TranslationStyle::Literary
        ));
    }

    #[test]
    fn build_config_maps_translation_scope() {
        let mut args = sample_translate_args();
        args.translate_body = false;
        args.translate_metadata = false;
        args.translate_toc = false;
        args.translate_alt_text = false;
        args.translate_image_captions = false;
        args.translate_tables = false;
        args.translate_footnotes = false;
        let config = build_config(&args).unwrap();
        assert!(!config.translation_scope.body);
        assert!(!config.translation_scope.metadata);
        assert!(!config.translation_scope.toc);
        assert!(!config.translation_scope.alt_text);
        assert!(!config.translation_scope.image_captions);
        assert!(!config.translation_scope.tables);
        assert!(!config.translation_scope.footnotes);
    }

    #[test]
    fn build_config_adds_pre_code_selectors_when_translate_code_false() {
        let mut args = sample_translate_args();
        args.translate_code = false;
        let config = build_config(&args).unwrap();
        assert!(config.exclude_selectors.contains(&"pre".to_string()));
        assert!(config.exclude_selectors.contains(&"code".to_string()));
    }

    #[test]
    fn build_config_omits_pre_code_selectors_when_translate_code_true() {
        let mut args = sample_translate_args();
        args.translate_code = true;
        let config = build_config(&args).unwrap();
        assert!(!config.exclude_selectors.contains(&"pre".to_string()));
        assert!(!config.exclude_selectors.contains(&"code".to_string()));
    }

    #[test]
    fn build_config_maps_provider_config() {
        let mut args = sample_translate_args();
        args.provider = "deepseek".to_string();
        args.api_key = "sk-test".to_string();
        args.base_url = Some("https://api.example.com".to_string());
        args.model = "custom-model".to_string();
        args.max_output_tokens = 1234;
        args.temperature = 0.7;
        let config = build_config(&args).unwrap();
        let pc = config
            .provider_config
            .expect("provider_config should be set");
        assert_eq!(pc.name, "deepseek");
        assert_eq!(pc.api_key, Some("sk-test".to_string()));
        assert_eq!(pc.base_url, Some("https://api.example.com".to_string()));
        assert_eq!(pc.default_model, "custom-model");
        assert_eq!(pc.max_tokens, 1234);
        assert!((pc.temperature - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn build_config_ignores_empty_api_key_and_base_url() {
        let mut args = sample_translate_args();
        args.api_key = String::new();
        args.base_url = Some(String::new());
        let config = build_config(&args).unwrap();
        assert!(config.api_key.is_none());
        assert!(config.base_url.is_none());
        let pc = config
            .provider_config
            .expect("provider_config should be set");
        assert!(pc.api_key.is_none());
        assert!(pc.base_url.is_none());
    }

    #[test]
    fn build_config_uses_checkpoint_dir_and_resume() {
        let mut args = sample_translate_args();
        args.checkpoint_dir = "/tmp/checkpoints".to_string();
        args.resume = Some("job-42".to_string());
        let config = build_config(&args).unwrap();
        assert_eq!(
            config.checkpoint_dir,
            std::path::PathBuf::from("/tmp/checkpoints")
        );
        assert_eq!(config.resume_job_id, Some("job-42".to_string()));
    }

    #[test]
    fn build_config_rejects_invalid_output_mode() {
        let mut args = sample_translate_args();
        args.output_mode = "invalid".to_string();
        let err = build_config(&args).unwrap_err();
        assert!(err.contains("invalid output_mode"));
    }

    #[test]
    fn build_test_config_uses_default_model() {
        let args = crate::args::TestConnectionArgs {
            provider: "openai".to_string(),
            api_key: "sk-test".to_string(),
            base_url: Some("https://api.openai.com".to_string()),
        };
        let config = build_test_config(&args);
        assert_eq!(config.model, "default");
        assert_eq!(config.provider, "openai");
        assert_eq!(config.api_key, Some("sk-test".to_string()));
        assert_eq!(config.base_url, Some("https://api.openai.com".to_string()));
    }
}
