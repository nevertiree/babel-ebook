# Changelog

## [0.3.2] - 2026-07-11

- Release version 0.3.2.

## [0.3.1] - 2026-07-11

- Release version 0.3.1.

## [0.3.0] - 2026-07-11

- Release version 0.3.0.

## [0.2.2] - 2026-07-10

### Added

- Resumable checkpoint list on the Translate page is shown automatically when a
  checkpoint directory is configured, with the task matching the current source
  file highlighted. Resuming a task created from a different source now shows a
  warning, and the active resume selection is surfaced next to Start Translation
  with a one-click clear button.
- A short cost note and a link to the refine prompt appear when the second-pass
  refine option is enabled.

### Fixed

- Desktop task queue can now cancel a running translation, so a mistaken target
  language no longer leaves the queue blocked until the current book finishes.
- The active theme is now applied to the document root so the base text color is
  inherited correctly everywhere.

## [0.2.1] - 2026-07-09

- Release version 0.2.1.

## [0.2.0] - 2026-07-09

- Release version 0.2.0.

## [0.1.2] - 2026-07-08

### Added

- New **Prompts** settings tab for customizing the system prompt and the prompt
  templates used by each translation style (default, literary, technical, academic).
  Templates support `{source_lang}` and `{target_lang}` placeholders and are sent
  to the translation backend.
- Backend diagnostics for the translation pipeline: source/output paths, EPUB load
  summary, per-chapter start/finish/failure events, and EPUB write confirmation.
- Frontend verification that the output EPUB file exists after translation; a clear
  error is shown if it is missing.

### Changed

- Model selection has moved from the **Model Parameters** settings tab to the main
  translate page, next to the active provider.
- **Compute** settings were redesigned to support multiple named provider configs
  with an inline row layout.
- Settings are now persisted under `Documents/BabelEbook/settings.json` so they
  survive app reinstallation.

### Fixed

- Progress bar no longer exceeds 100%; the counter was being incremented twice per
  chapter, and the percentage is now capped at 100%.
- Translated EPUBs are now written even when the output directory does not exist;
  the directory is created automatically.
- **Test Connection** no longer depends on a selected model and works for any
  configured provider.
- Empty custom prompt templates no longer overwrite the built-in defaults.

## [0.1.1] - 2026-07-06

### Fixed

- Desktop **Test Connection** button now passes the correct arguments to the
  `test_connection` command, so URL and API-key validation works for all
  providers (including DeepSeek).

### Internal

- Updated `crossbeam-epoch` to 0.9.20 to resolve `RUSTSEC-2026-0204` and keep
  `cargo-deny` passing.

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
