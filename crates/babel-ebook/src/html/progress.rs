//! Progress-callback adaptation for chapter-level chunk tracking.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::core::{CancellationToken, ProgressCallback, ProgressEvent};

/// Progress adapter that turns per-text-block chunk events into chapter-global
/// chunk events.
///
/// `process_document` first counts how many first-pass source chunks the whole
/// chapter will produce. While translating, each element calls `translate_text`,
/// which still emits chunk events relative to that single text block. This
/// adapter intercepts those events and rewrites `chunk_index`/`chunk_total` so
/// the progress bar moves smoothly across elements instead of resetting for
/// every paragraph or heading.
pub struct ChapterChunkAdapter<'a> {
    inner: Option<&'a dyn ProgressCallback>,
    chapter_index: usize,
    href: String,
    total_chunks: usize,
    next_chunk: AtomicUsize,
    cancellation: Option<&'a CancellationToken>,
}

impl<'a> ChapterChunkAdapter<'a> {
    /// Create a new adapter that maps per-block chunk events onto a chapter-global
    /// progress bar.
    pub fn new(
        inner: Option<&'a dyn ProgressCallback>,
        chapter_index: usize,
        href: String,
        total_chunks: usize,
        cancellation: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            inner,
            chapter_index,
            href,
            total_chunks,
            next_chunk: AtomicUsize::new(0),
            cancellation,
        }
    }

    /// Return the cancellation token held by this adapter, if any.
    pub const fn cancellation(&self) -> Option<&'a CancellationToken> {
        self.cancellation
    }
}

impl ProgressCallback for ChapterChunkAdapter<'_> {
    fn on_progress(&self, event: ProgressEvent) {
        match event {
            ProgressEvent::ChunkStarted { .. } => {
                let chunk_index = self.next_chunk.load(Ordering::SeqCst);
                if let Some(inner) = self.inner {
                    inner.on_progress(ProgressEvent::ChunkStarted {
                        index: self.chapter_index,
                        href: self.href.clone(),
                        chunk_index,
                        chunk_total: self.total_chunks,
                    });
                }
            }
            ProgressEvent::ChunkFinished { .. } => {
                let chunk_index = self.next_chunk.fetch_add(1, Ordering::SeqCst);
                if let Some(inner) = self.inner {
                    inner.on_progress(ProgressEvent::ChunkFinished {
                        index: self.chapter_index,
                        href: self.href.clone(),
                        chunk_index,
                        chunk_total: self.total_chunks,
                    });
                }
            }
            other => {
                if let Some(inner) = self.inner {
                    inner.on_progress(other);
                }
            }
        }
    }
}

/// Emit a chunk-level progress event if a callback is registered.
pub fn emit_chunk_progress(
    progress: Option<&dyn ProgressCallback>,
    index: usize,
    href: &str,
    chunk_index: usize,
    chunk_total: usize,
    finished: bool,
) {
    let event = if finished {
        ProgressEvent::ChunkFinished {
            index,
            href: href.to_string(),
            chunk_index,
            chunk_total,
        }
    } else {
        ProgressEvent::ChunkStarted {
            index,
            href: href.to_string(),
            chunk_index,
            chunk_total,
        }
    };
    if let Some(p) = progress {
        p.on_progress(event);
    }
}
