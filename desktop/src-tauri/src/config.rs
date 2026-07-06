//! Build `babel_ebook::Config` instances from frontend arguments.

use std::path::{Path, PathBuf};

use babel_ebook::{Config, OutputMode, TranslationScope, TranslationStyle};

use crate::args::{TestConnectionArgs, TranslateArgs};

/// Convert a non-EPUB ebook to EPUB using Calibre's `ebook-convert`.
/// EPUB sources are returned unchanged.
pub fn convert_to_epub(source: &str) -> Result<PathBuf, String> {
    let source_path = Path::new(source);
    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "epub" {
        return Ok(source_path.to_path_buf());
    }

    let temp_dir = std::env::temp_dir().join(format!("babel_ebook_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("book");
    let output = temp_dir.join(format!("{stem}.epub"));

    let result = std::process::Command::new("ebook-convert")
        .arg(source)
        .arg(&output)
        .output()
        .map_err(|e| format!("ebook-convert not found or failed to start: {e}"))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("ebook-convert failed: {stderr}"));
    }

    Ok(output)
}

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
    config.model.clone_from(&args.model);
    config.temperature = args.temperature;
    config.dry_run = false;
    config.verbose = false;

    Ok(config)
}
