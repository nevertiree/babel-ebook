//! Translator registry.

use crate::config::{provider_env_var, Config, ProviderConfig};
use crate::core::BabelEbookError;
use crate::translator::{
    anthropic::AnthropicTranslator, deepseek::DeepSeekTranslator, ollama::OllamaTranslator,
    openai::OpenAiTranslator, Translator,
};

/// Build a translator for the requested provider.
pub fn get_translator(
    name: &str,
    provider_config: Option<&ProviderConfig>,
    global_config: &Config,
    dry_run: bool,
) -> Result<Box<dyn Translator>, BabelEbookError> {
    let defaults = ProviderConfig::for_provider(name);
    let mut cfg = provider_config.cloned().unwrap_or_else(|| defaults.clone());

    // Base URL precedence: provider_config > global_config > provider default.
    cfg.base_url = cfg
        .base_url
        .clone()
        .or_else(|| global_config.base_url.clone())
        .or(defaults.base_url);

    let api_key = resolve_api_key(name, &cfg, global_config, dry_run)?;
    let model = if cfg.default_model.is_empty() || cfg.default_model == "unknown" {
        global_config.model.clone()
    } else {
        cfg.default_model.clone()
    };
    let max_tokens = if cfg.max_tokens == 0 {
        global_config.max_output_tokens
    } else {
        cfg.max_tokens
    };

    match name.to_ascii_lowercase().as_str() {
        "anthropic" => Ok(Box::new(AnthropicTranslator::new(
            api_key,
            Some(model),
            cfg.base_url,
            max_tokens,
            cfg.temperature,
        ))),
        "deepseek" => Ok(Box::new(DeepSeekTranslator::new(
            api_key,
            Some(model),
            cfg.base_url,
            max_tokens,
            cfg.temperature,
        ))),
        "openai" | "openai-compatible" => Ok(Box::new(OpenAiTranslator::new(
            api_key,
            Some(model),
            cfg.base_url,
            max_tokens,
            cfg.temperature,
        ))),
        "ollama" => Ok(Box::new(OllamaTranslator::new(
            api_key,
            Some(model),
            cfg.base_url,
        )?)),
        _ => Err(BabelEbookError::ProviderNotFound(name.into())),
    }
}

fn resolve_api_key(
    name: &str,
    provider_config: &ProviderConfig,
    global_config: &Config,
    dry_run: bool,
) -> Result<String, BabelEbookError> {
    if dry_run {
        // Dry runs never call the API; return a placeholder so construction
        // can succeed without a real key.
        return Ok(String::new());
    }

    // Ollama runs locally and does not require an API key.
    if name.eq_ignore_ascii_case("ollama") {
        return Ok(String::new());
    }

    // Precedence: global_config.api_key > provider_config.api_key > env vars.
    if let Some(key) = global_config
        .api_key
        .as_ref()
        .or(provider_config.api_key.as_ref())
    {
        if !key.is_empty() {
            return Ok(key.clone());
        }
    }

    let provider_env = provider_env_var(name);
    for env_name in [&provider_env, "LLM_API_KEY", "OPENAI_API_KEY"] {
        if let Ok(key) = std::env::var(env_name) {
            if !key.is_empty() {
                return Ok(key);
            }
        }
    }

    Err(BabelEbookError::Configuration(format!(
        "API key is required for provider {name}. Pass an api_key or set {provider_env}, LLM_API_KEY, or OPENAI_API_KEY."
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(api_key: Option<&str>) -> Config {
        let extra = api_key.map_or_else(String::new, |k| format!(r#","api_key":"{k}""#));
        let json =
            format!(r#"{{"source":"x.epub","output":"y.epub","provider":"deepseek"{extra}}}"#);
        serde_json::from_str(&json).expect("valid test config")
    }

    fn with_env_vars<F>(names: &[&str], f: F)
    where
        F: FnOnce(),
    {
        let saved: Vec<(String, Option<String>)> = names
            .iter()
            .map(|n| ((*n).to_string(), std::env::var(n).ok()))
            .collect();
        for n in names {
            std::env::remove_var(n);
        }
        f();
        for (n, v) in saved {
            match v {
                Some(value) => std::env::set_var(n, value),
                None => std::env::remove_var(n),
            }
        }
    }

    #[test]
    fn resolve_api_key_precedence() {
        let mut global_config = test_config(Some("from-global"));
        let provider_config = ProviderConfig::for_provider("deepseek");
        with_env_vars(
            &["DEEPSEEK_API_KEY", "LLM_API_KEY", "OPENAI_API_KEY"],
            || {
                // Global config key wins over provider config and env.
                let provider_with_key = ProviderConfig {
                    api_key: Some("from-provider".into()),
                    ..provider_config.clone()
                };
                std::env::set_var("DEEPSEEK_API_KEY", "from-env");
                assert_eq!(
                    resolve_api_key("deepseek", &provider_with_key, &global_config, false).unwrap(),
                    "from-global"
                );

                // Provider config key wins over env when global config has no key.
                global_config.api_key = None;
                assert_eq!(
                    resolve_api_key("deepseek", &provider_with_key, &global_config, false).unwrap(),
                    "from-provider"
                );

                // Env is the fallback when no config key is set.
                assert_eq!(
                    resolve_api_key("deepseek", &provider_config, &global_config, false).unwrap(),
                    "from-env"
                );
            },
        );
    }

    #[test]
    fn resolve_api_key_dry_run_returns_placeholder() {
        let config = test_config(None);
        assert_eq!(
            resolve_api_key(
                "deepseek",
                &ProviderConfig::for_provider("deepseek"),
                &config,
                true
            )
            .unwrap(),
            ""
        );
    }

    #[test]
    fn registry_returns_ollama_translator() {
        with_env_vars(&["OLLAMA_API_KEY", "LLM_API_KEY", "OPENAI_API_KEY"], || {
            let config = test_config(None);
            let translator =
                get_translator("ollama", None, &config, false).expect("ollama provider exists");
            assert_eq!(translator.name(), "ollama:llama3");
        });
    }
}
