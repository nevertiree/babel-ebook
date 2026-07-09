//! Tauri desktop entry point for BabelEbook.

// `warn` keeps `cargo test` passing while surfacing missing docs; Task 1.2 will
// add the documentation and switch this to `#![deny(missing_docs)]`.
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::doc_markdown)]
#![warn(clippy::nursery)]
#![warn(clippy::perf)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

mod args;
mod commands;
mod config;
mod keyring;
mod queue;
mod task;

#[cfg(not(test))]
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg(not(test))]
use crate::queue::QueueManager;

#[cfg(not(test))]
// `tauri::generate_context!()` expands to a large static struct that triggers
// `clippy::large_stack_frames`; this is a known Tauri macro behaviour.
#[allow(clippy::large_stack_frames)]
fn tauri_context() -> tauri::Context {
    tauri::generate_context!()
}

/// Run the Tauri desktop application.
///
/// # Panics
///
/// Panics if the Tauri application fails to start.
#[cfg(not(test))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let queue = QueueManager::new();
            queue.clone().spawn_worker(app.handle().clone());
            app.manage(queue);

            let mut builder =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("BabelEbook")
                    .inner_size(1100.0, 720.0)
                    .min_inner_size(800.0, 600.0);
            if let Ok(port) = std::env::var("BABEL_EBOOK_E2E_CDP_PORT") {
                builder = builder.additional_browser_args(&format!(
                    "--remote-debugging-port={port} --remote-allow-origins=*"
                ));
            }
            builder.build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::translate_epub,
            commands::get_system_locale,
            keyring::store_api_key,
            keyring::load_api_key,
            keyring::delete_api_key,
            commands::check_file_exists,
            commands::suggest_output_path,
            commands::test_connection,
            commands::get_e2e_args,
            commands::get_app_version,
            commands::get_default_prompts,
            commands::enqueue_task,
            commands::remove_task,
            commands::reorder_tasks,
            commands::cancel_task,
            commands::retry_task,
            commands::start_queue,
            commands::pause_queue,
            commands::get_queue_state,
        ])
        .run(tauri_context())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use babel_ebook::{write_epub, Chapter, EpubBook, EpubMetadata};

    use crate::args::TranslateArgs;
    use crate::commands::{get_app_version, translate_epub_internal};

    fn create_sample_fixture(dir: &std::path::Path) -> PathBuf {
        let path = dir.join("sample.epub");
        let book = EpubBook {
            metadata: EpubMetadata {
                title: Some("Sample".to_string()),
                language: Some("en".to_string()),
                identifier: Some("urn:test:sample".to_string()),
            },
            chapters: vec![
                Chapter {
                    href: "cover.xhtml".to_string(),
                    title: Some("Cover".to_string()),
                    content: br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Cover</title></head>
<body><h1>Cover</h1></body>
</html>"#
                        .to_vec(),
                },
                Chapter {
                    href: "ch01.xhtml".to_string(),
                    title: Some("Chapter 1".to_string()),
                    content: br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title></head>
<body><h1>Chapter 1</h1><p>Hello world.</p></body>
</html>"#
                        .to_vec(),
                },
            ],
            resources: vec![],
        };
        write_epub(&book, &path).expect("write sample fixture");
        path
    }

    fn sample_translate_args(source: &Path, output: &Path) -> TranslateArgs {
        TranslateArgs {
            source: source.to_string_lossy().to_string(),
            output: output.to_string_lossy().to_string(),
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
            prompts: crate::args::PromptTemplates::default(),
            refine: false,
            checkpoint_dir: ".babel_ebook_checkpoints".to_string(),
            resume: None,
        }
    }

    #[tokio::test]
    async fn test_translate_epub_dry_run() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let source = create_sample_fixture(temp_dir.path());
        let output = temp_dir.path().join("desktop_test.epub");
        let args = sample_translate_args(&source, &output);
        let result = translate_epub_internal(args, None).await;
        assert!(result.is_ok(), "{result:?}");
        let text = result.unwrap().to_lowercase();
        assert!(text.contains("estimated source tokens"));
    }

    #[test]
    fn test_get_app_version() {
        let version = get_app_version();
        assert!(!version.is_empty());
        assert!(version.contains('.'));
    }

    #[test]
    fn translate_args_validation_rejects_zero_concurrency() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let source = create_sample_fixture(temp_dir.path());
        let output = temp_dir.path().join("out.epub");
        let mut args = sample_translate_args(&source, &output);
        args.concurrency = 0;
        args.api_key = "test-key".to_string();
        let config = crate::config::build_config(&args).unwrap();
        let err = config.validate().unwrap_err();
        assert!(err
            .to_string()
            .contains("concurrency must be greater than 0"));
    }

    #[tokio::test]
    async fn translate_epub_internal_rejects_zero_concurrency() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let source = create_sample_fixture(temp_dir.path());
        let output = temp_dir.path().join("out.epub");
        let mut args = sample_translate_args(&source, &output);
        args.concurrency = 0;
        args.api_key = "test-key".to_string();
        let err = translate_epub_internal(args, None).await.unwrap_err();
        assert!(err.contains("concurrency must be greater than 0"));
    }
}
