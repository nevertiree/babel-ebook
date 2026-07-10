#!/usr/bin/env python3
"""Generate a tiny multi-chapter EPUB for integration testing."""

import sys
import zipfile
from pathlib import Path


def make_epub(output: Path, chapters: int) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as zf:
        # mimetype must be uncompressed and first
        zf.writestr("mimetype", "application/epub+zip", zipfile.ZIP_STORED)

        zf.writestr(
            "META-INF/container.xml",
            """<?xml version="1.0" encoding="UTF-8"?>
<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
""",
        )

        manifest_items = []
        spine_items = []
        toc_navpoints = []
        for i in range(1, chapters + 1):
            href = f"ch{i}.xhtml"
            manifest_items.append(f'<item id="ch{i}" href="{href}" media-type="application/xhtml+xml"/>')
            spine_items.append(f'<itemref idref="ch{i}"/>')
            toc_navpoints.append(
                f'<navPoint id="navPoint-{i}" playOrder="{i}">'
                f'<navLabel><text>Chapter {i}</text></navLabel>'
                f'<content src="{href}"/>'
                f'</navPoint>'
            )
            zf.writestr(
                f"OEBPS/{href}",
                f"""<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
  <title>Chapter {i}</title>
</head>
<body>
  <h1>Chapter {i}</h1>
  <p>This is chapter {i} of the test book. It contains a few sentences so the translator has something to process.</p>
  <p>We want to verify that progress events arrive per chapter and that the queue UI updates smoothly.</p>
</body>
</html>
""",
            )

        manifest_block = "\n    ".join(manifest_items)
        spine_block = "\n    ".join(spine_items)
        toc_block = "\n    ".join(toc_navpoints)

        zf.writestr(
            "OEBPS/content.opf",
            f"""<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Multi Chapter Test</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">urn:uuid:multichapter-test</dc:identifier>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    {manifest_block}
  </manifest>
  <spine toc="ncx">
    {spine_block}
  </spine>
</package>
""",
        )

        zf.writestr(
            "OEBPS/toc.ncx",
            f"""<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta name="dtb:uid" content="urn:uuid:multichapter-test"/>
    <meta name="dtb:depth" content="1"/>
    <meta name="dtb:totalPageCount" content="0"/>
    <meta name="dtb:maxPageNumber" content="0"/>
  </head>
  <docTitle><text>Multi Chapter Test</text></docTitle>
  <navMap>
    {toc_block}
  </navMap>
</ncx>
""",
        )


if __name__ == "__main__":
    count = int(sys.argv[1]) if len(sys.argv) > 1 else 5
    out = Path(__file__).with_name(f"multichapter_{count}.epub")
    make_epub(out, count)
    print(f"Created {out}")
