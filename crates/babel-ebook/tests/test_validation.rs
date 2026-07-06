// crates/babel-ebook/tests/test_validation.rs
// Tests for Config::validate().

use std::path::{Path, PathBuf};

use babel_ebook::Config;

fn valid_config(source: &Path, output: &Path) -> Config {
    let json = format!(
        r#"{{"source":{},"output":{},"api_key":"test-key"}}"#,
        serde_json::to_string(&source).unwrap(),
        serde_json::to_string(&output).unwrap(),
    );
    serde_json::from_str(&json).unwrap()
}

#[test]
fn valid_config_passes() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let config = valid_config(&source, &output);
    assert!(config.validate().is_ok());
}

#[test]
fn missing_source_rejected_when_not_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("missing.epub");
    let output = dir.path().join("out.epub");
    let config = valid_config(&source, &output);
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("source file does not exist"));
}

#[test]
fn missing_source_allowed_in_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("missing.epub");
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.dry_run = true;
    assert!(config.validate().is_ok());
}

#[test]
fn zero_concurrency_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.concurrency = 0;
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("concurrency must be greater than 0"));
}

#[test]
fn zero_max_input_tokens_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.max_input_tokens = 0;
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("max_input_tokens must be greater than 0"));
}

#[test]
fn zero_max_output_tokens_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.max_output_tokens = 0;
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("max_output_tokens must be greater than 0"));
}

#[test]
fn out_of_range_temperature_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.temperature = 2.5;
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("temperature must be between 0.0 and 2.0"));
}

#[test]
fn unknown_provider_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.provider = "unknown".into();
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("unknown provider"));
}

#[test]
fn openai_compatible_requires_base_url() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.provider = "openai-compatible".into();
    config.base_url = None;
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("base_url is required"));
}

#[test]
fn invalid_base_url_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.base_url = Some("not a url".into());
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("base_url must be a valid URL"));
}

#[test]
fn missing_api_key_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.api_key = None;
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("api_key cannot be empty"));
}

#[test]
fn ollama_allows_missing_api_key() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.provider = "ollama".into();
    config.api_key = None;
    assert!(config.validate().is_ok());
}

#[test]
fn unsupported_locale_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.target_lang = "fr".into();
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("unsupported target_lang"));
}

#[test]
fn invalid_exclude_selector_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.exclude_selectors = vec!["pre code".into()];
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("exclude_selectors contains invalid value"));
}

#[test]
fn invalid_translate_attribute_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.translate_attributes = vec!["data-title".into(), "bad value".into()];
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("translate_attributes contains invalid value"));
}

#[test]
fn invalid_translate_tag_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.translate_tags = vec!["P".into()];
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("translate_tags contains invalid value"));
}

#[test]
fn unsafe_output_font_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.output_font = Some("font;evil".into());
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("output_font contains invalid CSS characters"));
}

#[test]
fn negative_temperature_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.temperature = -0.1;
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("temperature must be between 0.0 and 2.0"));
}

#[test]
fn empty_model_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.model = "".into();
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("model cannot be empty"));
}

#[test]
fn empty_source_path_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = Path::new("");
    let output = dir.path().join("out.epub");
    let mut config = valid_config(source, &output);
    config.dry_run = true;
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("source cannot be empty"));
}

#[test]
fn empty_output_path_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = Path::new("");
    let config = valid_config(&source, output);
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("output cannot be empty"));
}

#[test]
fn empty_cache_dir_path_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.cache_dir = PathBuf::new();
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("cache_dir cannot be empty"));
}

#[test]
fn invalid_base_url_scheme_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.epub");
    std::fs::write(&source, b"x").unwrap();
    let output = dir.path().join("out.epub");
    let mut config = valid_config(&source, &output);
    config.base_url = Some("ftp://example.com".into());
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("base_url must use http or https"));
}
