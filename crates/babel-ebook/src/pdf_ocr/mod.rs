//! Convert scanned PDF files into EPUB using OCR + LLM verification.

use std::path::{Path, PathBuf};

use crate::core::BabelEbookError;
use crate::epub::EpubBook;

pub mod backend;
pub mod epub;
pub mod qwen;
pub mod render;
pub mod verify;

pub use backend::{BlockType, BoundingBox, OcrBackend, OcrPageResult, TextBlock};
pub use qwen::{QwenOcrBackend, QwenOcrConfig};
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
}

impl Default for PdfToEpubConfig {
    fn default() -> Self {
        Self {
            temp_dir: std::env::temp_dir().join("babel-ebook-pdf-ocr"),
            dpi: 200,
            verify_threshold: 0.7,
            max_retries: 2,
        }
    }
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
pub async fn convert_pdf_to_epub(
    pdf_path: &Path,
    title: &str,
    ocr: &dyn OcrBackend,
    verifier: Option<&dyn VerifyBackend>,
    config: &PdfToEpubConfig,
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

    let mut pages: Vec<OcrPageResult> = Vec::with_capacity(rendered.len());

    for (index, image_path) in rendered.iter().enumerate() {
        let page_number = index + 1;
        let image_bytes = std::fs::read(image_path).map_err(|e| {
            BabelEbookError::Anyhow(anyhow::anyhow!(
                "failed to read rendered page image {}: {e}",
                image_path.display()
            ))
        })?;

        let mut page = run_ocr_with_retries(ocr, &image_bytes, "image/png", page_number, config.max_retries).await?;

        if let Some(v) = verifier {
            verify::verify_page(v, &image_bytes, "image/png", &mut page, config.verify_threshold).await?;
        }

        pages.push(page);
    }

    Ok(epub::build_epub(title, &pages))
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
pub async fn convert_pdf_to_epub_file(
    pdf_path: &Path,
    output_path: &Path,
    title: &str,
    ocr: &dyn OcrBackend,
    verifier: Option<&dyn VerifyBackend>,
    config: &PdfToEpubConfig,
) -> Result<(), BabelEbookError> {
    let book = convert_pdf_to_epub(pdf_path, title, ocr, verifier, config).await?;
    crate::epub::write_epub(&book, output_path)
}
