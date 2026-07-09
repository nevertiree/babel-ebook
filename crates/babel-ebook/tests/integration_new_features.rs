use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use tempfile::TempDir;
use zip::write::FileOptions;
use zip::CompressionMethod;

use babel_ebook::{
    checkpoint::{ChapterStatus, CheckpointStore},
    translate_epub, write_epub, Chapter, Config, EpubBook, EpubMetadata, ProgressCallback,
    ProgressEvent, TranslateContext, Translator,
};

/// A fake translator that wraps every input with a `[ZH] ` marker.
///
/// NOTE: This helper intentionally mirrors `FakeTranslator` in `tests/test_html.rs`
/// and `tests/test_core.rs` to keep this integration-test file self-contained.
struct FakeTranslator;

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

/// Records all progress events emitted during a translation run.
#[derive(Default)]
struct RecordingCallback {
    events: Mutex<Vec<ProgressEvent>>,
}

impl ProgressCallback for RecordingCallback {
    fn on_progress(&self, event: ProgressEvent) {
        self.events.lock().expect("progress lock").push(event);
    }
}

/// Returns a baseline config that points at `source` and `output`.
///
/// NOTE: This helper intentionally mirrors `test_config` in `tests/test_core.rs`
/// to keep this integration-test file self-contained.
fn test_config(
    source: PathBuf,
    output: PathBuf,
    cache_dir: PathBuf,
    checkpoint_dir: PathBuf,
) -> Config {
    Config {
        source,
        output,
        provider: "deepseek".into(),
        api_key: Some("dummy".into()),
        base_url: None,
        model: "deepseek-chat".into(),
        concurrency: 2,
        max_input_tokens: 4000,
        max_output_tokens: 2000,
        cache_dir,
        checkpoint_dir,
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
        chapter_prompts: HashMap::new(),
        glossary: vec![],
        exclude_selectors: vec![],
        translate_attributes: vec![],
        preserve_classes: false,
        output_font: None,
        providers: HashMap::new(),
        prompts: babel_ebook::config::PromptTemplates::default(),
        refine: false,
    }
}

/// Creates a minimal EPUB in `dir` with the given chapters and returns its path.
fn create_epub(dir: &Path, chapters: &[(String, Option<String>, String)]) -> PathBuf {
    let path = dir.join("input.epub");
    let book = EpubBook {
        metadata: EpubMetadata {
            title: Some("Sample".into()),
            language: Some("en".into()),
            identifier: Some("sample-id".into()),
        },
        chapters: chapters
            .iter()
            .map(|(href, title, body)| Chapter {
                href: href.clone(),
                title: title.clone(),
                content: format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{}</title></head>
<body>{}</body>
</html>"#,
                    title.as_deref().unwrap_or("Untitled"),
                    body
                )
                .into_bytes(),
            })
            .collect(),
        resources: vec![],
    };
    write_epub(&book, &path).expect("write sample fixture");
    path
}

/// Reads the chapter whose href contains `needle` from the EPUB at `path`.
fn read_chapter_content(path: &Path, needle: &str) -> String {
    let book = babel_ebook::read_epub(path).expect("read output EPUB");
    let chapter = book
        .chapters
        .iter()
        .find(|c| c.href.contains(needle))
        .unwrap_or_else(|| panic!("chapter containing {} not found", needle));
    String::from_utf8_lossy(&chapter.content).to_string()
}

/// Returns the single job id found in `checkpoint_dir`, panicking if there is not exactly one.
fn find_job_id(checkpoint_dir: &Path) -> String {
    let mut files: Vec<PathBuf> = std::fs::read_dir(checkpoint_dir)
        .expect("read checkpoint dir")
        .filter_map(|e| {
            let e = e.ok()?;
            let path = e.path();
            if path.extension()? == "json" {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(files.len(), 1, "expected exactly one checkpoint file");
    files
        .pop()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
        .expect("checkpoint filename")
}

/// Writes a minimal DOCX (zipped) containing two paragraphs to `path`.
fn write_minimal_docx(path: &Path, paragraphs: &[String]) {
    let file = std::fs::File::create(path).expect("create docx file");
    let mut zip = zip::ZipWriter::new(file);
    let stored = FileOptions::<()>::default().compression_method(CompressionMethod::Stored);

    zip.start_file("[Content_Types].xml", stored)
        .expect("start content types");
    zip.write_all(CONTENT_TYPES_XML.as_bytes())
        .expect("write content types");

    zip.start_file("_rels/.rels", stored)
        .expect("start package rels");
    zip.write_all(PACKAGE_RELS_XML.as_bytes())
        .expect("write package rels");

    zip.start_file("word/_rels/document.xml.rels", stored)
        .expect("start document rels");
    zip.write_all(DOCUMENT_RELS_XML.as_bytes())
        .expect("write document rels");

    zip.start_file("word/document.xml", stored)
        .expect("start document");
    zip.write_all(document_xml(paragraphs).as_bytes())
        .expect("write document");

    zip.finish().expect("finish docx archive");
}

const CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>
"#;

const PACKAGE_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
"#;

const DOCUMENT_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
</Relationships>
"#;

fn document_xml(paragraphs: &[String]) -> String {
    let body = paragraphs
        .iter()
        .map(|text| {
            format!(
                "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                html_escape(text)
            )
        })
        .collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    {}
    <w:sectPr/>
  </w:body>
</w:document>
"#,
        body
    )
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[tokio::test]
async fn resume_skips_completed_chapters() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let checkpoint_dir = temp_dir.path().join("checkpoints");
    let cache_dir = temp_dir.path().join("cache");

    let source = create_epub(
        temp_dir.path(),
        &[
            ("cover.xhtml".into(), None, "<h1>Cover</h1>".into()),
            (
                "ch01.xhtml".into(),
                None,
                "<p>Hello from chapter one.</p>".into(),
            ),
            (
                "ch02.xhtml".into(),
                None,
                "<p>Hello from chapter two.</p>".into(),
            ),
        ],
    );

    struct FailingTranslator {
        counter: AtomicUsize,
    }

    #[async_trait]
    impl Translator for FailingTranslator {
        fn name(&self) -> String {
            "failing".into()
        }

        fn max_output_tokens(&self) -> usize {
            2000
        }

        async fn translate(
            &self,
            text: &str,
            _context: &TranslateContext<'_>,
        ) -> Result<String, babel_ebook::BabelEbookError> {
            let count = self.counter.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                Ok(format!("[ZH] {}", text))
            } else {
                Err(babel_ebook::BabelEbookError::ApiError(
                    "forced failure".into(),
                ))
            }
        }
    }

    let output1 = temp_dir.path().join("output1.epub");
    let config1 = {
        let mut c = test_config(
            source.clone(),
            output1.clone(),
            cache_dir.clone(),
            checkpoint_dir.clone(),
        );
        c.concurrency = 1;
        c.translation_scope.toc = false;
        c
    };

    translate_epub(
        &config1,
        &FailingTranslator {
            counter: AtomicUsize::new(0),
        },
        None,
        None,
    )
    .await
    .expect("first run should complete without error");

    let ch01_first = read_chapter_content(&output1, "ch01");
    let ch02_first = read_chapter_content(&output1, "ch02");
    assert!(
        ch01_first.contains("<p lang=\"zh-CN\">[ZH] Hello from chapter one.</p>"),
        "chapter 1 should be translated in first run: {}",
        ch01_first
    );
    assert!(
        !ch02_first.contains("[ZH]"),
        "chapter 2 should not be translated in first run: {}",
        ch02_first
    );

    let job_id = find_job_id(&checkpoint_dir);
    let store = CheckpointStore::new(checkpoint_dir.clone()).unwrap();
    let checkpoint = store
        .load(&job_id)
        .expect("checkpoint should exist after first run");
    assert_eq!(checkpoint.chapters.len(), 2);
    assert_eq!(checkpoint.chapters[0].status, ChapterStatus::Completed);
    assert_eq!(checkpoint.chapters[1].status, ChapterStatus::Failed);

    let output2 = temp_dir.path().join("output2.epub");
    let mut config2 = test_config(source, output2.clone(), cache_dir, checkpoint_dir.clone());
    config2.concurrency = 1;
    config2.translation_scope.toc = false;
    config2.resume_job_id = Some(job_id.clone());

    translate_epub(&config2, &FakeTranslator, None, None)
        .await
        .expect("second run should complete without error");

    let ch01_second = read_chapter_content(&output2, "ch01");
    let ch02_second = read_chapter_content(&output2, "ch02");
    assert!(
        ch01_second.contains("<p lang=\"zh-CN\">[ZH] Hello from chapter one.</p>"),
        "chapter 1 should be restored from the checkpoint: {}",
        ch01_second
    );
    assert_eq!(
        ch01_first, ch01_second,
        "chapter 1 should match the first run exactly"
    );
    assert!(
        ch02_second.contains("<p lang=\"zh-CN\">[ZH] Hello from chapter two.</p>"),
        "chapter 2 should be translated in second run: {}",
        ch02_second
    );

    let checkpoint = store.load(&job_id).expect("checkpoint should still exist");
    assert_eq!(checkpoint.chapters.len(), 2);
    assert!(checkpoint
        .chapters
        .iter()
        .all(|c| c.status == ChapterStatus::Completed));
}

#[tokio::test]
async fn srt_input_translates_to_epub() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let source = temp_dir.path().join("subtitles.srt");
    std::fs::write(
        &source,
        r#"1
00:00:01,000 --> 00:00:04,000
Hello world

2
00:00:05,000 --> 00:00:07,000
Second cue
"#,
    )
    .expect("write srt");

    let output = temp_dir.path().join("output.epub");
    let config = test_config(
        source,
        output.clone(),
        temp_dir.path().join("cache"),
        temp_dir.path().join("checkpoints"),
    );

    translate_epub(&config, &FakeTranslator, None, None)
        .await
        .expect("translation should succeed");

    let content = read_chapter_content(&output, "chapter");
    assert!(
        content.contains("<p lang=\"zh-CN\">[ZH] Hello world</p>"),
        "first cue should be translated: {}",
        content
    );
    assert!(
        content.contains("<p lang=\"en\">Hello world</p>"),
        "first cue original should be preserved: {}",
        content
    );
    assert!(
        content.contains("<p lang=\"zh-CN\">[ZH] Second cue</p>"),
        "second cue should be translated: {}",
        content
    );
    assert!(
        content.contains("<p lang=\"en\">Second cue</p>"),
        "second cue original should be preserved: {}",
        content
    );
}

#[tokio::test]
async fn txt_input_translates_to_epub() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let source = temp_dir.path().join("story.txt");
    std::fs::write(&source, "Line one\nLine two\n").expect("write txt");

    let output = temp_dir.path().join("output.epub");
    let config = test_config(
        source,
        output.clone(),
        temp_dir.path().join("cache"),
        temp_dir.path().join("checkpoints"),
    );

    translate_epub(&config, &FakeTranslator, None, None)
        .await
        .expect("translation should succeed");

    let content = read_chapter_content(&output, "chapter");
    assert!(
        content.contains("<p lang=\"zh-CN\">[ZH] Line one</p>"),
        "first line should be translated: {}",
        content
    );
    assert!(
        content.contains("<p lang=\"en\">Line one</p>"),
        "first line original should be preserved: {}",
        content
    );
    assert!(
        content.contains("<p lang=\"zh-CN\">[ZH] Line two</p>"),
        "second line should be translated: {}",
        content
    );
    assert!(
        content.contains("<p lang=\"en\">Line two</p>"),
        "second line original should be preserved: {}",
        content
    );
}

#[tokio::test]
async fn docx_input_translates_to_epub() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let source = temp_dir.path().join("document.docx");
    write_minimal_docx(
        &source,
        &["Hello from DOCX".into(), "Second paragraph".into()],
    );

    let output = temp_dir.path().join("output.epub");
    let config = test_config(
        source,
        output.clone(),
        temp_dir.path().join("cache"),
        temp_dir.path().join("checkpoints"),
    );

    translate_epub(&config, &FakeTranslator, None, None)
        .await
        .expect("translation should succeed");

    let content = read_chapter_content(&output, "chapter");
    assert!(
        content.contains("<p lang=\"zh-CN\">[ZH] Hello from DOCX</p>"),
        "first paragraph should be translated: {}",
        content
    );
    assert!(
        content.contains("<p lang=\"en\">Hello from DOCX</p>"),
        "first paragraph original should be preserved: {}",
        content
    );
    assert!(
        content.contains("<p lang=\"zh-CN\">[ZH] Second paragraph</p>"),
        "second paragraph should be translated: {}",
        content
    );
    assert!(
        content.contains("<p lang=\"en\">Second paragraph</p>"),
        "second paragraph original should be preserved: {}",
        content
    );
}

#[tokio::test]
async fn ordered_pipeline_emits_events_in_order() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let source = create_epub(
        temp_dir.path(),
        &[
            ("ch01.xhtml".into(), None, "<p>Alpha</p>".into()),
            ("ch02.xhtml".into(), None, "<p>Bravo</p>".into()),
            ("ch03.xhtml".into(), None, "<p>Charlie</p>".into()),
        ],
    );

    let output = temp_dir.path().join("output.epub");
    let mut config = test_config(
        source,
        output,
        temp_dir.path().join("cache"),
        temp_dir.path().join("checkpoints"),
    );
    config.concurrency = 3;
    config.translation_scope.toc = false;

    let callback = RecordingCallback::default();
    translate_epub(&config, &FakeTranslator, None, Some(&callback))
        .await
        .expect("translation should succeed");

    let events: Vec<ProgressEvent> = callback.events.into_inner().expect("progress lock");
    let finished_indices: Vec<usize> = events
        .iter()
        .filter_map(|e| match e {
            ProgressEvent::ChapterFinished { index, .. } => Some(*index),
            _ => None,
        })
        .collect();
    assert_eq!(
        finished_indices,
        vec![0, 1, 2],
        "chapter finished events should follow spine order: {:?}",
        events
    );
}

#[tokio::test]
async fn refine_pass_runs_second_pass() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let source = create_epub(
        temp_dir.path(),
        &[("ch01.xhtml".into(), None, "<p>Hello world</p>".into())],
    );

    let output = temp_dir.path().join("output.epub");
    let mut config = test_config(
        source,
        output.clone(),
        temp_dir.path().join("cache"),
        temp_dir.path().join("checkpoints"),
    );
    config.refine = true;

    translate_epub(&config, &FakeTranslator, None, None)
        .await
        .expect("translation should succeed");

    let content = read_chapter_content(&output, "ch01");
    assert!(
        content.contains("<p lang=\"zh-CN\">[ZH] [ZH] Hello world</p>"),
        "refine pass should translate twice: {}",
        content
    );
}
