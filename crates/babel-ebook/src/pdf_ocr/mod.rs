//! Convert scanned PDF files into EPUB using OCR + LLM verification.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::core::BabelEbookError;
use crate::epub::EpubBook;
use futures_util::{StreamExt, TryStreamExt};

pub mod backend;
pub mod epub;
pub mod post_process;
pub mod qwen;
pub mod refine;
pub mod render;
pub mod verify;

pub(crate) use backend::deserialize_bbox_flexible;
pub use backend::{BlockType, BoundingBox, OcrBackend, OcrPageResult, TextBlock};
pub use qwen::{QwenOcrBackend, QwenOcrConfig};
pub use refine::{refine_pages, OpenAiRefineBackend, OpenAiRefineConfig, RefineBackend};
pub use verify::{OpenAiVerifyBackend, OpenAiVerifyConfig, VerifyBackend};

/// Configuration for the PDF → EPUB conversion pipeline.
#[derive(Debug, Clone)]
pub struct PdfToEpubConfig {
    /// Temporary directory for rendered page images.
    pub temp_dir: PathBuf,
    /// Rendering resolution in DPI.
    pub dpi: u32,
    /// Confidence threshold below which a block is sent for verification.
    pub verify_threshold: f32,
    /// Maximum number of retry attempts for a failed page OCR.
    pub max_retries: usize,
    /// Maximum number of verify attempts for a low-confidence text block.
    pub verify_max_attempts: usize,
    /// Scale factors applied to the cropped block image during verify retries.
    /// Each factor is relative to the original block bounding box.
    pub verify_scale_factors: Vec<f32>,
    /// Number of pages to OCR concurrently.
    pub ocr_concurrency: usize,
    /// Number of LLM refinement rounds to run after OCR and before EPUB assembly.
    /// Set to 0 to disable refinement.
    pub refine_rounds: usize,
}

impl Default for PdfToEpubConfig {
    fn default() -> Self {
        Self {
            temp_dir: std::env::temp_dir().join("babel-ebook-pdf-ocr"),
            dpi: 200,
            verify_threshold: 0.7,
            max_retries: 2,
            verify_max_attempts: 3,
            verify_scale_factors: vec![1.0, 2.0, 3.0],
            ocr_concurrency: 3,
            refine_rounds: 0,
        }
    }
}

/// Stage of the PDF -> EPUB OCR pipeline, for progress reporting.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OcrStage {
    /// Rendering PDF pages to images.
    Render,
    /// OCR and per-page verification.
    Ocr,
    /// LLM structural refinement.
    Refine,
    /// Pipeline finished.
    Done,
}

/// A progress event emitted by the PDF -> EPUB OCR pipeline.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OcrProgressEvent {
    /// Current stage of the pipeline.
    pub stage: OcrStage,
    /// Completed page count during the OCR stage, or the page being refined.
    pub page: u32,
    /// Total number of pages in the PDF.
    pub page_total: u32,
    /// Current refinement round, if in the refine stage.
    pub refine_round: Option<u32>,
    /// Progress within the current stage, 0..=100.
    pub percent: u32,
    /// Human-readable status message.
    pub message: String,
}

/// Callback for reporting PDF -> EPUB OCR pipeline progress.
pub trait OcrProgressCallback: Send + Sync {
    /// Called whenever the pipeline makes progress.
    fn on_ocr_progress(&self, event: OcrProgressEvent);
}

/// Convert a scanned PDF to an `EpubBook`.
///
/// The pipeline renders each page to an image, runs OCR, optionally verifies
/// low-confidence blocks with an LLM, and assembles the results into an EPUB
/// with chapters split at detected headings.
///
/// # Errors
///
/// Returns an error if rendering fails, the OCR backend returns an error, or
/// the temporary directory cannot be created.
#[allow(clippy::too_many_lines)]
pub async fn convert_pdf_to_epub(
    pdf_path: &Path,
    title: &str,
    ocr: &dyn OcrBackend,
    verifier: Option<&dyn VerifyBackend>,
    refiner: Option<&dyn RefineBackend>,
    config: &PdfToEpubConfig,
    progress: Option<&dyn OcrProgressCallback>,
) -> Result<EpubBook, BabelEbookError> {
    std::fs::create_dir_all(&config.temp_dir).map_err(|e| {
        BabelEbookError::Anyhow(anyhow::anyhow!(
            "failed to create temp dir {}: {e}",
            config.temp_dir.display()
        ))
    })?;

    let rendered = render::render_pages(pdf_path, &config.temp_dir, config.dpi)?;
    if rendered.is_empty() {
        return Err(BabelEbookError::Configuration(
            "PDF contains no pages or pdftoppm produced no output".into(),
        ));
    }

    let total: u32 = u32::try_from(rendered.len()).unwrap_or(u32::MAX);
    if let Some(p) = progress {
        p.on_ocr_progress(OcrProgressEvent {
            stage: OcrStage::Ocr,
            page: 0,
            page_total: total,
            refine_round: None,
            percent: 0,
            message: format!("OCR 0/{total}"),
        });
    }

    let concurrency = config.ocr_concurrency.max(1);
    let completed = Arc::new(AtomicU32::new(0));
    let mut pages: Vec<OcrPageResult> =
        futures_util::stream::iter(rendered.clone().into_iter().enumerate())
            .map(|(index, image_path)| {
                let completed = completed.clone();
                async move {
                    let page_number = index + 1;
                    let image_bytes = std::fs::read(&image_path).map_err(|e| {
                        BabelEbookError::Anyhow(anyhow::anyhow!(
                            "failed to read rendered page image {}: {e}",
                            image_path.display()
                        ))
                    })?;

                    let mut page = run_ocr_with_retries(
                        ocr,
                        &image_bytes,
                        "image/png",
                        page_number,
                        config.max_retries,
                    )
                    .await?;

                    if let Some(v) = verifier {
                        verify::verify_page_with_retry(
                            v,
                            &image_bytes,
                            "image/png",
                            &mut page,
                            config.verify_threshold,
                            config.verify_max_attempts,
                            &config.verify_scale_factors,
                        )
                        .await?;
                    }

                    if let Some(p) = progress {
                        let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
                        let percent = (done * 100 / total).min(100);
                        p.on_ocr_progress(OcrProgressEvent {
                            stage: OcrStage::Ocr,
                            page: done,
                            page_total: total,
                            refine_round: None,
                            percent,
                            message: format!("OCR {done}/{total}"),
                        });
                    }

                    Ok::<_, BabelEbookError>(page)
                }
            })
            .buffered(concurrency)
            .try_collect()
            .await?;

    if let Some(r) = refiner {
        refine_pages(
            r,
            &rendered,
            &mut pages,
            config.refine_rounds,
            progress,
            total,
        )
        .await?;
    }

    post_process::clean_pages(&mut pages);
    let image_resources = extract_and_embed_images(&rendered, &mut pages, &config.temp_dir)?;

    if let Some(p) = progress {
        p.on_ocr_progress(OcrProgressEvent {
            stage: OcrStage::Done,
            page: total,
            page_total: total,
            refine_round: None,
            percent: 100,
            message: "Done".to_string(),
        });
    }

    let mut book = epub::build_epub(title, &pages);
    book.resources.extend(image_resources);
    Ok(book)
}

/// Crop figure/diagram regions from rendered page images and embed them as
/// EPUB resources. This only works when the OCR backend supplies bounding
/// boxes for `other` blocks; otherwise the blocks are left as text.
fn extract_and_embed_images(
    rendered: &[PathBuf],
    pages: &mut [OcrPageResult],
    temp_dir: &Path,
) -> Result<Vec<crate::epub::Resource>, BabelEbookError> {
    use crate::pdf_ocr::backend::{BlockType, BoundingBox};
    use image::imageops::crop_imm;

    let mut resources = Vec::new();
    let mut image_count = 0;
    for (page_idx, page) in pages.iter_mut().enumerate() {
        let Some(image_path) = rendered.get(page_idx) else {
            continue;
        };
        let img = image::open(image_path).map_err(|e| {
            BabelEbookError::Anyhow(anyhow::anyhow!(
                "failed to open rendered page image {}: {e}",
                image_path.display()
            ))
        })?;

        for block in &mut page.blocks {
            if block.block_type != BlockType::Other {
                continue;
            }
            let Some(BoundingBox { x, y, w, h }) = block.bbox else {
                continue;
            };
            if w < 20 || h < 20 {
                continue;
            }

            image_count += 1;
            let filename = format!("figure-{page_idx:03}-{image_count:03}.png");
            let crop_path = temp_dir.join(&filename);
            let cropped = crop_imm(&img, x, y, w, h).to_image();
            let mut out = std::fs::File::create(&crop_path).map_err(|e| {
                BabelEbookError::Anyhow(anyhow::anyhow!(
                    "failed to create cropped image {}: {e}",
                    crop_path.display()
                ))
            })?;
            cropped
                .write_to(&mut out, image::ImageFormat::Png)
                .map_err(|e| {
                    BabelEbookError::Anyhow(anyhow::anyhow!(
                        "failed to encode cropped image {}: {e}",
                        crop_path.display()
                    ))
                })?;

            let data = std::fs::read(&crop_path).map_err(|e| {
                BabelEbookError::Anyhow(anyhow::anyhow!(
                    "failed to read cropped image {}: {e}",
                    crop_path.display()
                ))
            })?;
            resources.push(crate::epub::Resource {
                href: filename.clone(),
                mime: "image/png".to_string(),
                data,
            });

            block.block_type = BlockType::Image;
            block.text = filename;
        }
    }

    Ok(resources)
}

async fn run_ocr_with_retries(
    ocr: &dyn OcrBackend,
    image_bytes: &[u8],
    mime_type: &str,
    page_number: usize,
    max_retries: usize,
) -> Result<OcrPageResult, BabelEbookError> {
    let mut last_err: Option<BabelEbookError> = None;

    for attempt in 0..=max_retries {
        match ocr.extract(image_bytes, mime_type).await {
            Ok(mut page) => {
                page.page_number = page_number;
                return Ok(page);
            }
            Err(err) => {
                tracing::warn!(
                    page = page_number,
                    attempt = attempt + 1,
                    error = %err,
                    "OCR attempt failed"
                );
                last_err = Some(err);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| {
        BabelEbookError::ApiError(format!("OCR failed for page {page_number} after retries"))
    }))
}

/// Convenience function that renders, OCRs, verifies and writes an EPUB file.
#[allow(clippy::too_many_arguments)]
pub async fn convert_pdf_to_epub_file(
    pdf_path: &Path,
    output_path: &Path,
    title: &str,
    ocr: &dyn OcrBackend,
    verifier: Option<&dyn VerifyBackend>,
    refiner: Option<&dyn RefineBackend>,
    config: &PdfToEpubConfig,
    progress: Option<&dyn OcrProgressCallback>,
) -> Result<(), BabelEbookError> {
    let book =
        convert_pdf_to_epub(pdf_path, title, ocr, verifier, refiner, config, progress).await?;
    crate::epub::write_epub(&book, output_path)
}

/// Strip JavaScript-style comments from a JSON string.
///
/// Some vision models return JSON with `//` or `/* */` comments despite being
/// asked for raw JSON. This function removes those comments while preserving
/// content inside string literals.
pub(crate) fn strip_json_comments(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut in_string = false;

    while i < chars.len() {
        if in_string {
            result.push(chars[i]);
            if chars[i] == '\\' && i + 1 < chars.len() {
                result.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if chars[i] == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if chars[i] == '"' {
            in_string = true;
        } else if chars[i] == '/' && i + 1 < chars.len() {
            if chars[i + 1] == '/' {
                // Line comment: skip until newline.
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                i += 1;
                continue;
            } else if chars[i + 1] == '*' {
                // Block comment: skip until */.
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::strip_json_comments;

    #[test]
    fn strip_json_comments_removes_line_comments() {
        let input = r#"{"blocks": [{"text": "hello // world", "confidence": 1.0}]} // comment"#;
        let expected = r#"{"blocks": [{"text": "hello // world", "confidence": 1.0}]} "#;
        assert_eq!(strip_json_comments(input), expected);
    }

    #[test]
    fn strip_json_comments_removes_block_comments() {
        let input = r#"{"blocks": [/* inner */{"text": "hi", "confidence": 1.0}]}"#;
        let expected = r#"{"blocks": [{"text": "hi", "confidence": 1.0}]}"#;
        assert_eq!(strip_json_comments(input), expected);
    }

    #[test]
    fn strip_json_comments_preserves_escaped_quotes() {
        let input = r#"{"text": "say \"hello\"", "confidence": 1.0}"#;
        assert_eq!(strip_json_comments(input), input);
    }
}
