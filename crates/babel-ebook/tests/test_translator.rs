use async_trait::async_trait;
use babel_ebook::{
    get_translator, BabelEbookError, Config, ProviderConfig, TranslateContext, Translator,
};

/// A fake translator used to verify the trait object works.
pub struct FakeTranslator;

#[async_trait]
impl Translator for FakeTranslator {
    fn name(&self) -> String {
        "fake".into()
    }

    fn max_output_tokens(&self) -> usize {
        2000
    }

    async fn translate(
        &self,
        text: &str,
        _context: &TranslateContext<'_>,
    ) -> Result<String, BabelEbookError> {
        Ok(format!("[ZH] {}", text))
    }
}

#[tokio::test]
async fn test_fake_translator_translate() {
    let translator: Box<dyn Translator> = Box::new(FakeTranslator);
    let context = TranslateContext {
        system_prompt: "translate to {target_lang}",
        target_lang: "zh-CN",
    };
    assert_eq!(
        translator.translate("hello", &context).await.unwrap(),
        "[ZH] hello"
    );
}

#[tokio::test]
async fn test_fake_translator_uses_context() {
    let translator = FakeTranslator;
    let context = TranslateContext {
        system_prompt: "translate to {target_lang}",
        target_lang: "zh-CN",
    };
    let result = translator.translate("hello", &context).await.unwrap();
    assert!(result.contains("[ZH]"));
}

#[tokio::test]
async fn test_fake_translator_metadata() {
    let translator: Box<dyn Translator> = Box::new(FakeTranslator);
    assert_eq!(translator.name(), "fake");
    assert_eq!(translator.max_output_tokens(), 2000);
}

fn test_config(api_key: Option<&str>) -> Config {
    let extra = api_key.map_or_else(String::new, |k| format!(r#","api_key":"{k}""#));
    let json = format!(r#"{{"source":"x.epub","output":"y.epub","provider":"deepseek"{extra}}}"#);
    serde_json::from_str(&json).expect("valid test config")
}

#[test]
fn test_translator_registry_deepseek_defaults() {
    let config = test_config(Some("dummy-key"));
    let translator = get_translator("deepseek", None, &config, false)
        .expect("deepseek provider should be registered");
    assert_eq!(translator.name(), "deepseek:deepseek-chat");
    assert_eq!(translator.max_output_tokens(), 2000);
}

#[test]
fn test_translator_registry_deepseek_custom_model() {
    let config = test_config(Some("dummy-key"));
    let provider_config = ProviderConfig {
        default_model: "deepseek-reasoner".into(),
        max_tokens: 4096,
        temperature: 0.7,
        ..ProviderConfig::for_provider("deepseek")
    };
    let translator = get_translator("deepseek", Some(&provider_config), &config, false)
        .expect("deepseek provider should accept a custom model");
    assert_eq!(translator.name(), "deepseek:deepseek-reasoner");
    assert_eq!(translator.max_output_tokens(), 4096);
}

#[test]
fn test_translator_registry_deepseek_uppercase() {
    let config = test_config(Some("dummy-key"));
    let translator = get_translator("DEEPSEEK", None, &config, false)
        .expect("deepseek provider should match case-insensitively");
    assert_eq!(translator.name(), "deepseek:deepseek-chat");
    assert_eq!(translator.max_output_tokens(), 2000);
}

#[test]
fn test_translator_registry_unknown_provider() {
    let config = test_config(Some("key"));
    match get_translator("google", None, &config, false) {
        Err(err) => {
            assert!(matches!(err, BabelEbookError::ProviderNotFound(_)));
            assert!(err.to_string().contains("google"));
        }
        Ok(_) => panic!("unknown provider should fail"),
    }
}

#[test]
fn test_openai_translator_returns_name() {
    let config = test_config(None);
    let provider_config = ProviderConfig {
        name: "openai".into(),
        api_key: Some("fake".into()),
        base_url: None,
        default_model: "gpt-4o-mini".into(),
        max_tokens: 100,
        temperature: 0.0,
        extra: None,
    };
    let translator = get_translator("openai", Some(&provider_config), &config, false)
        .expect("openai provider should be registered");
    assert!(translator.name().contains("openai"));
}

#[test]
fn test_translator_registry_anthropic_defaults() {
    let config = test_config(Some("fake-key"));
    let translator = get_translator("anthropic", None, &config, false)
        .expect("anthropic provider should be registered");
    assert_eq!(translator.name(), "anthropic:claude-3-5-sonnet-20241022");
    assert_eq!(translator.max_output_tokens(), 2000);
}

#[test]
fn test_translator_registry_anthropic_returns_name() {
    let config = test_config(None);
    let provider_config = ProviderConfig {
        name: "anthropic".into(),
        api_key: Some("fake".into()),
        base_url: None,
        default_model: "claude-3-5-sonnet-20241022".into(),
        max_tokens: 2000,
        temperature: 0.3,
        extra: None,
    };
    let translator = get_translator("anthropic", Some(&provider_config), &config, false)
        .expect("anthropic provider should be registered");
    assert!(translator.name().contains("anthropic"));
    assert_eq!(translator.name(), "anthropic:claude-3-5-sonnet-20241022");
}

#[tokio::test]
async fn test_translator_deepseek_max_tokens_config_error_fails_fast() {
    let oversized: usize = u32::MAX as usize + 1;
    let config = test_config(Some("dummy-key"));
    let provider_config = ProviderConfig {
        max_tokens: oversized,
        ..ProviderConfig::for_provider("deepseek")
    };
    let translator = get_translator("deepseek", Some(&provider_config), &config, false)
        .expect("registry should accept the translator");

    let context = TranslateContext {
        system_prompt: "translate to {target_lang}",
        target_lang: "zh-CN",
    };
    let err = translator
        .translate("hello", &context)
        .await
        .expect_err("max_tokens > u32::MAX should fail immediately");

    assert!(
        matches!(err, BabelEbookError::Configuration(_)),
        "expected configuration error, got {err}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("max_tokens") && msg.contains("u32::MAX"),
        "error message should describe the configuration problem: {msg}"
    );
    assert!(
        !matches!(err, BabelEbookError::ApiError(_)),
        "configuration error must not be wrapped as an API failure"
    );
}
