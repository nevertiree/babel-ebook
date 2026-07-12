//! Job-level checkpoint persistence for resumable translations.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::OutputMode;
use crate::core::BabelEbookError;

/// Lifecycle status of a single chapter within a checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChapterStatus {
    /// Chapter has not been processed yet.
    Pending,
    /// Chapter was translated successfully.
    Completed,
    /// Chapter translation failed.
    Failed,
}

/// Per-chapter checkpoint entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterCheckpoint {
    /// Index of the chapter in the source book spine.
    pub index: usize,
    /// Href path of the chapter document.
    pub href: String,
    /// Current translation status.
    pub status: ChapterStatus,
    /// Translated chapter content, when available.
    pub content: Option<Vec<u8>>,
    /// Error message, when translation failed.
    pub error: Option<String>,
}

/// Full checkpoint for one translation job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique identifier for the translation job.
    pub job_id: String,
    /// Hash of the source file used to detect changes.
    pub source_hash: String,
    /// Original path to the source file.
    #[serde(default)]
    pub source_path: String,
    /// Checkpoint entries for each chapter.
    pub chapters: Vec<ChapterCheckpoint>,
}

/// On-disk JSON store for checkpoints.
#[derive(Debug, Clone)]
pub struct CheckpointStore {
    /// Directory where checkpoint files are stored.
    dir: PathBuf,
}

impl CheckpointStore {
    /// Create the store, ensuring the directory exists.
    ///
    /// # Errors
    ///
    /// Returns `BabelEbookError::Anyhow` if the checkpoint directory cannot be
    /// created.
    pub fn new(dir: PathBuf) -> Result<Self, BabelEbookError> {
        std::fs::create_dir_all(&dir).map_err(|e| {
            BabelEbookError::Anyhow(anyhow::anyhow!(
                "failed to create checkpoint directory {}: {e}",
                dir.display()
            ))
        })?;
        Ok(Self { dir })
    }

    /// Load a checkpoint by job id if it exists.
    #[must_use]
    pub fn load(&self, job_id: &str) -> Option<Checkpoint> {
        let path = self.path(job_id);
        if !path.exists() {
            return None;
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(cp) => Some(cp),
                Err(err) => {
                    tracing::warn!(job_id, error = %err, "failed to parse checkpoint; starting fresh");
                    None
                }
            },
            Err(err) => {
                tracing::warn!(job_id, error = %err, "failed to read checkpoint; starting fresh");
                None
            }
        }
    }

    /// Persist a checkpoint atomically (write temp + rename).
    pub fn save(&self, checkpoint: &Checkpoint) -> Result<(), BabelEbookError> {
        let path = self.path(&checkpoint.job_id);
        let tmp = path.with_extension("tmp");
        let content = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("serialize checkpoint: {e}")))?;
        std::fs::write(&tmp, content).map_err(|e| {
            BabelEbookError::Anyhow(anyhow::anyhow!(
                "write checkpoint tmp {}: {e}",
                tmp.display()
            ))
        })?;
        std::fs::rename(&tmp, &path).map_err(|e| {
            BabelEbookError::Anyhow(anyhow::anyhow!("rename checkpoint {}: {e}", path.display()))
        })
    }

    /// Persist a checkpoint atomically using async I/O.
    ///
    /// The sync `save` API is preserved for callers that do not need async
    /// scheduling; this method performs the same operation with
    /// `tokio::fs::write` + `tokio::fs::rename` so it does not block the
    /// runtime thread.
    pub async fn save_async(&self, checkpoint: &Checkpoint) -> Result<(), BabelEbookError> {
        let path = self.path(&checkpoint.job_id);
        let tmp = path.with_extension("tmp");
        let content = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("serialize checkpoint: {e}")))?;
        tokio::fs::write(&tmp, content).await.map_err(|e| {
            BabelEbookError::Anyhow(anyhow::anyhow!(
                "write checkpoint tmp {}: {e}",
                tmp.display()
            ))
        })?;
        tokio::fs::rename(&tmp, &path).await.map_err(|e| {
            BabelEbookError::Anyhow(anyhow::anyhow!("rename checkpoint {}: {e}", path.display()))
        })
    }

    /// Generate a short stable job id from the source path and a hash of the
    /// translation parameters.
    ///
    /// The id is deterministic for the same source path, target language,
    /// output mode, provider, and model so that repeated runs of the same book
    /// with the same settings resume the same checkpoint.
    #[must_use]
    pub fn generate_job_id(
        source: &Path,
        target_lang: &str,
        output_mode: OutputMode,
        provider: &str,
        model: &str,
    ) -> String {
        let params_hash = Self::hash_params(target_lang, output_mode, provider, model);
        let input = format!("{}|{params_hash}", source.display());
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let hash = hasher.finalize();
        hex::encode(&hash[..8])
    }

    /// Compute a short SHA-256 hex hash of the translation parameters that
    /// should affect checkpoint identity.
    fn hash_params(
        target_lang: &str,
        output_mode: OutputMode,
        provider: &str,
        model: &str,
    ) -> String {
        let input = format!("{target_lang}|{}|{provider}|{model}", output_mode.as_str());
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let hash = hasher.finalize();
        hex::encode(&hash[..8])
    }

    /// Compute a SHA-256 hex hash of the source file contents.
    ///
    /// # Errors
    ///
    /// Returns `BabelEbookError::Anyhow` if the file cannot be read.
    pub fn source_hash(path: &Path) -> Result<String, BabelEbookError> {
        let file = std::fs::File::open(path)
            .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("open source for hash: {e}")))?;
        let mut reader = std::io::BufReader::new(file);
        let mut hasher = Sha256::new();
        std::io::copy(&mut reader, &mut hasher)
            .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("hash source file: {e}")))?;
        Ok(hex::encode(hasher.finalize()))
    }

    fn path(&self, job_id: &str) -> PathBuf {
        self.dir.join(format!("{job_id}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = CheckpointStore::new(dir.path().to_path_buf()).unwrap();
        let cp = Checkpoint {
            job_id: "job-1".into(),
            source_hash: "hash".into(),
            source_path: "input/book.epub".into(),
            chapters: vec![
                ChapterCheckpoint {
                    index: 0,
                    href: "ch01.xhtml".into(),
                    status: ChapterStatus::Completed,
                    content: Some(b"<p>translated</p>".to_vec()),
                    error: None,
                },
                ChapterCheckpoint {
                    index: 1,
                    href: "ch02.xhtml".into(),
                    status: ChapterStatus::Failed,
                    content: None,
                    error: Some("api error".into()),
                },
            ],
        };
        store.save(&cp).unwrap();
        let loaded = store.load("job-1").expect("checkpoint exists");
        assert_eq!(loaded.job_id, "job-1");
        assert_eq!(loaded.chapters.len(), 2);
        assert!(loaded.chapters[0].content.is_some());
        assert_eq!(loaded.chapters[1].status, ChapterStatus::Failed);
    }

    #[test]
    fn generate_job_id_properties() {
        let source_a = Path::new("input/book-a.epub");
        let id_a = CheckpointStore::generate_job_id(
            source_a,
            "zh-CN",
            OutputMode::Bilingual,
            "deepseek",
            "deepseek-chat",
        );
        assert!(!id_a.is_empty());
        assert!(id_a.chars().all(|c| c.is_ascii_hexdigit()));

        // Same source + same params must produce the same id.
        let id_a2 = CheckpointStore::generate_job_id(
            source_a,
            "zh-CN",
            OutputMode::Bilingual,
            "deepseek",
            "deepseek-chat",
        );
        assert_eq!(id_a, id_a2);

        // Different source produces a different id.
        let source_b = Path::new("input/book-b.epub");
        let id_b = CheckpointStore::generate_job_id(
            source_b,
            "zh-CN",
            OutputMode::Bilingual,
            "deepseek",
            "deepseek-chat",
        );
        assert_ne!(id_a, id_b);
        assert!(id_b.chars().all(|c| c.is_ascii_hexdigit()));

        // Different target language produces a different id.
        let id_diff_lang = CheckpointStore::generate_job_id(
            source_a,
            "en",
            OutputMode::Bilingual,
            "deepseek",
            "deepseek-chat",
        );
        assert_ne!(id_a, id_diff_lang);

        // Different output mode produces a different id.
        let id_diff_mode = CheckpointStore::generate_job_id(
            source_a,
            "zh-CN",
            OutputMode::TranslationOnly,
            "deepseek",
            "deepseek-chat",
        );
        assert_ne!(id_a, id_diff_mode);

        // Different provider produces a different id.
        let id_diff_provider = CheckpointStore::generate_job_id(
            source_a,
            "zh-CN",
            OutputMode::Bilingual,
            "openai",
            "deepseek-chat",
        );
        assert_ne!(id_a, id_diff_provider);

        // Different model produces a different id.
        let id_diff_model = CheckpointStore::generate_job_id(
            source_a,
            "zh-CN",
            OutputMode::Bilingual,
            "deepseek",
            "deepseek-coder",
        );
        assert_ne!(id_a, id_diff_model);
    }

    #[test]
    fn generate_job_id_is_stable_across_time() {
        let source = Path::new("input/book-a.epub");
        let id1 = CheckpointStore::generate_job_id(
            source,
            "zh-CN",
            OutputMode::Bilingual,
            "deepseek",
            "deepseek-chat",
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
        let id2 = CheckpointStore::generate_job_id(
            source,
            "zh-CN",
            OutputMode::Bilingual,
            "deepseek",
            "deepseek-chat",
        );
        assert_eq!(id1, id2);
    }
}
