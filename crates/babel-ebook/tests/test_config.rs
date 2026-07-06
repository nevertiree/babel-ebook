use std::io::Write;

use anyhow::{Context, Result};
use babel_ebook::config::{GlossaryEntry, OutputMode, TranslationStyle};
use babel_ebook::Config;

fn write_config(dir: &tempfile::TempDir, content: &str) -> std::path::PathBuf {
    let path = dir.path().join("config.json");
    let mut file = std::fs::File::create(&path).expect("create config file");
    file.write_all(content.as_bytes()).expect("write config");
    path
}

#[test]
fn load_minimal_config_uses_defaults() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_config(
        &dir,
        r#"{
            "source": "input.epub",
            "output": "output.epub"
        }"#,
    );

    let config = Config::load(&path).context("failed to load config")?;

    assert_eq!(config.source, std::path::PathBuf::from("input.epub"));
    assert_eq!(config.output, std::path::PathBuf::from("output.epub"));
    assert_eq!(config.provider, "deepseek");
    assert_eq!(config.model, "deepseek-chat");
    assert_eq!(config.concurrency, 3);
    assert_eq!(config.max_input_tokens, 4000);
    assert_eq!(config.max_output_tokens, 2000);
    assert_eq!(
        config.cache_dir,
        std::path::PathBuf::from(".babel_ebook_cache")
    );
    assert!((config.temperature - 0.3).abs() < f32::EPSILON);
    assert_eq!(config.target_lang, "zh-CN");
    assert_eq!(
        config.skip_doc_patterns,
        vec!["cover", "titlepage", "copyright", "dedication", "colophon"]
    );
    assert_eq!(
        config.translate_tags,
        vec![
            "p",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "li",
            "figcaption",
            "dt",
            "dd",
            "td",
            "th"
        ]
    );
    assert!(!config.dry_run);
    assert!(!config.verbose);
    assert!(config.api_key.is_none());
    assert!(config.base_url.is_none());
    Ok(())
}

#[test]
fn default_system_prompt_uses_target_lang() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_config(
        &dir,
        r#"{
            "source": "input.epub",
            "output": "output.epub",
            "target_lang": "fr-FR"
        }"#,
    );

    let config = Config::load(&path).context("failed to load config")?;
    let prompt = config.system_prompt();

    assert!(prompt.contains("fr-FR"));
    assert!(prompt.contains("professional translator"));
    Ok(())
}

#[test]
fn custom_system_prompt_is_preserved() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_config(
        &dir,
        r#"{
            "source": "input.epub",
            "output": "output.epub",
            "system_prompt": "Custom prompt"
        }"#,
    );

    let config = Config::load(&path).context("failed to load config")?;
    assert_eq!(config.system_prompt(), "Custom prompt");
    Ok(())
}

#[test]
fn max_source_tokens_reserves_room() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_config(
        &dir,
        r#"{
            "source": "input.epub",
            "output": "output.epub"
        }"#,
    );

    let config = Config::load(&path).context("failed to load config")?;
    let max_source = config.max_source_tokens();
    assert!(max_source > 0);
    assert!(max_source < config.max_input_tokens);
    assert!(max_source < config.max_output_tokens);
    Ok(())
}

#[test]
fn unknown_fields_are_ignored() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_config(
        &dir,
        r#"{
            "source": "input.epub",
            "output": "output.epub",
            "unknown_field": "ignored"
        }"#,
    );

    let config = Config::load(&path).context("failed to load config")?;
    assert_eq!(config.source, std::path::PathBuf::from("input.epub"));
    Ok(())
}

#[test]
fn load_config_with_custom_options() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_config(
        &dir,
        r#"{
            "source": "a.epub",
            "output": "b.epub",
            "output_mode": "translation_only",
            "style": "technical",
            "glossary": [{"term": "AI", "translation": "人工智能"}]
        }"#,
    );

    let config = Config::load(&path).context("failed to load config")?;
    assert_eq!(config.output_mode, OutputMode::TranslationOnly);
    assert!(matches!(config.style, TranslationStyle::Technical));
    assert_eq!(config.glossary.len(), 1);
    assert_eq!(
        config.glossary[0],
        GlossaryEntry {
            term: "AI".into(),
            translation: "人工智能".into(),
            context: None,
        }
    );
    assert!(config.translation_scope.body);
    assert!(config.exclude_selectors.is_empty());
    Ok(())
}

#[test]
fn technical_system_prompt_uses_technical_template() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_config(
        &dir,
        r#"{
            "source": "input.epub",
            "output": "output.epub",
            "style": "technical",
            "target_lang": "zh-CN"
        }"#,
    );

    let config = Config::load(&path).context("failed to load config")?;
    let prompt = config.system_prompt();

    assert!(prompt.contains("technical translator"));
    assert!(prompt.contains("zh-CN"));
    assert!(!prompt.contains("professional translator"));
    Ok(())
}

#[test]
fn system_prompt_includes_glossary_entries() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_config(
        &dir,
        r#"{
            "source": "input.epub",
            "output": "output.epub",
            "target_lang": "zh-CN",
            "glossary": [
                {"term": "AI", "translation": "人工智能"},
                {"term": "CPU", "translation": "中央处理器"}
            ]
        }"#,
    );

    let config = Config::load(&path).context("failed to load config")?;
    let prompt = config.system_prompt();

    assert!(prompt.contains("Use the following glossary:"));
    assert!(prompt.contains("- AI => 人工智能"));
    assert!(prompt.contains("- CPU => 中央处理器"));
    Ok(())
}

#[test]
fn chapter_system_prompt_overrides_style_and_glossary() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_config(
        &dir,
        r#"{
            "source": "input.epub",
            "output": "output.epub",
            "style": "technical",
            "target_lang": "zh-CN",
            "glossary": [{"term": "AI", "translation": "人工智能"}],
            "chapter_prompts": {
                "chapter.xhtml": "Chapter-specific prompt."
            }
        }"#,
    );

    let config = Config::load(&path).context("failed to load config")?;
    let prompt = config.system_prompt_for_chapter("chapter.xhtml");

    assert_eq!(prompt, "Chapter-specific prompt.");
    assert!(!prompt.contains("technical translator"));
    assert!(!prompt.contains("Use the following glossary"));
    Ok(())
}
