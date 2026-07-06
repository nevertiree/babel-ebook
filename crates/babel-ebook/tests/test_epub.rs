use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use babel_ebook::{read_epub, write_epub, Chapter, EpubBook, EpubMetadata, Resource};
use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

/// Creates a small sample EPUB in `dir` and returns its path.
fn create_sample_fixture(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("sample.epub");
    let book = EpubBook {
        metadata: EpubMetadata {
            title: Some("Sample".to_string()),
            language: Some("en".to_string()),
            identifier: Some("urn:test:sample".to_string()),
        },
        chapters: vec![
            Chapter {
                href: "nav.xhtml".to_string(),
                title: Some("Nav".to_string()),
                content: br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Nav</title></head>
<body><nav><ul><li><a href="ch01.xhtml">Chapter 1</a></li></ul></nav></body>
</html>"#
                    .to_vec(),
            },
            Chapter {
                href: "ch01.xhtml".to_string(),
                title: Some("Chapter 1".to_string()),
                content: br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title></head>
<body><h1>Chapter 1</h1><p>Hello world.</p></body>
</html>"#
                    .to_vec(),
            },
        ],
        resources: vec![Resource {
            href: "data.bin".to_string(),
            mime: "application/x-custom-resource".to_string(),
            data: vec![0x00, 0x01, 0x02, 0x03],
        }],
    };

    write_epub(&book, &path).expect("write sample fixture");
    path
}

#[test]
fn read_sample_epub() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let fixture = create_sample_fixture(temp_dir.path());
    let book = read_epub(&fixture).expect("read sample.epub");

    assert!(
        book.chapters.len() >= 2,
        "sample should contain at least two spine entries, got {}",
        book.chapters.len()
    );

    let hrefs: Vec<&str> = book.chapters.iter().map(|c| c.href.as_str()).collect();
    assert_eq!(hrefs, ["/OEBPS/nav.xhtml", "/OEBPS/ch01.xhtml"]);
}

#[test]
fn round_trip_epub_updates_content() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let input = create_sample_fixture(temp_dir.path());
    let mut book = read_epub(&input).expect("read original sample.epub");

    // Mutate the first real chapter content (skip the nav document).
    let target_index = book
        .chapters
        .iter()
        .position(|c| !c.href.contains("nav"))
        .unwrap_or(0);
    let marker = "<p>BABEL_EBOOK_ROUND_TRIP_MARKER</p>";
    let original_content = String::from_utf8_lossy(&book.chapters[target_index].content);
    let updated = format!("{}\n{}", original_content, marker);
    book.chapters[target_index].content = updated.into_bytes();

    let original_resource_set: HashSet<(String, &str)> = book
        .resources
        .iter()
        .map(|r| {
            let href = r
                .href
                .strip_prefix("/OEBPS/")
                .unwrap_or(&r.href)
                .to_string();
            (href, r.mime.as_str())
        })
        .collect();

    let output = temp_dir.path().join("rewritten.epub");

    write_epub(&book, &output).expect("write rewritten.epub");

    let rewritten = read_epub(&output).expect("read rewritten.epub");

    // Chapters survive the round trip.
    assert_eq!(
        rewritten.chapters.len(),
        book.chapters.len(),
        "chapter count should be preserved"
    );
    assert_eq!(
        rewritten.chapters[target_index].href, book.chapters[target_index].href,
        "target chapter href should be preserved"
    );

    // All original non-chapter resources survive the round trip.
    let rewritten_resource_set: HashSet<(String, &str)> = rewritten
        .resources
        .iter()
        .map(|r| {
            let href = r
                .href
                .strip_prefix("/OEBPS/")
                .unwrap_or(&r.href)
                .to_string();
            (href, r.mime.as_str())
        })
        .collect();
    for resource in &original_resource_set {
        assert!(
            rewritten_resource_set.contains(resource),
            "resource {:?} should survive the round trip",
            resource
        );
    }

    let rewritten_content = String::from_utf8_lossy(&rewritten.chapters[target_index].content);
    assert!(
        rewritten_content.contains(marker),
        "rewritten chapter should contain the injected marker"
    );
}

#[test]
fn write_minimal_epub() {
    let book = EpubBook {
        metadata: EpubMetadata {
            title: Some("Minimal Book".to_string()),
            language: Some("en".to_string()),
            identifier: Some("urn:test:minimal".to_string()),
        },
        chapters: vec![Chapter {
            href: "chapter.xhtml".to_string(),
            title: Some("Chapter 1".to_string()),
            content: br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title></head>
<body><p>Hello world.</p></body>
</html>"#
                .to_vec(),
        }],
        resources: vec![],
    };

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let output = temp_dir.path().join("minimal.epub");

    write_epub(&book, &output).expect("write minimal.epub");

    let round_tripped = read_epub(&output).expect("read minimal.epub");
    assert_eq!(round_tripped.chapters.len(), 1);
    assert!(
        round_tripped.chapters[0].href.ends_with("chapter.xhtml"),
        "chapter href should end with chapter.xhtml, got {}",
        round_tripped.chapters[0].href
    );
    let content = String::from_utf8_lossy(&round_tripped.chapters[0].content);
    assert!(content.contains("Hello world."));
    assert_eq!(
        round_tripped.metadata.title,
        Some("Minimal Book".to_string())
    );
}

#[test]
fn round_trip_preserves_resource_media_type() {
    // Use a href whose extension would normally infer a different MIME type,
    // proving that the original manifest media-type is preserved rather than
    // re-inferred from the extension.
    let custom_mime = "application/x-custom-resource";
    let book = EpubBook {
        metadata: EpubMetadata {
            title: Some("Mime Test".to_string()),
            language: Some("en".to_string()),
            identifier: Some("urn:test:mime".to_string()),
        },
        chapters: vec![Chapter {
            href: "chapter.xhtml".to_string(),
            title: Some("Chapter 1".to_string()),
            content: br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title></head>
<body><p>Hello world.</p></body>
</html>"#
                .to_vec(),
        }],
        resources: vec![Resource {
            href: "data.bin".to_string(),
            mime: custom_mime.to_string(),
            data: vec![0x00, 0x01, 0x02, 0x03],
        }],
    };

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let output = temp_dir.path().join("mime_test.epub");

    write_epub(&book, &output).expect("write mime_test.epub");

    let round_tripped = read_epub(&output).expect("read mime_test.epub");
    let resource = round_tripped
        .resources
        .iter()
        .find(|r| r.href.ends_with("data.bin"))
        .expect("original resource should survive round trip");
    assert_eq!(
        resource.mime, custom_mime,
        "resource media type should be preserved, not inferred from extension"
    );
    assert_eq!(
        resource.data, book.resources[0].data,
        "resource data should be preserved"
    );
}

/// Writes a minimal EPUB whose toc.ncx has nested navPoints.
///
/// The top-level navPoint for the chapter has the canonical title, while a
/// later nested navPoint points to the same file with a fragment and a
/// different label (simulating a "References" subsection).
fn write_nested_toc_fixture(path: &std::path::Path, chapter_title: &str, fragment_label: &str) {
    let mut zip = ZipWriter::new(File::create(path).expect("create fixture file"));

    zip.start_file(
        "mimetype",
        FileOptions::<()>::default().compression_method(CompressionMethod::Stored),
    )
    .expect("start mimetype");
    zip.write_all(b"application/epub+zip")
        .expect("write mimetype");

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
    .expect("start container");
    zip.write_all(container).expect("write container");

    let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="2.0" xmlns="http://www.idpf.org/2007/opf">
  <metadata>
    <dc:title>Nested ToC Fixture</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="ch01" href="ch01.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="ch01"/>
  </spine>
</package>
"#
    .to_string();
    zip.start_file(
        "OEBPS/content.opf",
        FileOptions::<()>::default().compression_method(CompressionMethod::Deflated),
    )
    .expect("start content.opf");
    zip.write_all(opf.as_bytes()).expect("write content.opf");

    let ncx = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ncx version="2005-1" xmlns="http://www.daisy.org/z3986/2005/ncx/">
  <head>
    <meta name="dtb:uid" content="nested-toc"/>
    <meta name="dtb:depth" content="2"/>
  </head>
  <docTitle><text>Nested ToC Fixture</text></docTitle>
  <navMap>
    <navPoint id="np-1" playOrder="1">
      <navLabel><text>{chapter_title}</text></navLabel>
      <content src="ch01.xhtml"/>
      <navPoint id="np-1-1" playOrder="2">
        <navLabel><text>{fragment_label}</text></navLabel>
        <content src="ch01.xhtml#refs"/>
      </navPoint>
    </navPoint>
  </navMap>
</ncx>
"#
    );
    zip.start_file(
        "OEBPS/toc.ncx",
        FileOptions::<()>::default().compression_method(CompressionMethod::Deflated),
    )
    .expect("start toc.ncx");
    zip.write_all(ncx.as_bytes()).expect("write toc.ncx");

    let chapter = br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter</title></head>
<body><h1>Chapter</h1><p>Text.</p></body>
</html>
"#;
    zip.start_file(
        "OEBPS/ch01.xhtml",
        FileOptions::<()>::default().compression_method(CompressionMethod::Deflated),
    )
    .expect("start ch01");
    zip.write_all(chapter).expect("write ch01");

    zip.finish().expect("finish fixture zip");
}

#[test]
#[ignore = "requires original Agentic Design Patterns EPUB"]
fn read_original_agentic_epub_uses_top_level_titles() {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/sample.epub"
    ));
    if !path.exists() {
        return;
    }

    let book = read_epub(path).expect("read original epub");
    let chapter1 = book
        .chapters
        .iter()
        .find(|c| c.href.contains("Chapter1PromptChaining"))
        .expect("find Chapter 1");
    assert_eq!(
        chapter1.title.as_deref(),
        Some("Chapter 1: Prompt Chaining"),
        "Chapter 1 title should come from the top-level ToC entry, got {:?}",
        chapter1.title
    );
}

#[test]
fn read_epub_prefers_top_level_toc_title_over_fragment() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let fixture = temp_dir.path().join("nested_toc.epub");

    write_nested_toc_fixture(&fixture, "Chapter 1: Prompt Chaining", "References");

    let book = read_epub(&fixture).expect("read nested toc fixture");
    assert_eq!(book.chapters.len(), 1);
    assert_eq!(
        book.chapters[0].title.as_deref(),
        Some("Chapter 1: Prompt Chaining"),
        "top-level ToC title should win over fragment subsection label"
    );
}
