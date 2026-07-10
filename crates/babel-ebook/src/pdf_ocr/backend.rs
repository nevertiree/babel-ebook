//! OCR backend trait and shared types for PDF conversion.

use serde::{Deserialize, Serialize};

use crate::core::BabelEbookError;

/// A rectangular region within a page image, in pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

/// Semantic kind of a text region.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockType {
    /// Major title or chapter heading.
    Heading,
    /// Subheading.
    Subheading,
    /// Body paragraph.
    #[default]
    Paragraph,
    /// Caption for an image or figure.
    Caption,
    /// Text in a table cell.
    TableCell,
    /// Other or unknown.
    Other,
}

/// A single extracted text region.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextBlock {
    /// Extracted text.
    pub text: String,
    /// Estimated confidence, 0.0–1.0.
    #[serde(default)]
    pub confidence: f32,
    /// Region within the page image, if available.
    pub bbox: Option<BoundingBox>,
    /// Semantic kind.
    #[serde(default)]
    pub block_type: BlockType,
}

/// Result of OCR over one page image.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OcrPageResult {
    /// Page number (1-based).
    pub page_number: usize,
    /// Extracted text blocks in reading order.
    pub blocks: Vec<TextBlock>,
    /// Concatenated full text of the page.
    pub full_text: String,
}

/// A backend that extracts text from page images.
#[async_trait::async_trait]
pub trait OcrBackend: Send + Sync {
    /// Extract text blocks from a single image.
    ///
    /// `image_bytes` contains a PNG or JPEG encoded image.
    /// `mime_type` is either `image/png` or `image/jpeg`.
    async fn extract(
        &self,
        image_bytes: &[u8],
        mime_type: &str,
    ) -> Result<OcrPageResult, BabelEbookError>;
}
