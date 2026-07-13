//! Tauri commands exposed to the desktop frontend.

// Tauri command signatures are driven by the framework: arguments are passed by
// value and commands return `Result` so serialization errors are handled.
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unnecessary_wraps)]

#[cfg(not(test))]
use std::sync::Arc;

use babel_ebook::{
    read_input_book, run_dry_run, translatable_chapters, translator::get_translator,
    CancellationToken, ProgressCallback, ProviderConfig, TranslationCache, TranslationJob,
    TranslationJobHandle, TranslationWorker, KNOWN_PROVIDERS,
};

/// Summary of a translation checkpoint returned to the frontend.
#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct CheckpointInfo {
    /// Checkpoint job id.
    pub job_id: String,
    /// Hash of the source file used to detect changes.
    pub source_hash: String,
    /// Path to the source file, inferred from the job id when missing.
    pub source_path: String,
    /// Whether this checkpoint matches the currently selected source file.
    pub matches_current_source: bool,
    /// Number of completed chapters.
    pub completed: usize,
    /// Total number of chapters.
    pub total: usize,
    /// Number of failed chapters.
    pub failed: usize,
    /// Number of pending chapters.
    pub pending: usize,
}

impl CheckpointInfo {
    fn from_checkpoint(cp: &babel_ebook::checkpoint::Checkpoint) -> Self {
        use babel_ebook::checkpoint::ChapterStatus;
        let completed = cp
            .chapters
            .iter()
            .filter(|c| c.status == ChapterStatus::Completed)
            .count();
        let failed = cp
            .chapters
            .iter()
            .filter(|c| c.status == ChapterStatus::Failed)
            .count();
        let total = cp.chapters.len();
        Self {
            job_id: cp.job_id.clone(),
            source_hash: cp.source_hash.clone(),
            source_path: cp.source_path.clone(),
            matches_current_source: false,
            completed,
            total,
            failed,
            pending: total.saturating_sub(completed + failed),
        }
    }
}
#[cfg(not(test))]
use tauri::Emitter;

#[cfg(not(test))]
use crate::args::E2EArgs;
use crate::args::{PdfToEpubArgs, TestConnectionArgs, TranslateArgs};
use crate::config::{build_config, build_test_config};
use crate::queue::{QueueManager, QueueState};
use crate::task::Task;

/// Read E2E injection values from the environment.
///
/// Supported variables: `BABEL_EBOOK_E2E_SOURCE`, `BABEL_EBOOK_E2E_OUTPUT`,
/// `BABEL_EBOOK_E2E_CHECKPOINT_DIR`, `BABEL_EBOOK_E2E_API_KEY`,
/// `BABEL_EBOOK_E2E_DRY_RUN`, `BABEL_EBOOK_E2E_UI_LANGUAGE`.
#[cfg(not(test))]
#[tauri::command]
pub fn get_e2e_args() -> E2EArgs {
    let source = std::env::var("BABEL_EBOOK_E2E_SOURCE").ok();
    let output = std::env::var("BABEL_EBOOK_E2E_OUTPUT").ok();
    let checkpoint_dir = std::env::var("BABEL_EBOOK_E2E_CHECKPOINT_DIR").ok();
    let api_key = std::env::var("BABEL_EBOOK_E2E_API_KEY").ok();
    let dry_run = std::env::var("BABEL_EBOOK_E2E_DRY_RUN")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let ui_language = std::env::var("BABEL_EBOOK_E2E_UI_LANGUAGE").ok();
    tracing::debug!(
        source = ?source,
        output = ?output,
        checkpoint_dir = ?checkpoint_dir,
        api_key_present = api_key.is_some(),
        dry_run = ?dry_run,
        ui_language = ?ui_language,
        "get_e2e_args"
    );
    E2EArgs {
        source,
        output,
        checkpoint_dir,
        api_key,
        dry_run,
        ui_language,
    }
}

#[cfg(not(test))]
pub struct WindowProgressCallback(tauri::Window);

#[cfg(not(test))]
impl ProgressCallback for WindowProgressCallback {
    fn on_progress(&self, event: babel_ebook::ProgressEvent) {
        if let Err(err) = self.0.emit("translation_progress", event) {
            tracing::warn!("failed to emit translation_progress event: {err}");
        }
    }
}

/// Tauri command that translates an EPUB according to the provided arguments.
#[cfg(not(test))]
#[tauri::command]
pub async fn translate_epub(
    args: TranslateArgs,
    window: tauri::Window,
    worker: tauri::State<'_, Arc<TranslationWorker>>,
) -> Result<String, String> {
    let progress: Option<Box<dyn ProgressCallback + Send + Sync>> =
        Some(Box::new(WindowProgressCallback(window)));
    run_translation(args, progress, None, &worker).await
}

/// Run a translation job on the dedicated worker and forward progress events.
///
/// This is the shared implementation used by both the direct translate command
/// and the queue worker loop.
pub async fn run_translation(
    args: TranslateArgs,
    progress: Option<Box<dyn ProgressCallback + Send + Sync>>,
    cancellation: Option<CancellationToken>,
    worker: &TranslationWorker,
) -> Result<String, String> {
    let config = build_config(&args)?;
    config.validate().map_err(|e| e.to_string())?;
    tracing::info!(
        source = %config.source.display(),
        output = %config.output.display(),
        provider = %config.provider,
        dry_run = config.dry_run,
        "translating ebook"
    );

    let progress_ref: Option<&dyn ProgressCallback> = progress
        .as_ref()
        .map(|p| p.as_ref() as &dyn ProgressCallback);

    if config.dry_run {
        let book = read_input_book(&config.source).map_err(|e| e.to_string())?;
        let indices =
            translatable_chapters(&book, &config.skip_doc_patterns).map_err(|e| e.to_string())?;
        let (tokens, docs) = run_dry_run(&book, &indices, progress_ref);
        return Ok(format!(
            "Estimated source tokens: {tokens} ({docs} documents)"
        ));
    }

    let translator = get_translator(
        &config.provider,
        config.provider_config.as_ref(),
        &config,
        config.dry_run,
    )
    .map_err(|e| e.to_string())?;
    let cache = TranslationCache::new(config.cache_dir.clone());
    let job = TranslationJob::new(config.clone(), translator)
        .with_cache(cache)
        .with_cancellation(cancellation.unwrap_or_default());

    let TranslationJobHandle {
        progress: mut progress_rx,
        result_rx,
    } = worker.submit(job).await.map_err(|e| e.to_string())?;

    let forwarder = tokio::spawn(async move {
        while let Some(event) = progress_rx.recv().await {
            if let Some(p) = progress.as_ref() {
                p.on_progress(event);
            }
        }
    });

    let result = result_rx
        .await
        .map_err(|_| "translation worker dropped".to_string())?;
    if let Err(err) = forwarder.await {
        tracing::error!("progress forwarder task failed: {err}");
    }

    tracing::info!(
        output = %config.output.display(),
        "translation command completed successfully"
    );
    match result {
        Ok(()) => Ok(format!(
            "Translation completed: {}",
            config.output.display()
        )),
        Err(e) => Err(e.to_string()),
    }
}

/// Returns whether the given path exists on disk.
#[allow(dead_code, clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn check_file_exists(path: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || std::path::Path::new(&path).exists())
        .await
        .map_err(|e| e.to_string())
}

/// Suggests an output path based on the source file and a user-defined template.
#[allow(
    dead_code,
    clippy::needless_pass_by_value,
    clippy::literal_string_with_formatting_args
)]
#[tauri::command]
pub fn suggest_output_path(
    source: String,
    source_lang: String,
    target_lang: String,
    output_mode: String,
    output_filename_template: String,
) -> String {
    use std::path::Path;

    let source_path = Path::new(&source);
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let parent = dirs::download_dir()
        .or_else(|| source_path.parent().map(Path::to_path_buf))
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| String::from("."));

    let template = if output_filename_template.trim().is_empty() {
        "{stem}_{target_lang}"
    } else {
        &output_filename_template
    };

    let rendered = template
        .replace("{stem}", stem)
        .replace("{source_lang}", &source_lang)
        .replace("{target_lang}", &target_lang)
        .replace("{output_mode}", &output_mode);

    Path::new(&parent)
        .join(format!("{rendered}.epub"))
        .to_string_lossy()
        .into_owned()
}

/// Returns the best-matching supported UI locale for the host system.
///
/// Falls back to `"en"` when the system locale cannot be detected or is not
/// supported.
#[allow(dead_code)]
#[tauri::command]
pub fn get_system_locale() -> String {
    const SUPPORTED: &[&str] = &["en", "es", "ja", "ko", "ru", "zh-CN"];

    if let Some(locale) = sys_locale::get_locale() {
        let normalized = locale.replace('_', "-").to_lowercase();

        // Exact match first.
        if let Some(code) = SUPPORTED
            .iter()
            .find(|code| normalized == code.to_lowercase())
        {
            return (*code).to_string();
        }

        // Language-only fallback.
        if let Some(lang) = normalized.split('-').next() {
            if let Some(code) = SUPPORTED
                .iter()
                .find(|code| code.to_lowercase().starts_with(lang))
            {
                return (*code).to_string();
            }
        }
    }

    "en".to_string()
}

fn validate_connection_args(args: &TestConnectionArgs) -> Result<(), String> {
    let provider = args.provider.to_ascii_lowercase();
    if !KNOWN_PROVIDERS.contains(&provider.as_str()) {
        return Err(format!("unknown provider: {}", args.provider));
    }
    if args.api_key.trim().is_empty() && provider != "ollama" {
        return Err("api_key is required".into());
    }
    if let Some(url) = &args.base_url {
        if !url.trim().is_empty() {
            let parsed =
                url::Url::parse(url).map_err(|e| format!("base_url must be a valid URL: {e}"))?;
            if parsed.scheme() != "http" && parsed.scheme() != "https" {
                return Err("base_url must use http or https scheme".into());
            }
        }
    }
    if provider == "openai-compatible" && args.base_url.as_ref().is_none_or(|u| u.trim().is_empty())
    {
        return Err("base_url is required for provider openai-compatible".into());
    }
    Ok(())
}

/// Verify that a provider is reachable with the given credentials.
#[allow(dead_code)]
#[tauri::command]
pub async fn test_connection(args: TestConnectionArgs) -> Result<String, String> {
    validate_connection_args(&args)?;
    let config = build_test_config(&args);
    let mut provider_config = ProviderConfig::for_provider(&args.provider);
    provider_config.base_url = args.base_url.clone().filter(|url| !url.is_empty());

    let translator = get_translator(&args.provider, Some(&provider_config), &config, false)
        .map_err(|e| e.to_string())?;

    translator.health_check().await.map_err(|e| e.to_string())?;
    Ok("connection ok".to_string())
}

/// List available models for the given provider.
#[allow(dead_code)]
#[tauri::command]
pub async fn list_models(args: TestConnectionArgs) -> Result<Vec<String>, String> {
    validate_connection_args(&args)?;
    let config = build_test_config(&args);
    let mut provider_config = ProviderConfig::for_provider(&args.provider);
    provider_config.base_url = args.base_url.clone().filter(|url| !url.is_empty());

    let translator = get_translator(&args.provider, Some(&provider_config), &config, false)
        .map_err(|e| e.to_string())?;

    translator.list_models().await.map_err(|e| e.to_string())
}

/// Return the built-in default prompt templates for each translation style.
#[allow(dead_code)]
#[tauri::command]
pub fn get_default_prompts() -> crate::args::PromptTemplates {
    let core = babel_ebook::config::PromptTemplates::default();
    crate::args::PromptTemplates {
        default: core.default,
        literary: core.literary,
        technical: core.technical,
        academic: core.academic,
        refine: core.refine,
    }
}

/// Return the application version compiled from Cargo.
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Add a book to the translation queue using the current form arguments.
#[allow(dead_code)]
#[tauri::command]
pub async fn enqueue_task(
    args: TranslateArgs,
    queue: tauri::State<'_, QueueManager>,
) -> Result<Task, String> {
    let config = build_config(&args)?;
    config.validate().map_err(|e| e.to_string())?;
    Ok(queue.enqueue(args).await)
}

/// Remove a pending or finished task from the queue.
#[allow(dead_code)]
#[tauri::command]
pub async fn remove_task(id: String, queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.remove(&id).await.map_err(|e| e.to_string())
}

/// Reorder pending tasks to match the provided list of ids.
#[allow(dead_code)]
#[tauri::command]
pub async fn reorder_tasks(
    ids: Vec<String>,
    queue: tauri::State<'_, QueueManager>,
) -> Result<(), String> {
    queue.reorder(&ids).await.map_err(|e| e.to_string())
}

/// Cancel a pending task.
#[allow(dead_code)]
#[tauri::command]
pub async fn cancel_task(id: String, queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.cancel(&id).await.map_err(|e| e.to_string())
}

/// Retry a failed or cancelled task.
#[allow(dead_code)]
#[tauri::command]
pub async fn retry_task(id: String, queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.retry(&id).await.map_err(|e| e.to_string())
}

/// Resume a paused task from its last checkpoint.
#[allow(dead_code)]
#[tauri::command]
pub async fn resume_task(id: String, queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.resume_task(&id).await.map_err(|e| e.to_string())
}

/// Start processing the queue.
#[allow(dead_code)]
#[tauri::command]
pub async fn start_queue(queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.start().await;
    Ok(())
}

/// Pause after the current task finishes.
#[allow(dead_code)]
#[tauri::command]
pub async fn pause_queue(queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.pause().await;
    Ok(())
}

/// Pause the currently running task so it can be resumed later.
#[allow(dead_code)]
#[tauri::command]
pub async fn pause_task(id: String, queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.pause_task(&id).await.map_err(|e| e.to_string())
}

/// Return the current queue state.
#[allow(dead_code)]
#[tauri::command]
pub async fn get_queue_state(queue: tauri::State<'_, QueueManager>) -> Result<QueueState, String> {
    Ok(queue.state().await)
}

/// List translation checkpoints stored in `checkpoint_dir`.
#[allow(dead_code)]
#[tauri::command]
pub async fn list_checkpoints(
    checkpoint_dir: String,
    current_source: Option<String>,
) -> Result<Vec<CheckpointInfo>, String> {
    let dir = std::path::PathBuf::from(checkpoint_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let current_hash = current_source.as_ref().and_then(|path| {
        babel_ebook::checkpoint::CheckpointStore::source_hash(std::path::Path::new(path)).ok()
    });

    let mut entries = tokio::task::spawn_blocking(move || {
        let mut checkpoints = Vec::new();
        let read_dir = std::fs::read_dir(&dir).map_err(|e| format!("read checkpoint dir: {e}"))?;
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(cp) = serde_json::from_str::<babel_ebook::checkpoint::Checkpoint>(&text) else {
                continue;
            };
            let mut info = CheckpointInfo::from_checkpoint(&cp);
            if info.source_path.is_empty() {
                // Fallback: store the source file name in the first chapter href if available.
                info.source_path = cp
                    .chapters
                    .first()
                    .map(|c| c.href.clone())
                    .unwrap_or_default();
            }
            if let Some(hash) = current_hash.as_ref() {
                info.matches_current_source = !cp.source_hash.is_empty() && cp.source_hash == *hash;
            }
            checkpoints.push(info);
        }
        Ok::<Vec<CheckpointInfo>, String>(checkpoints)
    })
    .await
    .map_err(|e| format!("checkpoint listing task panicked: {e}"))??;

    entries.sort_by(|a, b| a.job_id.cmp(&b.job_id));
    Ok(entries)
}

/// Convert a scanned PDF to an EPUB using OCR + LLM verification.
#[allow(dead_code)]
#[tauri::command]
pub async fn convert_pdf_to_epub(args: PdfToEpubArgs) -> Result<String, String> {
    let pdf_path = std::path::PathBuf::from(&args.pdf_path);
    let output_path = std::path::PathBuf::from(&args.output_path);

    let title = args.title.unwrap_or_else(|| {
        pdf_path
            .file_stem()
            .map_or_else(|| "Untitled".into(), |s| s.to_string_lossy().into_owned())
    });

    let ocr_api_key = args.ocr_api_key.clone();
    let ocr_base_url = args.ocr_base_url.clone();
    let ocr_config = babel_ebook::pdf_ocr::QwenOcrConfig {
        api_key: args.ocr_api_key,
        base_url: args.ocr_base_url,
        model: args.ocr_model.unwrap_or_else(|| "qwen-vl-ocr".into()),
    };
    let ocr = babel_ebook::pdf_ocr::QwenOcrBackend::new(ocr_config);

    let verifier: Option<Box<dyn babel_ebook::pdf_ocr::VerifyBackend>> = if args.no_verify {
        None
    } else {
        let verify_api_key = args
            .verify_api_key
            .ok_or("verify_api_key is required unless no_verify is set")?;
        let verify_base_url = args
            .verify_base_url
            .ok_or("verify_base_url is required for the verifier")?;
        let verify_model = args
            .verify_model
            .ok_or("verify_model is required for the verifier")?;
        Some(Box::new(babel_ebook::pdf_ocr::OpenAiVerifyBackend::new(
            babel_ebook::pdf_ocr::OpenAiVerifyConfig {
                api_key: verify_api_key,
                base_url: verify_base_url,
                model: verify_model,
            },
        )))
    };

    let refiner: Option<Box<dyn babel_ebook::pdf_ocr::RefineBackend>> =
        if args.ocr_refine_rounds == 0 {
            None
        } else {
            Some(Box::new(babel_ebook::pdf_ocr::OpenAiRefineBackend::new(
                babel_ebook::pdf_ocr::OpenAiRefineConfig {
                    api_key: args.ocr_refine_api_key.clone().unwrap_or(ocr_api_key),
                    base_url: args
                        .ocr_refine_base_url
                        .clone()
                        .or(ocr_base_url)
                        .unwrap_or_else(|| {
                            "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string()
                        }),
                    model: args
                        .ocr_refine_model
                        .clone()
                        .unwrap_or_else(|| "qwen-max".into()),
                    max_tokens: 4096,
                    include_image: args.ocr_refine_with_image,
                },
            )))
        };

    let config = babel_ebook::pdf_ocr::PdfToEpubConfig {
        dpi: args.dpi,
        verify_threshold: args.verify_threshold,
        verify_max_attempts: args.verify_max_attempts,
        verify_scale_factors: args.verify_scale_factors,
        ocr_concurrency: args.ocr_concurrency,
        refine_rounds: args.ocr_refine_rounds,
        ..babel_ebook::pdf_ocr::PdfToEpubConfig::default()
    };

    babel_ebook::pdf_ocr::convert_pdf_to_epub_file(
        &pdf_path,
        &output_path,
        &title,
        &ocr,
        verifier.as_deref(),
        refiner.as_deref(),
        &config,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(output_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_args_validation_rejects_empty_api_key() {
        let args = TestConnectionArgs {
            provider: "deepseek".to_string(),
            api_key: String::new(),
            base_url: None,
        };
        let err = validate_connection_args(&args).unwrap_err();
        assert!(err.contains("api_key is required"));
    }

    #[test]
    fn connection_args_validation_rejects_unknown_provider() {
        let args = TestConnectionArgs {
            provider: "not-real".to_string(),
            api_key: "test".to_string(),
            base_url: None,
        };
        let err = validate_connection_args(&args).unwrap_err();
        assert!(err.contains("unknown provider"));
    }

    #[test]
    fn connection_args_validation_allows_ollama_without_api_key() {
        let args = TestConnectionArgs {
            provider: "ollama".to_string(),
            api_key: String::new(),
            base_url: None,
        };
        assert!(validate_connection_args(&args).is_ok());
    }

    #[test]
    fn connection_args_validation_requires_base_url_for_openai_compatible() {
        let args = TestConnectionArgs {
            provider: "openai-compatible".to_string(),
            api_key: "test".to_string(),
            base_url: None,
        };
        let err = validate_connection_args(&args).unwrap_err();
        assert!(err.contains("base_url is required"));
    }

    #[test]
    fn connection_args_validation_rejects_invalid_base_url_scheme() {
        let args = TestConnectionArgs {
            provider: "deepseek".to_string(),
            api_key: "test".to_string(),
            base_url: Some("ftp://example.com".to_string()),
        };
        let err = validate_connection_args(&args).unwrap_err();
        assert!(err.contains("http or https"));
    }

    #[test]
    fn suggest_output_path_uses_default_template_when_empty() {
        let result = suggest_output_path(
            "/home/user/book.epub".to_string(),
            "en".to_string(),
            "zh-CN".to_string(),
            "bilingual".to_string(),
            String::new(),
        );
        assert!(result.ends_with("book_zh-CN.epub"), "got {result}");
    }

    #[test]
    fn suggest_output_path_renders_custom_template() {
        let result = suggest_output_path(
            "/home/user/book.epub".to_string(),
            "en".to_string(),
            "zh-CN".to_string(),
            "translation_only".to_string(),
            "{stem}_{source_lang}_{target_lang}_{output_mode}".to_string(),
        );
        assert!(
            result.ends_with("book_en_zh-CN_translation_only.epub"),
            "got {result}"
        );
    }

    #[tokio::test]
    async fn check_file_exists_true_for_existing_file() {
        let path = std::env::current_dir().unwrap().join("Cargo.toml");
        assert!(check_file_exists(path.to_string_lossy().to_string())
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn check_file_exists_false_for_missing_file() {
        assert!(
            !check_file_exists("/nonexistent/babel_ebook_missing_file.txt".to_string())
                .await
                .unwrap()
        );
    }

    #[test]
    fn get_system_locale_returns_supported_code() {
        let locale = get_system_locale();
        assert!(
            ["en", "es", "ja", "ko", "ru", "zh-CN"].contains(&locale.as_str()),
            "unexpected locale: {locale}"
        );
    }

    #[test]
    fn get_default_prompts_returns_non_empty_templates() {
        let prompts = get_default_prompts();
        assert!(!prompts.default.is_empty());
        assert!(!prompts.literary.is_empty());
        assert!(!prompts.technical.is_empty());
        assert!(!prompts.academic.is_empty());
        assert!(!prompts.refine.is_empty());
    }

    #[tokio::test]
    async fn list_checkpoints_parses_valid_checkpoints() {
        let dir = tempfile::tempdir().unwrap();
        let cp = babel_ebook::checkpoint::Checkpoint {
            job_id: "job-1".to_string(),
            source_hash: "abc123".to_string(),
            source_path: "/input/book.epub".to_string(),
            chapters: vec![
                babel_ebook::checkpoint::ChapterCheckpoint {
                    index: 0,
                    href: "ch01.xhtml".to_string(),
                    status: babel_ebook::checkpoint::ChapterStatus::Completed,
                    content: None,
                    error: None,
                },
                babel_ebook::checkpoint::ChapterCheckpoint {
                    index: 1,
                    href: "ch02.xhtml".to_string(),
                    status: babel_ebook::checkpoint::ChapterStatus::Failed,
                    content: None,
                    error: Some("boom".to_string()),
                },
            ],
        };
        std::fs::write(
            dir.path().join("job-1.json"),
            serde_json::to_string_pretty(&cp).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.path().join("ignore.txt"), "not a checkpoint").unwrap();

        let infos = list_checkpoints(dir.path().to_string_lossy().to_string(), None)
            .await
            .unwrap();

        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].job_id, "job-1");
        assert_eq!(infos[0].completed, 1);
        assert_eq!(infos[0].failed, 1);
        assert_eq!(infos[0].total, 2);
        assert_eq!(infos[0].pending, 0);
        assert!(!infos[0].matches_current_source);
    }

    #[tokio::test]
    async fn list_checkpoints_returns_empty_for_missing_dir() {
        let infos = list_checkpoints("/nonexistent/babel_ebook_checkpoints".to_string(), None)
            .await
            .unwrap();
        assert!(infos.is_empty());
    }
}
