# Changelog

## [0.4.6] - 2026-07-12

- Release version 0.4.6.

## [0.4.5] - 2026-07-12

- Release version 0.4.5.

## [0.4.4] - 2026-07-12

- Release version 0.4.4.

## [0.4.3] - 2026-07-11

- Release version 0.4.3.

## [0.4.2] - 2026-07-11

- Release version 0.4.2.

## [0.4.1] - 2026-07-11

- Release version 0.4.1.

## [0.4.0] - 2026-07-11

### Changed

- Removed the prominent red validation banner and inline error messages from the
  translate page; required-field validation is now communicated only by disabling
  the Start/Dry-run buttons, keeping the form clean until the user is ready to act.

## [0.3.3] - 2026-07-11

### Added

- Desktop translate page now shows a configuration summary card and an **Estimate**
  (dry run) button next to Start Translation.
- Task queue supports multi-select, batch retry/cancel/remove, and up/down reordering
  for pending tasks.
- Logs page adds log-level filtering and pauses auto-scroll when the user scrolls up,
  with a "new logs" indicator and a jump-to-bottom control.
- Confirmation dialogs for destructive actions: removing tasks/providers and resetting
  custom prompts.
- `Ctrl+Enter` keyboard shortcut to start translation from the translate page.

### Changed

- Settings navigation is now grouped under a single **Settings** sidebar entry with
  tabbed sub-pages (Providers, Model, Translation, Output, Prompts, General).
- Provider cards were redesigned with clearer information hierarchy, accessible
  tooltips, and SVG show/hide icons for API keys.
- Numeric model parameters now validate ranges immediately and show min/max errors.
- "Compute" settings were relabeled to **Providers** and the prompt reset button now
  reads **Reset to defaults** in all locales.
- The translate page keeps the running/completed task visible in the progress panel
  instead of switching pages after start.
- License page renders the MIT text as readable paragraphs instead of a `<pre>` block.
- Source file picker shows a warning when a non-EPUB file is selected.
- Source file card now supports drag-and-drop highlighting and opens the file picker
  on drop for a smoother import flow.
- Empty states use consistent SVG icons instead of emoji for better cross-platform
  rendering and accessibility.
- Task error detail modal includes a **Copy error** button.
- Source and output paths on the translate page can be cleared with a single click.
- Alt+1..5 keyboard shortcuts jump to Translate, Logs, Tasks, Settings, and About.
- Settings tab icons are now SVG instead of emoji for consistent cross-platform
  rendering and accessibility.
- Provider cards and the translate-page provider selector show a type-specific SVG
  icon for each configured LLM provider.
- Task error detail modal can be closed with the Esc key.
- Sidebar navigation is split: Translate/Tasks/Logs stay at the top, while Settings
  and About are pushed to the bottom.
- Sidebar nav items now have SVG icons and "Task Queue" is renamed to "Tasks" in
  all locales.
- Sidebar width is now resizable via drag (140px–320px) and starts at a narrower
  180px default.
- Settings page header now places the "Settings" title and the horizontal sub-page
  tabs on the same line, saving vertical space.
- Settings sub-page tabs are forced into a single horizontal row with horizontal
  scrolling when needed, instead of wrapping vertically. (Root cause: the global
  `nav { flex-direction: column }` rule for the sidebar was also applying to the
  settings tab nav; now explicitly overridden.)

### Fixed

- Validation errors (missing source/output/API key) are now shown prominently on the
  translate page with direct links to the relevant settings.
- Running panel displays correct status labels (Running/Completed/Failed) and uses
  appropriate success/error colors.
- Unused CSS classes, the orphaned `LogPanel` component, and stale i18n keys were
  removed.

## [0.3.2] - 2026-07-11

### Fixed

- Desktop task queue now persists `chapter_total` and `chapters_completed` on each
  task, so refreshing the queue state no longer resets the progress bar to zero.
- Queue pause cancels the currently running translation and marks the task as
  `paused`; retrying resumes from the last checkpoint instead of restarting.
- Per-task pause is now available in the task list.
- Chapter chunk progress is now computed across the whole chapter, so translating
  multi-element chapters no longer loops the progress bar at the same percentage.
- Title translation no longer emits extra chunk events that could make the
  overall progress jump backwards.

### Changed

- Queue manager methods are now synchronous; Tauri commands remain unchanged.

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
