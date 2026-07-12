//! Dedicated worker thread for running `!Send` translation work.
//!
//! `kuchiki` uses `Rc` internally, so the translation future returned by
//! [`crate::translate_epub_with_cancellation`] is `!Send`. This worker isolates
//! that work on a single dedicated thread with a long-lived current-thread
//! Tokio runtime, avoiding the creation of a fresh runtime per translation.

use std::thread::{Builder, JoinHandle};

use tokio::runtime;
use tokio::sync::{mpsc, oneshot};

use crate::cache::TranslationCache;
use crate::config::Config;
use crate::core::{BabelEbookError, CancellationToken, ProgressCallback, ProgressEvent};
use crate::translator::Translator;

/// Error returned when a worker operation fails.
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    /// The worker thread failed to start.
    #[error("failed to spawn worker thread: {0}")]
    Spawn(#[from] std::io::Error),
    /// The worker is no longer running.
    #[error("worker is shut down")]
    ShutDown,
}

/// A translation job submitted to the worker.
pub struct TranslationJob {
    /// Runtime configuration for the translation.
    pub config: Config,
    /// Translator to use for the job.
    pub translator: Box<dyn Translator + Send + Sync>,
    /// Optional translation cache.
    pub cache: Option<TranslationCache>,
    /// Optional cooperative cancellation token.
    pub cancellation: Option<CancellationToken>,
}

impl TranslationJob {
    /// Create a new translation job.
    #[must_use]
    pub fn new(config: Config, translator: Box<dyn Translator + Send + Sync>) -> Self {
        Self {
            config,
            translator,
            cache: None,
            cancellation: None,
        }
    }

    /// Attach a cache to the job.
    #[must_use]
    pub fn with_cache(mut self, cache: TranslationCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Attach a cancellation token to the job.
    #[must_use]
    pub fn with_cancellation(mut self, cancellation: CancellationToken) -> Self {
        self.cancellation = Some(cancellation);
        self
    }
}

/// Internal command sent to the worker thread.
#[allow(clippy::large_enum_variant)]
enum WorkerCommand {
    /// Run a translation job and report the result.
    Run {
        /// Job to execute.
        job: Box<TranslationJob>,
        /// Channel used to send the final result back to the caller.
        result_tx: oneshot::Sender<Result<(), BabelEbookError>>,
        /// Channel used to stream progress events back to the caller.
        progress_tx: mpsc::UnboundedSender<ProgressEvent>,
    },
    /// Stop accepting new jobs and exit the worker thread.
    Shutdown,
}

/// Handle returned by [`TranslationWorker::submit`].
///
/// Callers can await [`Self::result`] to obtain the final outcome and read
/// progress events from [`Self::progress`] while the job is running.
pub struct TranslationJobHandle {
    /// Receiver for progress events emitted by the job.
    pub progress: mpsc::UnboundedReceiver<ProgressEvent>,
    /// Receiver for the job's final result.
    pub result_rx: oneshot::Receiver<Result<(), BabelEbookError>>,
}

impl std::fmt::Debug for TranslationJobHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranslationJobHandle")
            .finish_non_exhaustive()
    }
}

impl TranslationJobHandle {
    /// Wait for the job to finish and return its result.
    pub async fn result(self) -> Result<(), BabelEbookError> {
        self.result_rx.await.unwrap_or_else(|_| {
            Err(BabelEbookError::Anyhow(anyhow::anyhow!(
                "translation worker terminated unexpectedly"
            )))
        })
    }
}

/// Progress callback that forwards events through an unbounded channel.
struct ProgressSender(mpsc::UnboundedSender<ProgressEvent>);

impl ProgressCallback for ProgressSender {
    fn on_progress(&self, event: ProgressEvent) {
        let _ = self.0.send(event);
    }
}

/// A dedicated worker thread that runs `!Send` translation futures.
///
/// The worker owns a single `std::thread` with a long-lived current-thread
/// Tokio runtime inside. Jobs submitted via [`Self::submit`] are executed as
/// `!Send` local tasks on that runtime. The worker can be shut down cleanly
/// with [`Self::shutdown`]; dropping the worker also requests shutdown.
pub struct TranslationWorker {
    /// Command channel to the worker thread.
    tx: mpsc::UnboundedSender<WorkerCommand>,
    /// Token cancelled when the worker is asked to shut down.
    worker_token: CancellationToken,
    /// Handle to the dedicated worker thread.
    thread: Option<JoinHandle<()>>,
}

impl TranslationWorker {
    /// Spawn a new worker thread with a long-lived current-thread Tokio runtime.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerError::Spawn`] if the OS thread cannot be created.
    ///
    /// # Panics
    ///
    /// Panics if the worker fails to build its internal Tokio runtime. This
    /// should not happen in practice because the builder is configured with
    /// standard features.
    pub fn new() -> Result<Self, WorkerError> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let worker_token = CancellationToken::default();
        let worker_token_clone = worker_token.clone();

        let thread = Builder::new()
            .name("babel-ebook-translator".into())
            .spawn(move || {
                let rt = runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build worker tokio runtime");

                let local = tokio::task::LocalSet::new();
                rt.block_on(local.run_until(async move {
                    while let Some(cmd) = rx.recv().await {
                        match cmd {
                            WorkerCommand::Run {
                                job,
                                result_tx,
                                progress_tx,
                            } => {
                                if worker_token_clone.is_cancelled() {
                                    let _ = result_tx.send(Err(BabelEbookError::Cancelled));
                                    continue;
                                }

                                // Use a job-local token so that worker shutdown
                                // can cancel a job even when the caller did not
                                // supply a token.
                                let job_token = job.cancellation.clone().unwrap_or_default();
                                let job_token_for_translate = job_token.clone();
                                let progress = ProgressSender(progress_tx);
                                let worker_token = worker_token_clone.clone();

                                tokio::task::spawn_local(async move {
                                    // Monitor worker shutdown and propagate it
                                    // to the job token without busy-waiting.
                                    let monitor = tokio::task::spawn_local(async move {
                                        tokio::select! {
                                            () = worker_token.cancelled() => job_token.cancel(),
                                            () = job_token.cancelled() => {}
                                        }
                                    });

                                    let result = crate::translate_epub_with_cancellation(
                                        &job.config,
                                        job.translator.as_ref(),
                                        job.cache.as_ref(),
                                        Some(&progress),
                                        Some(&job_token_for_translate),
                                    )
                                    .await;

                                    monitor.abort();
                                    let _ = result_tx.send(result);
                                });
                            }
                            WorkerCommand::Shutdown => break,
                        }
                    }
                }));
            })?;

        Ok(Self {
            tx,
            worker_token,
            thread: Some(thread),
        })
    }

    /// Submit a translation job to the worker.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerError::ShutDown`] if the worker thread has already
    /// exited.
    #[allow(clippy::unused_async)]
    pub async fn submit(&self, job: TranslationJob) -> Result<TranslationJobHandle, WorkerError> {
        let (result_tx, result_rx) = oneshot::channel();
        let (progress_tx, progress_rx) = mpsc::unbounded_channel();
        self.tx
            .send(WorkerCommand::Run {
                job: Box::new(job),
                result_tx,
                progress_tx,
            })
            .map_err(|_| WorkerError::ShutDown)?;
        Ok(TranslationJobHandle {
            progress: progress_rx,
            result_rx,
        })
    }

    /// Request a clean shutdown of the worker.
    ///
    /// This cancels any in-flight job and waits for the worker thread to
    /// finish. Calling this multiple times is a no-op after the first call.
    pub fn shutdown(&mut self) {
        self.worker_token.cancel();
        let _ = self.tx.send(WorkerCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    fn shutdown_from_drop(&mut self) {
        self.worker_token.cancel();
        let _ = self.tx.send(WorkerCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for TranslationWorker {
    fn drop(&mut self) {
        self.shutdown_from_drop();
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::path::PathBuf;
    use std::sync::Arc;

    use tokio::sync::Notify;

    use super::*;
    use crate::epub::{write_epub, Chapter, EpubBook, EpubMetadata};
    use crate::translator::{TranslateContext, Translator};
    use async_trait::async_trait;

    fn make_config(source: PathBuf, output: PathBuf) -> Config {
        Config {
            source,
            output,
            provider: "dummy".into(),
            api_key: None,
            base_url: None,
            model: "dummy".into(),
            concurrency: 1,
            max_input_tokens: 4000,
            max_output_tokens: 2000,
            cache_dir: std::env::temp_dir().join(format!("test-cache-{}", std::process::id())),
            checkpoint_dir: std::env::temp_dir()
                .join(format!("test-checkpoint-{}", std::process::id())),
            resume_job_id: None,
            temperature: 0.3,
            source_lang: "en".into(),
            target_lang: "zh-CN".into(),
            skip_doc_patterns: vec![],
            translate_tags: vec!["p".into()],
            system_prompt: None,
            dry_run: false,
            verbose: false,
            provider_config: None,
            providers: std::collections::HashMap::default(),
            output_mode: crate::config::OutputMode::TranslationOnly,
            translation_scope: crate::config::TranslationScope::default(),
            style: crate::config::TranslationStyle::default(),
            chapter_prompts: std::collections::HashMap::default(),
            prompts: crate::config::PromptTemplates::default(),
            glossary: vec![],
            exclude_selectors: vec![],
            translate_attributes: vec![],
            preserve_classes: false,
            output_font: None,
            refine: false,
        }
    }

    fn write_test_epub(dir: &std::path::Path, name: &str, paragraphs: &[&str]) -> PathBuf {
        let path = dir.join(name);
        let mut content = String::new();
        for paragraph in paragraphs {
            let _ = write!(content, "<p>{paragraph}</p>");
        }
        let book = EpubBook {
            metadata: EpubMetadata::default(),
            chapters: vec![Chapter {
                href: "ch00.xhtml".into(),
                title: None,
                content: format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<body>{content}</body>
</html>"#
                )
                .into_bytes(),
            }],
            resources: vec![],
        };
        write_epub(&book, &path).expect("write test epub");
        path
    }

    struct DummyTranslator;

    #[async_trait]
    impl Translator for DummyTranslator {
        fn name(&self) -> String {
            "dummy".into()
        }

        fn max_output_tokens(&self) -> usize {
            1000
        }

        async fn translate(
            &self,
            text: &str,
            _ctx: &TranslateContext<'_>,
        ) -> Result<String, BabelEbookError> {
            Ok(format!("[{}]", text.trim()))
        }
    }

    fn collect_progress(rx: &mut mpsc::UnboundedReceiver<ProgressEvent>) -> Vec<ProgressEvent> {
        let mut events = Vec::new();
        // Drain events until no more are immediately available.
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        events
    }

    #[tokio::test]
    async fn worker_runs_dry_run_translation() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let source = write_test_epub(dir.path(), "dry-run.epub", &["Hello world."]);
        let output = dir.path().join("dry-run-out.epub");
        let mut config = make_config(source, output);
        config.dry_run = true;

        let mut worker = TranslationWorker::new().expect("spawn worker");
        let job = TranslationJob::new(config, Box::new(DummyTranslator));
        let TranslationJobHandle {
            progress: mut progress_rx,
            result_rx,
        } = worker.submit(job).await.expect("submit job");

        // Wait for completion before draining progress.
        let result = result_rx.await.unwrap_or(Err(BabelEbookError::Cancelled));
        assert!(result.is_ok(), "{result:?}");

        // Progress events should have been emitted.
        let events = collect_progress(&mut progress_rx);
        assert!(events
            .iter()
            .any(|e| matches!(e, ProgressEvent::Started { .. })));
        assert!(events.iter().any(|e| matches!(e, ProgressEvent::Completed)));

        worker.shutdown();
    }

    #[tokio::test]
    async fn worker_receives_progress_events() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let source = write_test_epub(dir.path(), "progress.epub", &["Hello world."]);
        let output = dir.path().join("progress-out.epub");
        let config = make_config(source, output);

        let mut worker = TranslationWorker::new().expect("spawn worker");
        let job = TranslationJob::new(config, Box::new(DummyTranslator));
        let TranslationJobHandle {
            progress: mut progress_rx,
            result_rx,
        } = worker.submit(job).await.expect("submit job");

        let result = result_rx.await.unwrap_or(Err(BabelEbookError::Cancelled));
        assert!(result.is_ok(), "{result:?}");

        let events = collect_progress(&mut progress_rx);
        assert!(events
            .iter()
            .any(|e| matches!(e, ProgressEvent::Started { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, ProgressEvent::ChapterStarted { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, ProgressEvent::ChapterFinished { .. })));
        assert!(events.iter().any(|e| matches!(e, ProgressEvent::Completed)));

        worker.shutdown();
    }

    struct ControlledTranslator {
        proceed: Arc<Notify>,
    }

    #[async_trait]
    impl Translator for ControlledTranslator {
        fn name(&self) -> String {
            "controlled".into()
        }

        fn max_output_tokens(&self) -> usize {
            1000
        }

        async fn translate(
            &self,
            text: &str,
            _ctx: &TranslateContext<'_>,
        ) -> Result<String, BabelEbookError> {
            self.proceed.notified().await;
            Ok(format!("[{}]", text.trim()))
        }
    }

    #[tokio::test]
    async fn worker_cancellation_propagates_to_job() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let source = write_test_epub(
            dir.path(),
            "cancel.epub",
            &["First paragraph.", "Second paragraph."],
        );
        let output = dir.path().join("cancel-out.epub");
        let config = make_config(source, output);

        let proceed = Arc::new(Notify::new());
        let translator = ControlledTranslator {
            proceed: proceed.clone(),
        };
        let token = CancellationToken::default();

        let mut worker = TranslationWorker::new().expect("spawn worker");
        let job =
            TranslationJob::new(config, Box::new(translator)).with_cancellation(token.clone());
        let TranslationJobHandle {
            mut progress,
            result_rx,
        } = worker.submit(job).await.expect("submit job");

        // Wait for the translation to start before requesting cancellation.
        let mut started = false;
        while let Some(event) = progress.recv().await {
            if matches!(event, ProgressEvent::Started { .. }) {
                started = true;
                break;
            }
        }
        assert!(started, "translation never started");

        token.cancel();
        proceed.notify_one();

        let result = result_rx.await.unwrap_or(Err(BabelEbookError::Cancelled));
        assert!(
            matches!(result, Err(BabelEbookError::Cancelled)),
            "expected cancelled, got {result:?}"
        );

        worker.shutdown();
    }

    #[tokio::test]
    async fn worker_runs_sequential_jobs() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let source1 = write_test_epub(dir.path(), "first.epub", &["First job."]);
        let output1 = dir.path().join("first-out.epub");
        let source2 = write_test_epub(dir.path(), "second.epub", &["Second job."]);
        let output2 = dir.path().join("second-out.epub");

        let mut worker = TranslationWorker::new().expect("spawn worker");

        let job1 = TranslationJob::new(
            make_config(source1, output1.clone()),
            Box::new(DummyTranslator),
        );
        let handle1 = worker.submit(job1).await.expect("submit first job");
        let result1 = handle1.result().await;
        assert!(result1.is_ok(), "{result1:?}");
        assert!(output1.exists(), "first output should exist");

        let job2 = TranslationJob::new(
            make_config(source2, output2.clone()),
            Box::new(DummyTranslator),
        );
        let handle2 = worker.submit(job2).await.expect("submit second job");
        let result2 = handle2.result().await;
        assert!(result2.is_ok(), "{result2:?}");
        assert!(output2.exists(), "second output should exist");

        worker.shutdown();
    }

    #[tokio::test]
    async fn worker_submit_after_shutdown_returns_error() {
        let mut worker = TranslationWorker::new().expect("spawn worker");
        worker.shutdown();

        let dir = tempfile::tempdir().expect("create temp dir");
        let source = write_test_epub(dir.path(), "after_shutdown.epub", &["Too late."]);
        let output = dir.path().join("after_shutdown-out.epub");
        let job = TranslationJob::new(make_config(source, output), Box::new(DummyTranslator));

        let result = worker.submit(job).await;
        assert!(
            matches!(result, Err(WorkerError::ShutDown)),
            "expected ShutDown error, got {result:?}"
        );
    }

    #[tokio::test]
    async fn worker_job_with_cache_writes_cache_entries() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let source = write_test_epub(dir.path(), "cached.epub", &["<p>Repeat</p><p>Again</p>"]);
        let output = dir.path().join("cached-out.epub");
        let cache = TranslationCache::new(dir.path().join("cache"));
        let mut config = make_config(source, output.clone());
        config.cache_dir = dir.path().join("cache");

        let mut worker = TranslationWorker::new().expect("spawn worker");
        let job = TranslationJob::new(config, Box::new(DummyTranslator)).with_cache(cache.clone());
        let handle = worker.submit(job).await.expect("submit cached job");
        let result = handle.result().await;
        assert!(result.is_ok(), "{result:?}");
        assert!(output.exists(), "output should be written");

        // The cache should contain entries for both translated paragraphs.
        let cached = cache.get("dummy", "Repeat");
        assert_eq!(
            cached,
            Some("[Repeat]".to_string()),
            "cache should store the first paragraph translation"
        );
        let cached = cache.get("dummy", "Again");
        assert_eq!(
            cached,
            Some("[Again]".to_string()),
            "cache should store the second paragraph translation"
        );

        worker.shutdown();
    }
}
