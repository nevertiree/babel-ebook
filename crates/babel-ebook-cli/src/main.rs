//! BabelEbook CLI for translating EPUB books and converting scanned PDFs.

// `warn` keeps `cargo test` passing while surfacing missing docs; Task 1.2 will
// add the documentation and switch this to `#![deny(missing_docs)]`.
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::doc_markdown)]
#![warn(clippy::nursery)]
#![warn(clippy::perf)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use babel_ebook::{t, Config, ProviderConfig};
use clap::parser::ValueSource;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

// Load translations from the core crate's locale files so the `t!` macro can
// resolve keys in this binary crate.
rust_i18n::i18n!("../babel-ebook/locales", fallback = "en");

/// Top-level CLI commands.
#[derive(Parser)]
#[command(name = "babel-ebook", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Translate an EPUB/MOBI/TXT/SRT/DOCX file to a target language.
    Translate(TranslateArgs),
    /// Convert a scanned PDF to EPUB using OCR + LLM verification.
    PdfToEpub(PdfToEpubArgs),
}

/// Arguments for the `translate` command.
#[derive(Parser)]
struct TranslateArgs {
    /// Path to the source file.
    source: PathBuf,
    /// Path to the output EPUB file.
    #[arg(short, long)]
    output: PathBuf,
    /// Translation provider short name.
    #[arg(long, default_value = "deepseek")]
    provider: String,
    /// API key for the selected provider.
    #[arg(long)]
    api_key: Option<String>,
    /// Custom base URL for the provider's API.
    #[arg(long)]
    base_url: Option<String>,
    /// Model name to use with the provider.
    #[arg(long, default_value = "deepseek-chat")]
    model: String,
    /// Maximum number of concurrent translation requests.
    #[arg(long, default_value_t = 3)]
    concurrency: usize,
    /// Maximum input tokens per API request.
    #[arg(long, default_value_t = 4000)]
    max_input_tokens: usize,
    /// Maximum output tokens per API request.
    #[arg(long, default_value_t = 2000)]
    max_output_tokens: usize,
    /// UI language (en, es, ja, ko, ru, zh-CN) or "auto" to detect the system locale.
    #[arg(long, default_value = "auto")]
    lang: String,
    /// Directory where translation cache entries are stored.
    #[arg(long, default_value = ".babel_ebook_cache")]
    cache_dir: PathBuf,
    /// Sampling temperature for the LLM.
    #[arg(long, default_value_t = 0.3)]
    temperature: f32,
    /// Target language for the translation.
    #[arg(long, default_value = "zh-CN")]
    target_lang: String,
    /// Optional path to a JSON config file.
    #[arg(long)]
    config: Option<PathBuf>,
    /// If true, only estimate token usage without calling the API.
    #[arg(long)]
    dry_run: bool,
    /// If true, enable verbose logging.
    #[arg(short, long)]
    verbose: bool,
    /// If true, refine an existing translation instead of translating from scratch.
    #[arg(long)]
    refine: bool,
    /// Directory where translation checkpoints are stored.
    #[arg(long, default_value = ".babel_ebook_checkpoints")]
    checkpoint_dir: PathBuf,
    /// Optional job ID to resume a previously interrupted translation.
    #[arg(long)]
    resume: Option<String>,
}

/// Arguments for the `pdf-to-epub` command.
#[derive(Parser)]
struct PdfToEpubArgs {
    /// Path to the source PDF file.
    pdf: PathBuf,
    /// Path to the output EPUB file.
    #[arg(short, long)]
    output: PathBuf,
    /// Title for the generated EPUB.
    #[arg(short, long)]
    title: Option<String>,
    /// OCR provider short name.
    #[arg(long, default_value = "qwen-vl-ocr")]
    ocr_provider: String,
    /// API key for the OCR provider.
    #[arg(long)]
    ocr_api_key: Option<String>,
    /// Base URL for the OCR provider.
    #[arg(
        long,
        default_value = "https://dashscope.aliyuncs.com/compatible-mode/v1"
    )]
    ocr_base_url: String,
    /// Model name for the OCR provider.
    #[arg(long, default_value = "qwen-vl-ocr")]
    ocr_model: String,
    /// Number of pages to OCR concurrently.
    #[arg(long, default_value_t = 3)]
    ocr_concurrency: usize,
    /// Verifier provider short name (openai, deepseek, qwen-vl-ocr, ...).
    #[arg(long, default_value = "deepseek")]
    verify_provider: String,
    /// API key for the verifier provider.
    #[arg(long)]
    verify_api_key: Option<String>,
    /// Base URL for the verifier provider.
    #[arg(long)]
    verify_base_url: Option<String>,
    /// Model name for the verifier provider.
    #[arg(long)]
    verify_model: Option<String>,
    /// Disable the LLM verification pass.
    #[arg(long)]
    no_verify: bool,
    /// Rendering resolution in DPI.
    #[arg(long, default_value_t = 200)]
    dpi: u32,
    /// Confidence threshold below which a block is verified.
    #[arg(long, default_value_t = 0.7)]
    verify_threshold: f32,
    /// Maximum number of verify attempts for a low-confidence block.
    #[arg(long, default_value_t = 3)]
    verify_max_attempts: usize,
    /// Comma-separated scale factors for verify retry crops (e.g. 1,2,3).
    #[arg(long, default_value = "1,2,3")]
    verify_scale_factors: String,
    /// UI language (en, es, ja, ko, ru, zh-CN) or "auto" to detect the system locale.
    #[arg(long, default_value = "auto")]
    lang: String,
    /// If true, enable verbose logging.
    #[arg(short, long)]
    verbose: bool,
}

/// Build a runtime `Config` from the optional config file and CLI overrides.
///
/// CLI arguments override the config file only when they are explicitly
/// provided on the command line. This matches the Python implementation's
/// merge semantics.
fn build_config(args: &TranslateArgs, matches: &clap::ArgMatches) -> Result<Config> {
    let mut config = match &args.config {
        Some(path) => Config::load(path)
            .map_err(|err| anyhow::anyhow!(err.to_string()))
            .context(t!("err_load_config").to_string())?,
        None => serde_json::from_str(r#"{"source":"","output":""}"#)
            .context(t!("err_default_config").to_string())?,
    };

    config.source.clone_from(&args.source);
    config.output.clone_from(&args.output);

    if is_explicit(matches, "provider") {
        config.provider.clone_from(&args.provider);
    }
    if is_explicit(matches, "api_key") {
        config.api_key.clone_from(&args.api_key);
    }
    if is_explicit(matches, "base_url") {
        config.base_url.clone_from(&args.base_url);
    }
    if is_explicit(matches, "model") {
        config.model.clone_from(&args.model);
    }
    if is_explicit(matches, "concurrency") {
        config.concurrency = args.concurrency;
    }
    if is_explicit(matches, "max_input_tokens") {
        config.max_input_tokens = args.max_input_tokens;
    }
    if is_explicit(matches, "max_output_tokens") {
        config.max_output_tokens = args.max_output_tokens;
    }
    if is_explicit(matches, "cache_dir") {
        config.cache_dir.clone_from(&args.cache_dir);
    }
    if is_explicit(matches, "temperature") {
        config.temperature = args.temperature;
    }
    if is_explicit(matches, "target_lang") {
        config.target_lang.clone_from(&args.target_lang);
    }
    if args.dry_run {
        config.dry_run = true;
    }
    if args.verbose {
        config.verbose = true;
    }
    if args.refine {
        config.refine = true;
    }
    if is_explicit(matches, "checkpoint_dir") {
        config.checkpoint_dir.clone_from(&args.checkpoint_dir);
    }
    if let Some(job_id) = &args.resume {
        config.resume_job_id = Some(job_id.clone());
    }

    if is_explicit(matches, "base_url") {
        if let Some(url) = &config.base_url {
            config.provider_config = Some(ProviderConfig {
                name: config.provider.clone(),
                api_key: config.api_key.clone(),
                base_url: Some(url.clone()),
                default_model: config.model.clone(),
                max_tokens: config.max_output_tokens,
                temperature: config.temperature,
                extra: None,
            });
        }
    }

    Ok(config)
}

fn is_explicit(matches: &clap::ArgMatches, id: &str) -> bool {
    matches.value_source(id) == Some(ValueSource::CommandLine)
}

/// Normalize an OS locale string for use with `babel_ebook::set_locale`.
///
/// Replaces underscores with hyphens and strips any charset suffix
/// (e.g. `.UTF-8`) so values like `zh_CN.UTF-8` become `zh-CN`.
fn normalize_locale(locale: &str) -> String {
    let without_charset = locale.split('.').next().unwrap_or(locale);
    without_charset.replace('_', "-")
}

/// Initialize the global locale from the `--lang` flag.
///
/// Uses `sys-locale` to detect the system locale when `lang` is `"auto"`,
/// otherwise uses the provided value directly.
fn init_locale(lang: &str) {
    let chosen = if lang == "auto" {
        sys_locale::get_locale().map_or_else(|| "en".into(), |locale| normalize_locale(&locale))
    } else {
        lang.into()
    };
    babel_ebook::set_locale(&chosen);
}

/// Run the translate subcommand.
async fn run_translate(args: TranslateArgs, matches: clap::ArgMatches) -> Result<()> {
    init_locale(&args.lang);

    let config = build_config(&args, &matches)?;
    config
        .validate()
        .context(t!("err_invalid_config").to_string())?;

    let level = if config.verbose {
        tracing_subscriber::filter::LevelFilter::DEBUG
    } else {
        tracing_subscriber::filter::LevelFilter::INFO
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    let translator = babel_ebook::translator::get_translator(
        &config.provider,
        config.provider_config.as_ref(),
        &config,
        config.dry_run,
    )
    .context(t!("err_create_translator").to_string())?;

    tokio::task::spawn_blocking(move || -> Result<(), anyhow::Error> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to build local tokio runtime")?;
        rt.block_on(async {
            babel_ebook::translate_epub(&config, translator.as_ref(), None, None)
                .await
                .context(t!("err_translation").to_string())
        })
    })
    .await
    .context(t!("err_translation").to_string())??;

    Ok(())
}

/// Run the pdf-to-epub subcommand.
async fn run_pdf_to_epub(args: PdfToEpubArgs) -> Result<()> {
    init_locale(&args.lang);

    let level = if args.verbose {
        tracing_subscriber::filter::LevelFilter::DEBUG
    } else {
        tracing_subscriber::filter::LevelFilter::INFO
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    let ocr_api_key = args.ocr_api_key.ok_or_else(|| {
        anyhow::anyhow!("--ocr-api-key is required (or set DASHSCOPE_API_KEY in the environment)")
    })?;

    let ocr = babel_ebook::pdf_ocr::QwenOcrBackend::new(babel_ebook::pdf_ocr::QwenOcrConfig {
        api_key: ocr_api_key,
        base_url: Some(args.ocr_base_url),
        model: args.ocr_model,
    });

    let verifier: Option<Box<dyn babel_ebook::pdf_ocr::VerifyBackend>> = if args.no_verify {
        None
    } else {
        let verify_api_key = args.verify_api_key.ok_or_else(|| {
            anyhow::anyhow!("--verify-api-key is required unless --no-verify is set")
        })?;
        let base_url = args.verify_base_url.ok_or_else(|| {
            anyhow::anyhow!("--verify-base-url is required for the verifier provider")
        })?;
        let model = args.verify_model.ok_or_else(|| {
            anyhow::anyhow!("--verify-model is required for the verifier provider")
        })?;
        Some(Box::new(babel_ebook::pdf_ocr::OpenAiVerifyBackend::new(
            babel_ebook::pdf_ocr::OpenAiVerifyConfig {
                api_key: verify_api_key,
                base_url,
                model,
            },
        )))
    };

    let title = args.title.unwrap_or_else(|| {
        args.pdf
            .file_stem()
            .map_or_else(|| "Untitled".into(), |s| s.to_string_lossy().into_owned())
    });

    let verify_scale_factors: Vec<f32> = args
        .verify_scale_factors
        .split(',')
        .map(|part| part.trim().parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("invalid --verify-scale-factors: {e}"))?;

    let config = babel_ebook::pdf_ocr::PdfToEpubConfig {
        dpi: args.dpi,
        verify_threshold: args.verify_threshold,
        verify_max_attempts: args.verify_max_attempts,
        verify_scale_factors,
        ocr_concurrency: args.ocr_concurrency,
        ..babel_ebook::pdf_ocr::PdfToEpubConfig::default()
    };

    babel_ebook::pdf_ocr::convert_pdf_to_epub_file(
        &args.pdf,
        &args.output,
        &title,
        &ocr,
        verifier.as_deref(),
        &config,
    )
    .await
    .context("pdf-to-epub conversion failed")?;

    tracing::info!(output = %args.output.display(), "PDF converted to EPUB successfully");
    Ok(())
}

/// Entry point for the `babel-ebook` CLI.
#[tokio::main]
async fn main() -> Result<()> {
    let cmd = Cli::command();
    let matches = cmd.get_matches();
    let cli =
        Cli::from_arg_matches(&matches).unwrap_or_else(|_| panic!("{}", t!("err_parsed_args")));

    match cli.command {
        Command::Translate(args) => {
            // Re-parse the subcommand matches so build_config can inspect value sources.
            let sub_matches = matches
                .subcommand_matches("translate")
                .expect("translate subcommand matches")
                .clone();
            run_translate(args, sub_matches).await
        }
        Command::PdfToEpub(args) => run_pdf_to_epub(args).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use babel_ebook::provider_env_var;

    fn parse_translate_args(cmdline: &[&str]) -> (TranslateArgs, clap::ArgMatches) {
        let cmd = Cli::command();
        let matches = cmd.try_get_matches_from(cmdline).unwrap();
        let cli = Cli::from_arg_matches(&matches).unwrap();
        let Command::Translate(args) = cli.command else {
            panic!("expected translate command")
        };
        let sub_matches = matches.subcommand_matches("translate").unwrap().clone();
        (args, sub_matches)
    }

    #[test]
    fn build_config_preserves_file_values_when_cli_is_default() {
        let dir = std::env::temp_dir();
        let path = dir.join("babel-ebook-test-config.json");
        std::fs::write(
            &path,
            r#"{"source":"file.epub","output":"file-out.epub","model":"custom-model"}"#,
        )
        .unwrap();

        let (args, matches) = parse_translate_args(&[
            "babel-ebook",
            "translate",
            "cli.epub",
            "-o",
            "cli-out.epub",
            "--config",
            path.to_str().unwrap(),
        ]);
        let config = build_config(&args, &matches).unwrap();
        assert_eq!(config.model, "custom-model");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn build_config_overrides_file_values_when_cli_is_explicit() {
        let dir = std::env::temp_dir();
        let path = dir.join("babel-ebook-test-config2.json");
        std::fs::write(
            &path,
            r#"{"source":"file.epub","output":"file-out.epub","model":"custom-model"}"#,
        )
        .unwrap();

        let (args, matches) = parse_translate_args(&[
            "babel-ebook",
            "translate",
            "cli.epub",
            "-o",
            "cli-out.epub",
            "--config",
            path.to_str().unwrap(),
            "--model",
            "explicit-model",
        ]);
        let config = build_config(&args, &matches).unwrap();
        assert_eq!(config.model, "explicit-model");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn build_config_explicit_default_overrides_file_value() {
        let dir = std::env::temp_dir();
        let path = dir.join("babel-ebook-test-config3.json");
        std::fs::write(
            &path,
            r#"{"source":"file.epub","output":"file-out.epub","model":"custom-model"}"#,
        )
        .unwrap();

        // Passing the clap default value explicitly should still override the config file.
        let (args, matches) = parse_translate_args(&[
            "babel-ebook",
            "translate",
            "cli.epub",
            "-o",
            "cli-out.epub",
            "--config",
            path.to_str().unwrap(),
            "--model",
            "deepseek-chat",
        ]);
        let config = build_config(&args, &matches).unwrap();
        assert_eq!(config.model, "deepseek-chat");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn provider_env_var_follows_name() {
        assert_eq!(provider_env_var("openai"), "OPENAI_API_KEY");
        assert_eq!(
            provider_env_var("openai-compatible"),
            "OPENAI_COMPATIBLE_API_KEY"
        );
    }

    #[test]
    fn build_config_populates_provider_config_when_base_url_is_explicit() {
        let (args, matches) = parse_translate_args(&[
            "babel-ebook",
            "translate",
            "cli.epub",
            "-o",
            "cli-out.epub",
            "--provider",
            "openai-compatible",
            "--base-url",
            "https://example.com/v1",
            "--model",
            "gpt-4",
            "--max-output-tokens",
            "1000",
            "--temperature",
            "0.7",
        ]);
        let config = build_config(&args, &matches).unwrap();
        let provider_config = config
            .provider_config
            .expect("provider_config should be set when --base-url is explicit");
        assert_eq!(provider_config.name, "openai-compatible");
        assert_eq!(
            provider_config.base_url,
            Some("https://example.com/v1".into())
        );
        assert_eq!(provider_config.default_model, "gpt-4");
        assert_eq!(provider_config.max_tokens, 1000);
        assert!((provider_config.temperature - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn build_config_uses_file_verbose_flag() {
        let dir = std::env::temp_dir();
        let path = dir.join("babel-ebook-test-config4.json");
        std::fs::write(
            &path,
            r#"{"source":"file.epub","output":"file-out.epub","verbose":true}"#,
        )
        .unwrap();

        let (args, matches) = parse_translate_args(&[
            "babel-ebook",
            "translate",
            "cli.epub",
            "-o",
            "cli-out.epub",
            "--config",
            path.to_str().unwrap(),
        ]);
        let config = build_config(&args, &matches).unwrap();
        assert!(config.verbose);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn normalize_locale_replaces_underscores_and_strips_charset() {
        assert_eq!(normalize_locale("zh_CN.UTF-8"), "zh-CN");
        assert_eq!(normalize_locale("en_US"), "en-US");
        assert_eq!(normalize_locale("ja_JP.UTF-8"), "ja-JP");
        assert_eq!(normalize_locale("ko_KR"), "ko-KR");
        assert_eq!(normalize_locale("ru_RU.UTF-8"), "ru-RU");
        assert_eq!(normalize_locale("es_ES"), "es-ES");
        assert_eq!(normalize_locale("en"), "en");
    }

    #[test]
    fn build_config_validation_rejects_missing_source() {
        let (args, matches) = parse_translate_args(&[
            "babel-ebook",
            "translate",
            "definitely-missing-file.epub",
            "-o",
            "out.epub",
        ]);
        let config = build_config(&args, &matches).unwrap();
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("source file does not exist"));
    }

    #[test]
    fn cli_accepts_refine_and_checkpoint_args() {
        let (args, _matches) = parse_translate_args(&[
            "babel-ebook",
            "translate",
            "book.epub",
            "-o",
            "out.epub",
            "--refine",
            "--checkpoint-dir",
            ".checkpoints",
            "--resume",
            "abc123",
        ]);
        assert!(args.refine);
        assert_eq!(args.checkpoint_dir, PathBuf::from(".checkpoints"));
        assert_eq!(args.resume, Some("abc123".into()));
    }

    #[test]
    fn pdf_to_epub_command_parses() {
        let cmd = Cli::command();
        let matches = cmd
            .try_get_matches_from([
                "babel-ebook",
                "pdf-to-epub",
                "book.pdf",
                "-o",
                "book.epub",
                "--ocr-api-key",
                "sk-test",
                "--verify-api-key",
                "sk-test",
                "--verify-base-url",
                "https://api.example.com/v1",
                "--verify-model",
                "gpt-4o",
            ])
            .unwrap();
        assert!(matches.subcommand_matches("pdf-to-epub").is_some());
    }
}
