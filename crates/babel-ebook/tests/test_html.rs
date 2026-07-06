use async_trait::async_trait;
use babel_ebook::config::OutputMode;
use babel_ebook::html::{process_document, translate_text};
use babel_ebook::{Config, TranslateContext, TranslationCache, Translator};
use std::path::PathBuf;
use tempfile::TempDir;

/// A fake translator that wraps text with a marker.
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

fn test_config() -> Config {
    Config {
        source: PathBuf::from("dummy"),
        output: PathBuf::from("dummy"),
        provider: "deepseek".into(),
        api_key: None,
        base_url: None,
        model: "deepseek-chat".into(),
        concurrency: 3,
        max_input_tokens: 4000,
        max_output_tokens: 2000,
        cache_dir: PathBuf::from(".cache"),
        temperature: 0.3,
        source_lang: "en".into(),
        target_lang: "zh-CN".into(),
        skip_doc_patterns: vec![],
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
    }
}

fn test_cache() -> (TempDir, TranslationCache) {
    let dir = TempDir::new().expect("temp dir");
    let cache = TranslationCache::new(dir.path().to_path_buf());
    (dir, cache)
}

#[tokio::test]
async fn translate_text_translates_and_caches() {
    let (_dir, cache) = test_cache();
    let config = test_config();

    let result = translate_text("hello world", &FakeTranslator, &config, &cache, "")
        .await
        .expect("translation should succeed");
    assert_eq!(result, "[ZH] hello world");

    let cached = cache.get("fake", "hello world");
    assert_eq!(cached, Some("[ZH] hello world".into()));
}

#[tokio::test]
async fn process_document_translates_headings_and_paragraphs() {
    let (_dir, cache) = test_cache();
    let config = test_config();
    let html = r#"<html><body><h1>Title</h1><p>Hello world.</p></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .expect("processing should succeed");
    let out_str = String::from_utf8(out).expect("valid UTF-8");

    assert!(
        out_str.contains("<h1 lang=\"zh-CN\">[ZH] Title</h1>"),
        "translated heading missing in: {}",
        out_str
    );
    assert!(
        out_str.contains("<h1 lang=\"en\">Title</h1>"),
        "original heading missing in: {}",
        out_str
    );
    assert!(
        out_str.contains("<p lang=\"zh-CN\">[ZH] Hello world.</p>"),
        "translated paragraph missing in: {}",
        out_str
    );
    assert!(
        out_str.contains("<p lang=\"en\">Hello world.</p>"),
        "original paragraph missing in: {}",
        out_str
    );
}

#[tokio::test]
async fn process_document_skips_code_blocks() {
    let (_dir, cache) = test_cache();
    let config = test_config();
    let html = r#"<html><body><p>Translate me.</p><pre><code>skip me</code></pre></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .expect("processing should succeed");
    let out_str = String::from_utf8(out).expect("valid UTF-8");

    assert!(
        out_str.contains("[ZH] Translate me."),
        "paragraph should be translated"
    );
    assert!(
        !out_str.contains("[ZH] skip me"),
        "code block should not be translated: {}",
        out_str
    );
}

#[tokio::test]
async fn process_document_adds_lang_attributes() {
    let (_dir, cache) = test_cache();
    let config = test_config();
    let html = r#"<html><body><p>Hello.</p></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .expect("processing should succeed");
    let out_str = String::from_utf8(out).expect("valid UTF-8");

    assert!(out_str.contains("lang=\"zh-CN\""), "target lang missing");
    assert!(out_str.contains("lang=\"en\""), "source lang missing");
}

#[tokio::test]
async fn process_document_skips_short_text_and_single_cjk_char() {
    let (_dir, cache) = test_cache();
    let config = test_config();
    // A single CJK character is 3 UTF-8 bytes but only 1 Unicode code point.
    let html = r#"<html><body><p>A</p><p>中</p></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .expect("processing should succeed");
    let out_str = String::from_utf8(out).expect("valid UTF-8");

    assert!(
        !out_str.contains("[ZH]"),
        "short text and single CJK char should not be translated: {}",
        out_str
    );
}

#[tokio::test]
async fn process_document_skips_nested_translatable_children() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    // Make both <div> and <p> translatable so the outer div is selected.
    config.translate_tags = vec![
        "p".into(),
        "h1".into(),
        "h2".into(),
        "li".into(),
        "div".into(),
    ];
    // The outer div is translatable, but it contains a translatable child <p>.
    // The outer div should be skipped; the inner <p> should be translated.
    let html = r#"<html><body><div><p>Inner paragraph.</p></div></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .expect("processing should succeed");
    let out_str = String::from_utf8(out).expect("valid UTF-8");

    assert!(
        out_str.contains("[ZH] Inner paragraph."),
        "inner paragraph should be translated: {}",
        out_str
    );
    // The outer div should not have its own translated clone.
    assert!(
        !out_str.contains("<div lang=\"zh-CN\">"),
        "outer div should not be duplicated: {}",
        out_str
    );
}

#[tokio::test]
async fn process_document_ignores_skipped_descendants_when_checking_children() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    // Make both <div> and <p> translatable.
    config.translate_tags = vec!["p".into(), "div".into()];
    // The only <p> descendant of the <div> is inside a skipped <pre>.
    // The <div> should still be translated because the skipped <p> is ignored.
    let html = r#"<html><body><div><pre><p>code</p></pre></div></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .expect("processing should succeed");
    let out_str = String::from_utf8(out).expect("valid UTF-8");

    assert!(
        out_str.contains("<div lang=\"zh-CN\">[ZH] code</div>"),
        "outer div should be translated: {}",
        out_str
    );
    assert!(
        !out_str.contains("<p lang=\"zh-CN\">"),
        "code paragraph inside pre should not be duplicated: {}",
        out_str
    );
}

#[tokio::test]
async fn process_document_handles_list_items() {
    let (_dir, cache) = test_cache();
    let config = test_config();
    let html = r#"<html><body><ul><li>Item one</li></ul></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .expect("processing should succeed");
    let out_str = String::from_utf8(out).expect("valid UTF-8");

    assert!(
        out_str.contains("class=\"bilingual-li\""),
        "li class missing in: {}",
        out_str
    );
    assert!(
        out_str.contains("<p lang=\"zh-CN\">[ZH] Item one</p>"),
        "translated li paragraph missing in: {}",
        out_str
    );
}

#[tokio::test]
async fn translation_only_replaces_original() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    config.output_mode = OutputMode::TranslationOnly;
    let html = r#"<html><body><p>Hello</p></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .unwrap();
    let s = String::from_utf8(out).unwrap();

    assert!(s.contains("[ZH] Hello"));
    assert!(!s.contains(">Hello<"));
}

#[tokio::test]
async fn exclude_selector_skips_matching_element() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    config.exclude_selectors = vec![".no-translate".into()];
    let html = r#"<html><body><p class="no-translate">Skip</p><p>Translate</p></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .unwrap();
    let s = String::from_utf8(out).unwrap();

    assert!(
        !s.contains("[ZH] Skip"),
        "excluded element should not be translated: {}",
        s
    );
    assert!(
        s.contains("[ZH] Translate"),
        "included element should be translated: {}",
        s
    );
}

#[tokio::test]
async fn translate_attributes_translates_configured_attributes() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    config.translate_tags = vec!["img".into(), "p".into()];
    config.translate_attributes = vec!["alt".into(), "title".into()];
    let html =
        r#"<html><body><img src="a.jpg" alt="Photo" title="A picture"><p>Hello</p></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .unwrap();
    let s = String::from_utf8(out).unwrap();

    assert!(
        s.contains(r#"alt="[ZH] Photo""#),
        "alt attribute should be translated: {}",
        s
    );
    assert!(
        s.contains(r#"title="[ZH] A picture""#),
        "title attribute should be translated: {}",
        s
    );
    assert!(
        s.contains(r#"src="a.jpg""#),
        "src attribute should not be translated: {}",
        s
    );
}

#[tokio::test]
async fn exclude_selector_skips_attributes() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    config.translate_tags = vec!["img".into(), "p".into()];
    config.translate_attributes = vec!["title".into()];
    config.exclude_selectors = vec![".no-translate".into()];
    let html = r#"<html><body><img class="no-translate" src="a.jpg" title="Skip this"><p>Translate</p></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .unwrap();
    let s = String::from_utf8(out).unwrap();

    assert!(
        s.contains(r#"title="Skip this""#),
        "excluded element's attribute should not be translated: {}",
        s
    );
    assert!(
        s.contains("[ZH] Translate"),
        "included element should be translated: {}",
        s
    );
}

#[tokio::test]
async fn short_attribute_values_are_not_translated() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    config.translate_tags = vec!["img".into()];
    config.translate_attributes = vec!["title".into()];
    let html = r#"<html><body><img src="a.jpg" title="x"></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .unwrap();
    let s = String::from_utf8(out).unwrap();

    assert!(
        s.contains(r#"title="x""#),
        "short attribute value should not be translated: {}",
        s
    );
    assert!(
        !s.contains(r#"title="[ZH]""#),
        "short attribute value should not have translation marker: {}",
        s
    );
}

#[tokio::test]
async fn process_document_interleaved_paragraph() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    config.output_mode = OutputMode::Interleaved;
    let html = r#"<html><body><p>Hello</p></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .expect("processing should succeed");
    let out_str = String::from_utf8(out).expect("valid UTF-8");

    let original_pos = out_str
        .find("<p lang=\"en\">Hello</p>")
        .expect("original paragraph missing");
    let translated_pos = out_str
        .find("<p lang=\"zh-CN\">[ZH] Hello</p>")
        .expect("translated paragraph missing");
    assert!(
        original_pos < translated_pos,
        "original paragraph should appear before translated in: {}",
        out_str
    );
}

#[tokio::test]
async fn translation_scope_body_false_skips_paragraphs() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    config.translation_scope.body = false;
    let html = r#"<html><body><p>Hello world.</p><h1>Title</h1></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .expect("processing should succeed");
    let out_str = String::from_utf8(out).expect("valid UTF-8");

    assert!(
        !out_str.contains("[ZH]"),
        "body text should not be translated when scope.body is false: {}",
        out_str
    );
}

#[tokio::test]
async fn translation_scope_alt_text_false_skips_alt_attributes() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    config.translate_tags = vec!["img".into()];
    config.translate_attributes = vec!["alt".into(), "title".into()];
    config.translation_scope.alt_text = false;
    let html = r#"<html><body><img src="a.jpg" alt="Photo" title="A picture"></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .unwrap();
    let s = String::from_utf8(out).unwrap();

    assert!(
        s.contains(r#"alt="Photo""#),
        "alt attribute should not be translated when scope.alt_text is false: {}",
        s
    );
    assert!(
        s.contains(r#"title="[ZH] A picture""#),
        "title attribute should still be translated: {}",
        s
    );
}

#[tokio::test]
async fn exclude_selector_protects_descendants() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    config.exclude_selectors = vec![".no-translate".into()];
    let html = r#"<html><body><div class="no-translate"><p>Inside excluded div.</p></div><p>Outside.</p></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .unwrap();
    let s = String::from_utf8(out).unwrap();

    assert!(
        !s.contains("[ZH] Inside excluded div."),
        "descendant of excluded element should not be translated: {}",
        s
    );
    assert!(
        s.contains("[ZH] Outside."),
        "non-descendant element should be translated: {}",
        s
    );
}

#[tokio::test]
async fn preserve_classes_copies_original_class() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    config.preserve_classes = true;
    let html = r#"<html><body><p class="intro">Hello</p></body></html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .unwrap();
    let s = String::from_utf8(out).unwrap();

    assert!(
        s.contains(r#"<p class="intro" lang="zh-CN">[ZH] Hello</p>"#),
        "translated paragraph should preserve original class: {}",
        s
    );
    assert!(
        s.contains(r#"<p class="intro" lang="en">Hello</p>"#),
        "original paragraph should preserve class: {}",
        s
    );
}

#[tokio::test]
async fn process_document_injects_output_font_css() {
    let (_dir, cache) = test_cache();
    let mut config = test_config();
    config.output_font = Some("'Noto Serif', serif".into());

    let html = r#"<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>T</title></head>
<body><p>Hello.</p></body>
</html>"#;

    let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, "")
        .await
        .unwrap();
    let s = String::from_utf8(out).unwrap();

    assert!(
        s.contains("body { font-family: 'Noto Serif', serif; }"),
        "font CSS should be injected: {}",
        s
    );
}
