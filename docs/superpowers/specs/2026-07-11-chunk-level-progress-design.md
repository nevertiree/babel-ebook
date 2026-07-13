# Chunk-Level Translation Progress

## Problem

The current desktop UI only logs `ChapterStarted` and `ChapterFinished` progress events.
For books whose chapters are split into many LLM chunks (e.g. `Founding Brothers`), the
user sees a long silence between these two events and cannot tell whether the program is
still working.

## Goal

Emit a progress event for every text chunk that is translated, log it in the UI, and use
the chunk-level events to make the progress bar advance smoothly within each chapter.

## Design

### 1. Rust core: new progress events

In `crates/babel-ebook/src/core.rs`, extend the `ProgressEvent` enum with:

```rust
ChunkStarted {
    index: usize,       // chapter index in the EPUB spine
    href: String,       // chapter href
    chunk_index: usize, // 0-based chunk number within this text block
    chunk_total: usize, // total chunks in this text block
}
ChunkFinished {
    index: usize,
    href: String,
    chunk_index: usize,
    chunk_total: usize,
}
```

Both variants use the chapter `index` so the frontend can correlate chunk events with
the chapter that produced them.

### 2. Plumb the progress callback through the HTML pipeline

Add an optional progress callback parameter to:

- `translate_text` in `crates/babel-ebook/src/html.rs`
- `process_document` in the same file
- `translate_element_text_and_attributes` in the same file
- `translate_title` in `crates/babel-ebook/src/core.rs`

`translate_text` already splits text via `split_text_chunks`. Before and after each
chunk (including cache hits) it emits `ChunkStarted` / `ChunkFinished`. The second-pass
refinement loop does the same.

The call sites in the pipeline (`process_document`) and in `translate_epub`
(`translate_title`) pass the existing `Option<&dyn ProgressCallback>` down.

### 3. Frontend: new payload type and progress calculation

In `desktop/src/App.tsx`:

- Extend `ProgressPayload` with `ChunkStarted` and `ChunkFinished` cases.
- Add a per-task `chapter_chunk_progress` map keyed by chapter index. Each entry stores
  `chunk_total` and `chunks_done`.
- In `applyProgressToTask`, update the map on chunk events and compute:

```text
completed = number of fully finished chapters
in_flight = sum(chunks_done / chunk_total) for chapters that have started but not finished
percent   = min(99, round((completed + in_flight) / total_chapters * 100))
```

- In the direct `translation_progress` listener, keep the same map in component state
  (or derive it from the queue task if the task is queued) and update the top progress
  bar the same way.

### 4. Frontend: new log messages

Add i18n keys:

- `log_chunk_started`: e.g. "Translating chunk {{chunk_index}}/{{chunk_total}} of {{href}}"
- `log_chunk_finished`: e.g. "Finished chunk {{chunk_index}}/{{chunk_total}} of {{href}}"

Add them to `desktop/src/locales/en.json`, `desktop/src/locales/zh-CN.json`, and use
English fallbacks for the other locales.

### 5. Quality gates and release

After the change:

- `cargo fmt -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd desktop && pnpm exec tsc --noEmit && pnpm build`

Then create a `release/0.3.2` branch from `develop`, run `pnpm version:bump patch` to
produce the annotated tag `v0.3.2`, and build the release from that tag. Do **not** merge
into `master` yet, per the user's instruction.

## Out of scope

- Re-adding provider/model dropdowns on the home page.
- Changing settings storage from the OS keyring to a plain-text folder.
- Fixing the "config name input loses focus" issue on the compute page.

These are tracked separately and will not be touched by this change.
