//! On-disk cache for translation chunks.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Simple JSON file cache keyed by a stable hash of the source text.
#[derive(Debug, Clone)]
pub struct TranslationCache {
    dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    translation: String,
    tokens: Option<usize>,
}

impl TranslationCache {
    /// Initialize the cache, creating the directory if needed.
    ///
    /// # Panics
    ///
    /// Panics if the cache directory cannot be created.
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("failed to create cache directory {}: {e}", dir.display()));
        Self { dir }
    }

    /// Return cached translation or `None`.
    #[must_use]
    pub fn get(&self, provider: &str, text: &str) -> Option<String> {
        let path = self.entry_path(provider, text);
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        let entry: CacheEntry = serde_json::from_str(&content).ok()?;
        Some(entry.translation)
    }

    /// Store a translation in the cache.
    ///
    /// # Panics
    ///
    /// Panics if the cache entry cannot be serialized or written to disk.
    pub fn put(&self, provider: &str, text: &str, translation: &str, tokens: Option<usize>) {
        let path = self.entry_path(provider, text);
        let entry = CacheEntry {
            translation: translation.to_string(),
            tokens,
        };
        let content =
            serde_json::to_string(&entry).expect("cache entry should always serialize to JSON");
        std::fs::write(&path, content)
            .unwrap_or_else(|e| panic!("failed to write cache entry {}: {e}", path.display()));
    }

    /// Return cached translation or `None` without blocking the async runtime.
    ///
    /// This is the async counterpart of [`TranslationCache::get`] and should be
    /// used from async translation paths to avoid blocking runtime threads on
    /// filesystem I/O.
    pub async fn get_async(&self, provider: &str, text: &str) -> Option<String> {
        let path = self.entry_path(provider, text);
        let Ok(content) = tokio::fs::read_to_string(&path).await else {
            return None;
        };
        let entry: CacheEntry = serde_json::from_str(&content).ok()?;
        Some(entry.translation)
    }

    /// Store a translation in the cache without blocking the async runtime.
    ///
    /// This is the async counterpart of [`TranslationCache::put`] and should be
    /// used from async translation paths to avoid blocking runtime threads on
    /// filesystem I/O.
    ///
    /// # Panics
    ///
    /// Panics if the cache entry cannot be serialized or written to disk.
    pub async fn put_async(
        &self,
        provider: &str,
        text: &str,
        translation: &str,
        tokens: Option<usize>,
    ) {
        let path = self.entry_path(provider, text);
        let display = path.display().to_string();
        let entry = CacheEntry {
            translation: translation.to_string(),
            tokens,
        };
        let content =
            serde_json::to_string(&entry).expect("cache entry should always serialize to JSON");
        tokio::fs::write(&path, content)
            .await
            .unwrap_or_else(|e| panic!("failed to write cache entry {display}: {e}"));
    }

    /// Remove all cached entries.
    ///
    /// # Panics
    ///
    /// Panics if the cache directory cannot be read or a cache file cannot be
    /// removed.
    pub fn clear(&self) {
        let entries = std::fs::read_dir(&self.dir).unwrap_or_else(|e| {
            panic!("failed to read cache directory {}: {e}", self.dir.display())
        });
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                std::fs::remove_file(&path).unwrap_or_else(|e| {
                    panic!("failed to remove cache file {}: {e}", path.display())
                });
            }
        }
    }

    fn entry_path(&self, provider: &str, text: &str) -> PathBuf {
        self.dir.join(format!("{}.json", Self::key(provider, text)))
    }

    fn key(provider: &str, text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(provider.as_bytes());
        hasher.update(b":");
        hasher.update(text.as_bytes());
        hex::encode(hasher.finalize())
    }
}
