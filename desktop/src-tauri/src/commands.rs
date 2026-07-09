//! Tauri commands exposed to the desktop frontend.

use babel_ebook::{
    estimate_source_tokens, read_input_book, translatable_chapters,
    translate_epub as translate_epub_core, translator::get_translator, ProgressCallback,
    ProgressEvent, ProviderConfig, TranslationCache, KNOWN_PROVIDERS,
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
            source_path: String::new(),
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
use crate::args::{TestConnectionArgs, TranslateArgs};
use crate::config::{build_config, build_test_config};
use crate::queue::{QueueManager, QueueState};
use crate::task::Task;

/// Read E2E injection values from the environment.
///
/// Supported variables: `BABEL_EBOOK_E2E_SOURCE`, `BABEL_EBOOK_E2E_OUTPUT`,
/// `BABEL_EBOOK_E2E_API_KEY`, `BABEL_EBOOK_E2E_DRY_RUN`,
/// `BABEL_EBOOK_E2E_UI_LANGUAGE`.
#[cfg(not(test))]
#[tauri::command]
pub fn get_e2e_args() -> E2EArgs {
    let source = std::env::var("BABEL_EBOOK_E2E_SOURCE").ok();
    let output = std::env::var("BABEL_EBOOK_E2E_OUTPUT").ok();
    let api_key = std::env::var("BABEL_EBOOK_E2E_API_KEY").ok();
    let dry_run = std::env::var("BABEL_EBOOK_E2E_DRY_RUN")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let ui_language = std::env::var("BABEL_EBOOK_E2E_UI_LANGUAGE").ok();
    E2EArgs {
        source,
        output,
        api_key,
        dry_run,
        ui_language,
    }
}

#[cfg(not(test))]
pub struct WindowProgressCallback(tauri::Window);

#[cfg(not(test))]
impl ProgressCallback for WindowProgressCallback {
    fn on_progress(&self, event: ProgressEvent) {
        let _ = self.0.emit("translation_progress", event);
    }
}

/// Tauri command that translates an EPUB according to the provided arguments.
#[cfg(not(test))]
#[tauri::command]
pub async fn translate_epub(args: TranslateArgs, window: tauri::Window) -> Result<String, String> {
    let progress: Option<Box<dyn ProgressCallback + Send + Sync>> =
        Some(Box::new(WindowProgressCallback(window)));
    translate_epub_internal(args, progress).await
}

pub async fn translate_epub_internal(
    args: TranslateArgs,
    progress: Option<Box<dyn ProgressCallback + Send + Sync>>,
) -> Result<String, String> {
    // The core translator uses `kuchiki`, whose `Rc`-based DOM is `!Send`.
    // Tauri async commands must return a `Send` future, so run the core work
    // on a blocking thread with a local current-thread Tokio runtime.
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;

        rt.block_on(async {
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

            let translator = get_translator(
                &config.provider,
                config.provider_config.as_ref(),
                &config,
                config.dry_run,
            )
            .map_err(|e| e.to_string())?;

            if config.dry_run {
                let book = read_input_book(&config.source).map_err(|e| e.to_string())?;
                let indices = translatable_chapters(&book, &config.skip_doc_patterns)
                    .map_err(|e| e.to_string())?;
                let (tokens, docs) = estimate_source_tokens(&book, &indices);
                if let Some(p) = progress_ref {
                    p.on_progress(ProgressEvent::Completed);
                }
                return Ok(format!(
                    "Estimated source tokens: {tokens} ({docs} documents)"
                ));
            }

            let cache = TranslationCache::new(config.cache_dir.clone());

            translate_epub_core(&config, translator.as_ref(), Some(&cache), progress_ref)
                .await
                .map_err(|e| e.to_string())?;

            tracing::info!(
                output = %config.output.display(),
                "translation command completed successfully"
            );
            Ok(format!(
                "Translation completed: {}",
                config.output.display()
            ))
        })
    })
    .await
    .map_err(|e| format!("translation task panicked: {e}"))?
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

    format!("{parent}/{rendered}.epub")
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
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;

        rt.block_on(async {
            let config = build_test_config(&args)?;
            let mut provider_config = ProviderConfig::for_provider(&args.provider);
            provider_config.base_url = args.base_url.clone().filter(|url| !url.is_empty());

            let translator = get_translator(&args.provider, Some(&provider_config), &config, false)
                .map_err(|e| e.to_string())?;

            translator.health_check().await.map_err(|e| e.to_string())
        })
    })
    .await
    .map_err(|e| format!("health check task panicked: {e}"))?
    .map(|()| "connection ok".to_string())
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
    Ok(queue.enqueue(args).await)
}

/// Remove a pending or finished task from the queue.
#[allow(dead_code)]
#[tauri::command]
pub async fn remove_task(id: String, queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.remove(&id).await
}

/// Reorder pending tasks to match the provided list of ids.
#[allow(dead_code)]
#[tauri::command]
pub async fn reorder_tasks(
    ids: Vec<String>,
    queue: tauri::State<'_, QueueManager>,
) -> Result<(), String> {
    queue.reorder(ids).await
}

/// Cancel a pending task.
#[allow(dead_code)]
#[tauri::command]
pub async fn cancel_task(id: String, queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.cancel(&id).await
}

/// Retry a failed or cancelled task.
#[allow(dead_code)]
#[tauri::command]
pub async fn retry_task(id: String, queue: tauri::State<'_, QueueManager>) -> Result<(), String> {
    queue.retry(&id).await
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

/// Return the current queue state.
#[allow(dead_code)]
#[tauri::command]
pub async fn get_queue_state(queue: tauri::State<'_, QueueManager>) -> Result<QueueState, String> {
    Ok(queue.state().await)
}

/// List translation checkpoints stored in `checkpoint_dir`.
#[allow(dead_code)]
#[tauri::command]
pub async fn list_checkpoints(checkpoint_dir: String) -> Result<Vec<CheckpointInfo>, String> {
    let dir = std::path::PathBuf::from(checkpoint_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }

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
            checkpoints.push(info);
        }
        Ok::<Vec<CheckpointInfo>, String>(checkpoints)
    })
    .await
    .map_err(|e| format!("checkpoint listing task panicked: {e}"))??;

    entries.sort_by(|a, b| a.job_id.cmp(&b.job_id));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use crate::args::TestConnectionArgs;
    use crate::commands::validate_connection_args;

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
}
