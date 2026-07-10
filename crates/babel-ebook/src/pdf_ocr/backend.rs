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
    #[serde(default, deserialize_with = "deserialize_bbox_flexible")]
    pub bbox: Option<BoundingBox>,
    /// Semantic kind.
    #[serde(default)]
    pub block_type: BlockType,
}

/// Deserialize a bounding box from either an object `{x,y,w,h}` or an array
/// `[x,y,w,h]`. Malformed values fall back to `None` instead of failing the
/// entire page parse.
pub(crate) fn deserialize_bbox_flexible<'de, D>(
    deserializer: D,
) -> Result<Option<BoundingBox>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        Some(serde_json::Value::Object(mut map)) => {
            let x = map
                .remove("x")
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok());
            let y = map
                .remove("y")
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok());
            let w = map
                .remove("w")
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok());
            let h = map
                .remove("h")
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok());
            match (x, y, w, h) {
                (Some(x), Some(y), Some(w), Some(h)) => Ok(Some(BoundingBox { x, y, w, h })),
                _ => Ok(None),
            }
        }
        Some(serde_json::Value::Array(arr)) if arr.len() == 4 => {
            let vals: Option<Vec<u32>> = arr
                .iter()
                .map(|v| v.as_u64().and_then(|n| u32::try_from(n).ok()))
                .collect();
            Ok(vals.map(|v| BoundingBox {
                x: v[0],
                y: v[1],
                w: v[2],
                h: v[3],
            }))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::BoundingBox;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        #[serde(default, deserialize_with = "super::deserialize_bbox_flexible")]
        bbox: Option<BoundingBox>,
    }

    #[test]
    fn deserialize_bbox_object() {
        let json = r#"{"bbox": {"x": 10, "y": 20, "w": 30, "h": 40}}"#;
        let parsed: Wrapper = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed.bbox,
            Some(BoundingBox {
                x: 10,
                y: 20,
                w: 30,
                h: 40
            })
        );
    }

    #[test]
    fn deserialize_bbox_array() {
        let json = r#"{"bbox": [10, 20, 30, 40]}"#;
        let parsed: Wrapper = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed.bbox,
            Some(BoundingBox {
                x: 10,
                y: 20,
                w: 30,
                h: 40
            })
        );
    }

    #[test]
    fn deserialize_bbox_malformed_falls_back_to_none() {
        // Missing fields fall back to None without failing the whole object.
        let json = r#"{"bbox": {"x": 10, "w": 30, "h": 40}}"#;
        let parsed: Wrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.bbox, None);
    }

    #[test]
    fn deserialize_missing_bbox() {
        let json = r"{}";
        let parsed: Wrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.bbox, None);
    }
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
