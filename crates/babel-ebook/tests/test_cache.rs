use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use babel_ebook::TranslationCache;

fn only_json_file(dir: &Path) -> std::path::PathBuf {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .expect("read dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    assert_eq!(files.len(), 1, "expected exactly one JSON cache file");
    files.pop().unwrap()
}

#[test]
fn roundtrip_stores_and_retrieves_translation() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let cache = TranslationCache::new(dir.path().to_path_buf());

    assert!(cache.get("deepseek", "hello").is_none());

    cache.put("deepseek", "hello", "你好", Some(42));
    assert_eq!(cache.get("deepseek", "hello").as_deref(), Some("你好"));
}

#[test]
fn provider_isolation() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let cache = TranslationCache::new(dir.path().to_path_buf());

    cache.put("deepseek", "hello", "你好", None);
    cache.put("openai", "hello", "您好", None);

    assert_eq!(cache.get("deepseek", "hello").as_deref(), Some("你好"));
    assert_eq!(cache.get("openai", "hello").as_deref(), Some("您好"));
}

#[test]
fn clear_removes_all_entries() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let cache = TranslationCache::new(dir.path().to_path_buf());

    cache.put("deepseek", "a", "A", None);
    cache.put("deepseek", "b", "B", None);

    cache.clear();
    assert!(cache.get("deepseek", "a").is_none());
    assert!(cache.get("deepseek", "b").is_none());
}

#[test]
fn missing_file_returns_none() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let cache = TranslationCache::new(dir.path().to_path_buf());

    assert!(cache.get("deepseek", "not-cached").is_none());
}

#[test]
fn malformed_json_returns_none() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let cache = TranslationCache::new(dir.path().to_path_buf());

    cache.put("deepseek", "bad", "valid", None);
    let path = only_json_file(dir.path());
    let mut file = std::fs::File::create(&path).expect("create file");
    file.write_all(b"not json").expect("write file");

    assert!(cache.get("deepseek", "bad").is_none());
}

#[test]
fn missing_translation_field_returns_none() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let cache = TranslationCache::new(dir.path().to_path_buf());

    cache.put("deepseek", "missing", "valid", None);
    let path = only_json_file(dir.path());
    let mut file = std::fs::File::create(&path).expect("create file");
    file.write_all(br#"{"tokens": 1}"#).expect("write file");

    assert!(cache.get("deepseek", "missing").is_none());
}

#[tokio::test]
async fn async_roundtrip_stores_and_retrieves_translation() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let cache = TranslationCache::new(dir.path().to_path_buf());

    assert!(cache.get_async("deepseek", "hello").await.is_none());

    cache.put_async("deepseek", "hello", "你好", Some(42)).await;

    assert_eq!(
        cache.get_async("deepseek", "hello").await.as_deref(),
        Some("你好")
    );
    // Async writes must be visible to synchronous readers as well.
    assert_eq!(cache.get("deepseek", "hello").as_deref(), Some("你好"));
}

#[tokio::test]
async fn concurrent_async_cache_reads_and_writes() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let cache = Arc::new(TranslationCache::new(dir.path().to_path_buf()));
    let task_count = 50;

    let mut handles = Vec::with_capacity(task_count);
    for i in 0..task_count {
        let cache = Arc::clone(&cache);
        let text = format!("text-{i}");
        let translation = format!("translation-{i}");
        handles.push(tokio::spawn(async move {
            cache.put_async("deepseek", &text, &translation, None).await;
            let read_back = cache
                .get_async("deepseek", &text)
                .await
                .expect("concurrent read should find written entry");
            assert_eq!(read_back, translation);
        }));
    }

    for handle in handles {
        handle.await.expect("concurrent cache task should succeed");
    }
}
