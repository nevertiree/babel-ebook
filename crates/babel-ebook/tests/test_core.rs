use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use tempfile::TempDir;

use babel_ebook::{
    translate_epub, write_epub, Chapter, Config, EpubBook, EpubMetadata, ProgressCallback,
    ProgressEvent, TranslateContext, Translator,
};

/// A fake translator that wraps text with a marker.
struct FakeTranslator;

/// A progress callback that records all received events.
#[derive(Default)]
struct RecordingCallback {
    events: Mutex<Vec<ProgressEvent>>,
}

impl ProgressCallback for RecordingCallback {
    fn on_progress(&self, event: ProgressEvent) {
        self.events.lock().expect("progress lock").push(event);
    }
}

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
    ) -> Result<String, babel_ebook::BabelEbookError> {
        Ok(format!("[ZH] {}", text))
    }
}

/// Creates a minimal sample EPUB in `dir` and returns its path.
fn create_sample_fixture(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("sample.epub");
    let book = EpubBook {
        metadata: EpubMetadata {
            title: Some("Sample".into()),
            language: Some("en".into()),
            identifier: Some("sample-id".into()),
        },
        chapters: vec![
            Chapter {
                href: "cover.xhtml".into(),
                title: Some("Cover".into()),
                content: br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Cover</title></head>
<body><h1>Cover</h1></body>
</html>"#
                    .to_vec(),
            },
            Chapter {
                href: "ch01.xhtml".into(),
                title: Some("Chapter 1".into()),
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

fn test_config(source: PathBuf, output: PathBuf, cache_dir: PathBuf) -> Config {
    Config {
        source,
        output,
        provider: "deepseek".into(),
        api_key: None,
        base_url: None,
        model: "deepseek-chat".into(),
        concurrency: 2,
        max_input_tokens: 4000,
        max_output_tokens: 2000,
        cache_dir,
        checkpoint_dir: std::env::temp_dir().join("test-checkpoints"),
        resume_job_id: None,
        temperature: 0.3,
        source_lang: "en".into(),
        target_lang: "zh-CN".into(),
        skip_doc_patterns: vec![
            "cover".into(),
            "titlepage".into(),
            "copyright".into(),
            "dedication".into(),
            "colophon".into(),
        ],
        translate_tags: vec!["p".into(), "h1".into(), "h2".into(), "li".into()],
        system_prompt: None,
        dry_run: false,
        verbose: false,
        provider_config: None,
        output_mode: babel_ebook::config::OutputMode::Bilingual,
        translation_scope: babel_ebook::config::TranslationScope::default(),
        style: babel_ebook::config::TranslationStyle::Default,
        chapter_prompts: std::collections::HashMap::new(),
        glossary: vec![],
        exclude_selectors: vec![],
        translate_attributes: vec![],
        preserve_classes: false,
        output_font: None,
        providers: std::collections::HashMap::new(),
        prompts: babel_ebook::config::PromptTemplates::default(),
        refine: false,
    }
}

#[tokio::test]
async fn core_translate_epub_produces_bilingual_output() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let fixture = create_sample_fixture(temp_dir.path());
    let output = temp_dir.path().join("output.epub");
    let cache_dir = temp_dir.path().join("cache");
    let config = test_config(fixture, output.clone(), cache_dir);

    translate_epub(&config, &FakeTranslator, None, None)
        .await
        .expect("translation should succeed");

    assert!(output.exists(), "output EPUB should be written");

    let book = babel_ebook::read_epub(&output).expect("read output EPUB");
    let chapter = book
        .chapters
        .iter()
        .find(|c| c.href.contains("ch01"))
        .expect("ch01 chapter should exist");
    let content = String::from_utf8_lossy(&chapter.content);

    assert!(
        content.contains("<h1 lang=\"zh-CN\">[ZH] Chapter 1</h1>"),
        "translated heading missing in: {}",
        content
    );
    assert!(
        content.contains("<h1 lang=\"en\">Chapter 1</h1>"),
        "original heading missing in: {}",
        content
    );
    assert!(
        content.contains("<p lang=\"zh-CN\">[ZH] Hello world.</p>"),
        "translated paragraph missing in: {}",
        content
    );
    assert!(
        content.contains("<p lang=\"en\">Hello world.</p>"),
        "original paragraph missing in: {}",
        content
    );

    // Verify the ToC title was translated.
    assert_eq!(
        chapter.title,
        Some("[ZH] Chapter 1".into()),
        "chapter ToC title should be translated"
    );
}

#[tokio::test]
async fn core_translate_epub_dry_run_does_not_write_output() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let fixture = create_sample_fixture(temp_dir.path());
    let output = temp_dir.path().join("dry_run.epub");
    let cache_dir = temp_dir.path().join("cache");
    let mut config = test_config(fixture, output.clone(), cache_dir);
    config.dry_run = true;

    translate_epub(&config, &FakeTranslator, None, None)
        .await
        .expect("dry run should succeed");

    assert!(!output.exists(), "dry run should not write output EPUB");
}

#[tokio::test]
async fn core_translate_epub_emits_progress_events() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let fixture = create_sample_fixture(temp_dir.path());
    let output = temp_dir.path().join("progress.epub");
    let cache_dir = temp_dir.path().join("cache");
    let config = test_config(fixture, output, cache_dir);
    let callback = RecordingCallback::default();

    translate_epub(&config, &FakeTranslator, None, Some(&callback))
        .await
        .expect("translation should succeed");

    let events: Vec<ProgressEvent> = callback.events.into_inner().expect("progress lock");

    assert!(
        matches!(
            events.as_slice(),
            [
                ProgressEvent::Started { total: 1 },
                ProgressEvent::ChapterStarted { index: 1, .. },
                ProgressEvent::ChapterFinished { index: 1, .. },
                ProgressEvent::Completed,
            ]
        ),
        "unexpected event sequence: {events:?}"
    );
}
