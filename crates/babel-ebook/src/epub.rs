//! EPUB read/write abstraction built on `rbook` with a `zip` fallback.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use rbook::epub::Epub;
use regex::RegexBuilder;
use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use crate::core::BabelEbookError;
use crate::escape::xml_escape;

const DEFAULT_TITLE: &str = "Untitled";
const DEFAULT_LANGUAGE: &str = "en";
const DEFAULT_IDENTIFIER: &str = "urn:babel-ebook:unknown";
const CONTENT_DIR_PREFIXES: &[&str] = &["OEBPS/", "EPUB/"];

fn rbook_err<E: std::fmt::Display>(err: E) -> BabelEbookError {
    BabelEbookError::Anyhow(anyhow::anyhow!("rbook error: {err}"))
}

/// Core metadata fields extracted from an EPUB.
#[derive(Debug, Clone, Default)]
pub struct EpubMetadata {
    /// Book title.
    pub title: Option<String>,
    /// Book language.
    pub language: Option<String>,
    /// Unique identifier.
    pub identifier: Option<String>,
}

impl EpubMetadata {
    fn title_or_default(&self) -> &str {
        self.title.as_deref().unwrap_or(DEFAULT_TITLE)
    }

    fn language_or_default(&self) -> &str {
        self.language.as_deref().unwrap_or(DEFAULT_LANGUAGE)
    }

    fn identifier_or_default(&self) -> &str {
        self.identifier.as_deref().unwrap_or(DEFAULT_IDENTIFIER)
    }
}

/// A single document in the EPUB reading order.
#[derive(Debug, Clone)]
pub struct Chapter {
    /// Manifest href of the chapter.
    pub href: String,
    /// Optional table-of-contents label.
    pub title: Option<String>,
    /// Raw XHTML content bytes.
    pub content: Vec<u8>,
}

/// A non-document resource such as an image, stylesheet, or font.
#[derive(Debug, Clone)]
pub struct Resource {
    /// Manifest href of the resource.
    pub href: String,
    /// MIME type of the resource.
    pub mime: String,
    /// Raw resource bytes.
    pub data: Vec<u8>,
}

/// In-memory representation of an EPUB book.
#[derive(Debug, Clone)]
pub struct EpubBook {
    /// EPUB metadata.
    pub metadata: EpubMetadata,
    /// Reading-order chapters.
    pub chapters: Vec<Chapter>,
    /// Non-document resources.
    pub resources: Vec<Resource>,
}

impl EpubBook {
    /// Read an EPUB from disk into the `EpubBook` abstraction.
    ///
    /// # Errors
    ///
    /// Returns `BabelEbookError::Anyhow` if `rbook` fails to open or parse the
    /// EPUB.
    pub fn read(path: &Path) -> Result<Self, BabelEbookError> {
        read_epub(path)
    }

    /// Write this `EpubBook` to disk as a valid EPUB 2 archive.
    ///
    /// # Errors
    ///
    /// Returns `BabelEbookError::Anyhow` if the output file or parent directory
    /// cannot be created.
    pub fn write(&self, path: &Path) -> Result<(), BabelEbookError> {
        write_epub(self, path)
    }
}

/// Read an EPUB from disk into the `EpubBook` abstraction.
pub fn read_epub(path: &Path) -> Result<EpubBook, BabelEbookError> {
    let epub = Epub::open(path).map_err(rbook_err)?;

    let metadata = EpubMetadata {
        title: epub.metadata().title().map(|t| t.value().to_owned()),
        language: epub.metadata().language().map(|l| l.value().to_owned()),
        identifier: epub.metadata().identifier().map(|i| i.value().to_owned()),
    };

    // Build a lookup from document href (without fragment) to its ToC label.
    //
    // Nested ToC entries may point to the same chapter file with a fragment
    // (e.g. a "References" subsection). Non-fragment entries are the chapter's
    // main title and must take precedence over fragment entries.
    let mut toc_titles: HashMap<String, String> = HashMap::new();
    if let Some(contents) = epub.toc().contents() {
        for entry in contents.flatten() {
            if let Some(href) = entry.href() {
                let key = href.path().as_str().to_owned();
                let label = entry.label().to_owned();
                if href.fragment().is_some() {
                    // Fragment entries only fill in a missing title; they never
                    // overwrite the main chapter title.
                    toc_titles.entry(key).or_insert(label);
                } else {
                    // A non-fragment entry is the canonical chapter title.
                    toc_titles.insert(key, label);
                }
            }
        }
    }

    // Collect chapters from the spine reader.
    let mut chapters = Vec::new();
    let mut chapter_hrefs = HashSet::new();
    let mut reader = epub.reader();
    while let Some(result) = reader.read_next() {
        let content = result.map_err(rbook_err)?;
        let manifest_entry = content.manifest_entry();
        let href = manifest_entry.href().path().as_str().to_owned();

        let title = toc_titles.get(&href).cloned();
        let content_bytes = content.into_bytes();

        chapter_hrefs.insert(href.clone());
        chapters.push(Chapter {
            href,
            title,
            content: content_bytes,
        });
    }

    // Collect remaining manifest items as resources.
    let mut resources = Vec::new();
    for entry in epub.manifest() {
        let href = entry.href().path().as_str().to_owned();
        if chapter_hrefs.contains(&href) {
            continue;
        }

        let mime = entry.media_type().to_owned();
        // Skip generated NCX/package files; they are recreated on write.
        if mime == "application/x-dtbncx+xml" {
            continue;
        }

        let data = entry.read_bytes().map_err(rbook_err)?;
        resources.push(Resource { href, mime, data });
    }

    Ok(EpubBook {
        metadata,
        chapters,
        resources,
    })
}

/// Write an `EpubBook` to disk as a valid EPUB 2 archive.
///
/// The implementation uses `zip::ZipWriter` and writes handwritten
/// `content.opf`/`toc.ncx` files so the output is deterministic and does not
/// depend on the input archive layout.
pub fn write_epub(book: &EpubBook, path: &Path) -> Result<(), BabelEbookError> {
    write_epub_zip(book, path)
}

/// Normalize an href so it is relative to `OEBPS/` and has no leading slash
/// or known content-directory prefix.
fn normalize_href(href: &str) -> String {
    let trimmed = href.trim_start_matches('/');
    let without_prefix = CONTENT_DIR_PREFIXES
        .iter()
        .find_map(|prefix| trimmed.strip_prefix(prefix))
        .unwrap_or(trimmed);
    if without_prefix.is_empty() {
        return "index.xhtml".to_string();
    }
    without_prefix.to_string()
}

/// Build an XML-safe manifest item id from an href.
fn manifest_id(href: &str) -> String {
    let normalized = normalize_href(href);
    let mut id: String = normalized
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '.' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if id.chars().next().is_none_or(|c| c.is_ascii_digit()) {
        id.insert(0, 'i');
    }
    id
}

/// Return a manifest item id for `href`, guaranteeing uniqueness within `used`.
fn unique_manifest_id(href: &str, used: &mut HashSet<String>) -> String {
    let base = manifest_id(href);
    if used.insert(base.clone()) {
        return base;
    }
    let mut counter = 2;
    loop {
        let candidate = format!("{base}_{counter}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        counter += 1;
    }
}

/// Write an EPUB using `zip::ZipWriter` and handwritten `content.opf`/`toc.ncx`.
pub(crate) fn write_epub_zip(book: &EpubBook, path: &Path) -> Result<(), BabelEbookError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            BabelEbookError::Anyhow(anyhow::anyhow!(
                "failed to create output directory {}: {e}",
                parent.display()
            ))
        })?;
    }

    let file = File::create(path)
        .map_err(|e| BabelEbookError::Anyhow(anyhow::anyhow!("failed to create EPUB file: {e}")))?;
    let mut zip = ZipWriter::new(file);

    // mimetype must be the first entry and stored uncompressed.
    zip.start_file(
        "mimetype",
        FileOptions::<()>::default().compression_method(CompressionMethod::Stored),
    )
    .map_err(zip_err)?;
    zip.write_all(b"application/epub+zip").map_err(zip_err)?;

    // META-INF/container.xml
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#;
    zip.start_file(
        "META-INF/container.xml",
        FileOptions::<()>::default().compression_method(CompressionMethod::Deflated),
    )
    .map_err(zip_err)?;
    zip.write_all(container).map_err(zip_err)?;

    // OEBPS/content.opf
    let opf = build_content_opf(book);
    zip.start_file(
        "OEBPS/content.opf",
        FileOptions::<()>::default().compression_method(CompressionMethod::Deflated),
    )
    .map_err(zip_err)?;
    zip.write_all(opf.as_bytes()).map_err(zip_err)?;

    // OEBPS/toc.ncx
    let ncx = build_toc_ncx(book);
    zip.start_file(
        "OEBPS/toc.ncx",
        FileOptions::<()>::default().compression_method(CompressionMethod::Deflated),
    )
    .map_err(zip_err)?;
    zip.write_all(ncx.as_bytes()).map_err(zip_err)?;

    // Chapters.
    for chapter in &book.chapters {
        let href = normalize_href(&chapter.href);
        zip.start_file(
            format!("OEBPS/{href}"),
            FileOptions::<()>::default().compression_method(CompressionMethod::Deflated),
        )
        .map_err(zip_err)?;
        zip.write_all(&chapter.content).map_err(zip_err)?;
    }

    // Resources.
    for resource in &book.resources {
        let href = normalize_href(&resource.href);
        zip.start_file(
            format!("OEBPS/{href}"),
            FileOptions::<()>::default().compression_method(CompressionMethod::Deflated),
        )
        .map_err(zip_err)?;
        zip.write_all(&resource.data).map_err(zip_err)?;
    }

    zip.finish().map_err(zip_err)?;
    Ok(())
}

fn zip_err<E: std::fmt::Display>(err: E) -> BabelEbookError {
    BabelEbookError::Anyhow(anyhow::anyhow!("zip writer error: {err}"))
}

/// Build a minimal but valid EPUB 2/3 package document.
fn build_content_opf(book: &EpubBook) -> String {
    let title = xml_escape(book.metadata.title_or_default());
    let language = xml_escape(book.metadata.language_or_default());
    let identifier = xml_escape(book.metadata.identifier_or_default());

    // Pre-compute unique manifest ids so manifest items and spine itemrefs agree.
    let mut used_ids: HashSet<String> = HashSet::new();
    used_ids.insert("ncx".to_string());
    used_ids.insert("bookid".to_string());
    let chapter_ids: Vec<String> = book
        .chapters
        .iter()
        .map(|c| unique_manifest_id(&c.href, &mut used_ids))
        .collect();
    let resource_ids: Vec<String> = book
        .resources
        .iter()
        .map(|r| unique_manifest_id(&r.href, &mut used_ids))
        .collect();

    let mut opf = String::new();
    opf.push_str(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="2.0" xmlns="http://www.idpf.org/2007/opf" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>"#,
    );
    opf.push_str(&title);
    opf.push_str("</dc:title>\n");
    opf.push_str("    <dc:language>");
    opf.push_str(&language);
    opf.push_str("</dc:language>\n");
    opf.push_str("    <dc:identifier id=\"bookid\">");
    opf.push_str(&identifier);
    opf.push_str("</dc:identifier>\n");
    opf.push_str("  </metadata>\n  <manifest>\n");

    // NCX is required for EPUB 2 readers.
    opf.push_str(r#"    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>"#);
    opf.push('\n');

    for (chapter, id) in book.chapters.iter().zip(&chapter_ids) {
        let href = xml_escape(&normalize_href(&chapter.href));
        let id = xml_escape(id);
        let _ = writeln!(
            opf,
            "    <item id=\"{id}\" href=\"{href}\" media-type=\"application/xhtml+xml\"/>"
        );
    }

    for (resource, id) in book.resources.iter().zip(&resource_ids) {
        let href = xml_escape(&normalize_href(&resource.href));
        let id = xml_escape(id);
        let mime = xml_escape(&resource.mime);
        let _ = writeln!(
            opf,
            "    <item id=\"{id}\" href=\"{href}\" media-type=\"{mime}\"/>"
        );
    }

    opf.push_str("  </manifest>\n  <spine toc=\"ncx\">\n");

    for id in &chapter_ids {
        let id = xml_escape(id);
        let _ = writeln!(opf, "    <itemref idref=\"{id}\"/>");
    }

    opf.push_str("  </spine>\n</package>\n");
    opf
}

/// Build an EPUB 2 NCX table of contents.
fn build_toc_ncx(book: &EpubBook) -> String {
    let title = xml_escape(book.metadata.title_or_default());
    let identifier = xml_escape(book.metadata.identifier_or_default());

    let mut ncx = String::new();
    ncx.push_str(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ncx version="2005-1" xmlns="http://www.daisy.org/z3986/2005/ncx/">
  <head>
    <meta name="dtb:uid" content=""#,
    );
    ncx.push_str(&identifier);
    ncx.push_str(
        r#""/>
    <meta name="dtb:depth" content="1"/>
    <meta name="dtb:totalPageCount" content="0"/>
    <meta name="dtb:maxPageNumber" content="0"/>
  </head>
  <docTitle>
    <text>"#,
    );
    ncx.push_str(&title);
    ncx.push_str(
        r"</text>
  </docTitle>
  <navMap>
",
    );

    for (index, chapter) in book.chapters.iter().enumerate() {
        let play_order = index + 1;
        let label = xml_escape(chapter.title.as_deref().unwrap_or("Untitled"));
        let href = xml_escape(&normalize_href(&chapter.href));
        let _ = write!(
            ncx,
            r#"    <navPoint id="navPoint-{play_order}" playOrder="{play_order}">
      <navLabel>
        <text>{label}</text>
      </navLabel>
      <content src="{href}"/>
    </navPoint>
"#
        );
    }

    ncx.push_str("  </navMap>\n</ncx>\n");
    ncx
}

/// Return `true` only if `name` does not match any of the case-insensitive
/// regex `patterns`.
///
/// # Errors
///
/// Returns `BabelEbookError::Configuration` if any `pattern` is not a valid regular
/// expression.
pub fn should_translate_doc(name: &str, patterns: &[String]) -> Result<bool, BabelEbookError> {
    let lower = name.to_lowercase();
    for pattern in patterns {
        let regex = RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .map_err(|err| {
                BabelEbookError::Configuration(format!("invalid skip pattern '{pattern}': {err}"))
            })?;
        if regex.is_match(&lower) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::read::ZipArchive;

    #[test]
    fn should_translate_doc_filters_patterns() {
        let patterns = vec!["cover".to_string(), "copyright".to_string()];
        assert!(should_translate_doc("chapter01.xhtml", &patterns).unwrap());
        assert!(!should_translate_doc("cover.xhtml", &patterns).unwrap());
        assert!(!should_translate_doc("Copyright_Page.xhtml", &patterns).unwrap());
    }

    #[test]
    fn should_translate_doc_errors_on_invalid_regex() {
        let patterns = vec!["(".to_string()];
        let result = should_translate_doc("chapter.xhtml", &patterns);
        assert!(matches!(result, Err(BabelEbookError::Configuration(_))));
    }

    #[test]
    fn normalize_href_trims_leading_slash_and_content_prefix() {
        assert_eq!(normalize_href("/EPUB/ch01.xhtml"), "ch01.xhtml");
        assert_eq!(normalize_href("chapter.xhtml"), "chapter.xhtml");
        assert_eq!(
            normalize_href("OEBPS/images/figure.png"),
            "images/figure.png"
        );
        assert_eq!(normalize_href("/OEBPS/cover.xhtml"), "cover.xhtml");
        assert_eq!(normalize_href("/"), "index.xhtml");
        assert_eq!(normalize_href("OEBPS/"), "index.xhtml");
    }

    #[test]
    fn manifest_id_is_xml_safe() {
        assert_eq!(manifest_id("chapter.xhtml"), "chapter.xhtml");
        assert_eq!(manifest_id("/EPUB/ch01.xhtml"), "ch01.xhtml");
        assert_eq!(manifest_id("1chapter.xhtml"), "i1chapter.xhtml");
        assert_eq!(manifest_id("OEBPS/images/figure.png"), "images_figure.png");
    }

    #[test]
    fn write_epub_zip_round_trip() {
        let custom_mime = "image/png";
        let book = EpubBook {
            metadata: EpubMetadata {
                title: Some("Zip Fallback Book".to_string()),
                language: Some("en".to_string()),
                identifier: Some("urn:test:zip-fallback".to_string()),
            },
            chapters: vec![
                Chapter {
                    href: "/cover.xhtml".to_string(),
                    title: Some("Cover".to_string()),
                    content: br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Cover</title></head>
<body><h1>Cover</h1></body>
</html>"#
                        .to_vec(),
                },
                Chapter {
                    href: "chapter.xhtml".to_string(),
                    title: Some("Chapter 1".to_string()),
                    content: br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title></head>
<body><p>Zip fallback content.</p></body>
</html>"#
                        .to_vec(),
                },
            ],
            resources: vec![Resource {
                href: "/images/figure.png".to_string(),
                mime: custom_mime.to_string(),
                data: vec![0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a],
            }],
        };

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let output = temp_dir.path().join("zip_fallback.epub");

        write_epub_zip(&book, &output).expect("write zip_fallback.epub");

        let round_tripped = read_epub(&output).expect("read zip_fallback.epub");

        assert_eq!(
            round_tripped.chapters.len(),
            book.chapters.len(),
            "zip fallback should preserve chapter count"
        );

        assert!(
            round_tripped.chapters[0].href.ends_with("cover.xhtml"),
            "cover href should end with cover.xhtml"
        );
        assert!(
            round_tripped.chapters[1].href.ends_with("chapter.xhtml"),
            "chapter href should end with chapter.xhtml"
        );

        let content = String::from_utf8_lossy(&round_tripped.chapters[1].content);
        assert!(content.contains("Zip fallback content."));

        assert_eq!(
            round_tripped.metadata.title,
            Some("Zip Fallback Book".to_string())
        );

        let resource = round_tripped
            .resources
            .iter()
            .find(|r| r.href.ends_with("images/figure.png"))
            .expect("original resource should survive zip fallback");
        assert_eq!(resource.mime, custom_mime);
        assert_eq!(resource.data, book.resources[0].data);
    }

    #[test]
    fn write_epub_zip_strips_content_directory_prefix() {
        let book = EpubBook {
            metadata: EpubMetadata {
                title: Some("Prefixed Book".to_string()),
                language: Some("en".to_string()),
                identifier: Some("urn:test:prefixed".to_string()),
            },
            chapters: vec![Chapter {
                href: "OEBPS/chapter.xhtml".to_string(),
                title: Some("Chapter 1".to_string()),
                content: br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title></head>
<body><p>Prefixed content.</p></body>
</html>"#
                    .to_vec(),
            }],
            resources: vec![Resource {
                href: "OEBPS/images/figure.png".to_string(),
                mime: "image/png".to_string(),
                data: vec![0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a],
            }],
        };

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let output = temp_dir.path().join("prefixed.epub");

        write_epub_zip(&book, &output).expect("write prefixed.epub");

        // Verify the archive does not double-prefix the content directory.
        let file = File::open(&output).expect("open prefixed.epub");
        let archive = ZipArchive::new(file).expect("read prefixed.epub as zip");
        let names: Vec<String> = archive
            .file_names()
            .map(std::borrow::ToOwned::to_owned)
            .collect();
        assert!(
            names.iter().any(|n| n == "OEBPS/images/figure.png"),
            "resource should be written to OEBPS/images/figure.png, got {names:?}"
        );
        assert!(
            !names.iter().any(|n| n == "OEBPS/OEBPS/images/figure.png"),
            "resource should not be double-prefixed, got {names:?}"
        );

        // Verify the book still round-trips correctly.
        let round_tripped = read_epub(&output).expect("read prefixed.epub");
        assert_eq!(round_tripped.chapters.len(), 1);
        assert!(
            round_tripped.chapters[0].href.ends_with("chapter.xhtml"),
            "chapter href should end with chapter.xhtml, got {}",
            round_tripped.chapters[0].href
        );
        let content = String::from_utf8_lossy(&round_tripped.chapters[0].content);
        assert!(content.contains("Prefixed content."));

        let resource = round_tripped
            .resources
            .iter()
            .find(|r| r.href.ends_with("images/figure.png"))
            .expect("resource should survive zip fallback");
        assert_eq!(resource.data, book.resources[0].data);
    }
}
