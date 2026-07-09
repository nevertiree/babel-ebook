//! Job-level checkpoint persistence for resumable translations.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    /// # Panics
    ///
    /// Panics if the checkpoint directory cannot be created.
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| {
            panic!(
                "failed to create checkpoint directory {}: {e}",
                dir.display()
            )
        });
        Self { dir }
    }

    /// Load a checkpoint by job id if it exists.
    #[must_use]
    pub fn load(&self, job_id: &str) -> Option<Checkpoint> {
        let path = self.path(job_id);
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
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

    /// Generate a short unique job id from source path + timestamp.
    #[must_use]
    pub fn generate_job_id(source: &Path) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let input = format!("{}-{now}", source.display());
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
        let bytes = std::fs::read(path)
            .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("read source for hash: {e}")))?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
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
        let store = CheckpointStore::new(dir.path().to_path_buf());
        let cp = Checkpoint {
            job_id: "job-1".into(),
            source_hash: "hash".into(),
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
        let id_a = CheckpointStore::generate_job_id(source_a);
        assert!(!id_a.is_empty());
        assert!(id_a.chars().all(|c| c.is_ascii_hexdigit()));

        let source_b = Path::new("input/book-b.epub");
        let id_b = CheckpointStore::generate_job_id(source_b);
        assert_ne!(id_a, id_b);
        assert!(id_b.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
