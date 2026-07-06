# Changelog

## [0.1.1] - 2026-07-06

- Release version 0.1.1.

## [0.1.0] - 2026-07-06

### Added

- Input parameter validation across the core crate, CLI, and desktop GUI. Invalid
  paths, numeric ranges, provider names, locales, URLs, and unsafe strings are now
  rejected early with clear error messages.
- End User License Agreement (EULA) in both Markdown and plain-text form, wired
  into the Windows NSIS/WiX installers and accessible from the About page.
- New in-app Legal page that displays the full EULA.

### Fixed

- TOC labels in translated EPUBs no longer get overwritten by nested subsection
  entries (e.g. "References") that point to the same chapter file with a fragment.

## [0.2.2] - 2026-07-05

### Added

- Real-time output path suggestion on the translate page. The output path updates
  automatically when the source file, source/target language, output mode, or output
  filename template changes, defaulting to the system Downloads folder.
- "Remember API key securely" is now enabled by default for new installations.

### Changed

- Simplified the translate page by removing the duplicate configuration summary card;
  the top quick-settings row is now the single source of truth for provider, model,
  languages, and output mode.
- Removed the "dry-run" toggle from the translate page; real translations now always
  require an API key (ollama remains the exception).
- Removed the unused connection-status indicator from the translate page.

### Fixed

- Updated API-key requirement messages in all locale files to no longer reference
  the removed dry-run option.

## [0.2.1] - 2026-07-05

### Added

- New translation-scope toggles: body, metadata, toc, alt_text, image_captions, tables,
  footnotes, and code.
- Configurable EPUB splitting concurrency, max translation chunk length, and code
  translation option.
- Per-target-language default output font setting, injected into generated EPUBs.
- Customizable output filename template with `{stem}`, `{source_lang}`, `{target_lang}`,
  `{output_mode}` placeholders.
- Versioned local settings storage with migration from legacy flat keys.
- New **About** tab in the desktop app, showing app version, description, authors,
  license, homepage, and acknowledgments.
- New **Logs** tab in the desktop app sidebar, displaying all events with timestamps
  and a real-time search box.

### Fixed

- Table of contents (`toc`) titles are now translated when `translation_scope.toc` is enabled.
- `remember_api_key` preference is now preserved across restarts and the API key is only
  loaded from the OS credential store when the user opted to remember it. Switching
  providers now loads the remembered key for the selected provider.
- Desktop `suggest_output_path` now defaults to the system Downloads folder (e.g.
  `C:\Users\<user>\Downloads`) instead of the source EPUB directory.

### Internal

- Activated pre-commit hooks for Rust fmt/clippy/tests, cargo-deny, desktop typecheck/build,
  and markdown lint.
- Added GitHub Actions jobs for desktop builds and pre-commit checks.

## [0.2.0] - 2026-07-05

- Release version 0.2.0.
