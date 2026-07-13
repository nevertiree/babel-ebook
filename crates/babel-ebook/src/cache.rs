//! On-disk cache for translation chunks.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Simple JSON file cache keyed by a stable hash of the source text.
#[derive(Debug, Clone)]
pub struct TranslationCache {
    dir: PathBuf,
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    translation: String,
    tokens: Option<usize>,
}

impl TranslationCache {
    /// Initialize the cache, creating the directory if needed.
    ///
    /// If the cache directory cannot be created, the cache is disabled and I/O
    /// operations become no-ops. This prevents a full translation from crashing
    /// just because the cache directory is unreachable.
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        match std::fs::create_dir_all(&dir) {
            Ok(()) => Self { dir, enabled: true },
            Err(err) => {
                tracing::error!(
                    dir = %dir.display(),
                    error = %err,
                    "failed to create cache directory; cache disabled"
                );
                Self {
                    dir,
                    enabled: false,
                }
            }
        }
    }

    /// Return cached translation or `None`.
    #[must_use]
    pub fn get(&self, provider: &str, text: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }
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
    /// Failures are logged but not propagated: the cache is a best-effort
    /// optimisation and should never break a translation.
    pub fn put(&self, provider: &str, text: &str, translation: &str, tokens: Option<usize>) {
        if !self.enabled {
            return;
        }
        let path = self.entry_path(provider, text);
        let entry = CacheEntry {
            translation: translation.to_string(),
            tokens,
        };
        let content = match serde_json::to_string(&entry) {
            Ok(value) => value,
            Err(err) => {
                tracing::error!(path = %path.display(), error = %err, "failed to serialize cache entry");
                return;
            }
        };
        if let Err(err) = std::fs::write(&path, content) {
            tracing::error!(path = %path.display(), error = %err, "failed to write cache entry");
        }
    }

    /// Return cached translation or `None` without blocking the async runtime.
    ///
    /// This is the async counterpart of [`TranslationCache::get`] and should be
    /// used from async translation paths to avoid blocking runtime threads on
    /// filesystem I/O.
    pub async fn get_async(&self, provider: &str, text: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }
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
    pub async fn put_async(
        &self,
        provider: &str,
        text: &str,
        translation: &str,
        tokens: Option<usize>,
    ) {
        if !self.enabled {
            return;
        }
        let path = self.entry_path(provider, text);
        let entry = CacheEntry {
            translation: translation.to_string(),
            tokens,
        };
        let content = match serde_json::to_string(&entry) {
            Ok(value) => value,
            Err(err) => {
                tracing::error!(path = %path.display(), error = %err, "failed to serialize cache entry");
                return;
            }
        };
        if let Err(err) = tokio::fs::write(&path, content).await {
            tracing::error!(path = %path.display(), error = %err, "failed to write cache entry");
        }
    }

    /// Remove all cached entries.
    ///
    /// Failures are logged but not propagated.
    pub fn clear(&self) {
        if !self.enabled {
            return;
        }
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(iter) => iter,
            Err(err) => {
                tracing::error!(dir = %self.dir.display(), error = %err, "failed to read cache directory");
                return;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Err(err) = std::fs::remove_file(&path) {
                    tracing::error!(path = %path.display(), error = %err, "failed to remove cache file");
                }
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
