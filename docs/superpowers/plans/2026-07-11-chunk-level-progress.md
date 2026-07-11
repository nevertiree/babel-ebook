# Chunk-Level Translation Progress Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add per-chunk progress events so the UI logs each translated slice and the progress bar advances within a chapter.

**Architecture:** Extend `ProgressEvent` with `ChunkStarted`/`ChunkFinished`, pass the existing `ProgressCallback` through the HTML processing chain, emit events around every chunk, and update the React state to aggregate per-chapter chunk ratios into a single percentage.

**Tech Stack:** Rust, Tauri, React/TypeScript, i18next.

## Global Constraints

- Do not change unrelated UI (home-page provider/model dropdowns, settings storage, compute-page layout).
- Do not merge into `master`; release from `develop` as `v0.3.2`.
- All existing tests must keep passing.
- Run `cargo fmt`, `cargo clippy`, `cargo test`, `pnpm exec tsc --noEmit`, and `pnpm build` before tagging.

---

## Task 1: Extend the Rust `ProgressEvent` enum

**Files:**
- Modify: `crates/babel-ebook/src/core.rs:40-73`

**Interfaces:**
- Produces: two new enum variants consumed by Tauri backend event serialization and frontend `ProgressPayload`.

- [ ] **Step 1: Add `ChunkStarted` and `ChunkFinished` variants**

Insert after `ChapterFinished` and before `Failed`:

```rust
    /// A chunk of text inside a chapter has started translation.
    ChunkStarted {
        /// Index of the chapter in the EPUB spine.
        index: usize,
        /// Href of the chapter document.
        href: String,
        /// 0-based index of this chunk within the current text block.
        chunk_index: usize,
        /// Total number of chunks in the current text block.
        chunk_total: usize,
    },
    /// A chunk of text inside a chapter has finished translation.
    ChunkFinished {
        /// Index of the chapter in the EPUB spine.
        index: usize,
        /// Href of the chapter document.
        href: String,
        /// 0-based index of this chunk within the current text block.
        chunk_index: usize,
        /// Total number of chunks in the current text block.
        chunk_total: usize,
    },
```

- [ ] **Step 2: Verify `cargo check`**

Run: `cargo check -p babel-ebook`
Expected: compiles.

---

## Task 2: Plumb the progress callback through HTML processing

**Files:**
- Modify: `crates/babel-ebook/src/html.rs`
- Modify: `crates/babel-ebook/src/core.rs:319-328`
- Modify: `crates/babel-ebook/src/pipeline.rs:90`

**Interfaces:**
- Consumes: `Option<&dyn ProgressCallback>` from the pipeline.
- Produces: `translate_text(..., chapter_index, chapter_href, progress)` and `process_document(..., chapter_index, chapter_href, progress)`.

- [ ] **Step 1: Update `translate_text` signature and emit chunk events**

Change the signature:

```rust
pub async fn translate_text(
    text: &str,
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    chapter_index: usize,
    chapter_href: &str,
    progress: Option<&dyn ProgressCallback>,
) -> Result<String, BabelEbookError> {
```

In the first-pass loop (after `let chunks = split_text_chunks(text, max_source);`):

```rust
        let chunk_total = chunks.len();
        let mut translated_parts = Vec::with_capacity(chunk_total);
        for (chunk_index, chunk) in chunks.iter().enumerate() {
            emit_chunk_progress(
                progress,
                chapter_index,
                chapter_href,
                chunk_index,
                chunk_total,
                false,
            );
            if let Some(cached) = cache.get(&translator.name(), chunk) {
                translated_parts.push(cached);
                emit_chunk_progress(
                    progress,
                    chapter_index,
                    chapter_href,
                    chunk_index,
                    chunk_total,
                    true,
                );
                continue;
            }

            let context = TranslateContext {
                system_prompt: &system_prompt,
                target_lang,
            };
            let result = translator.translate(chunk, &context).await?;
            let tokens = count_tokens(chunk) + count_tokens(&result);
            cache.put(&translator.name(), chunk, &result, Some(tokens));
            translated_parts.push(result);
            emit_chunk_progress(
                progress,
                chapter_index,
                chapter_href,
                chunk_index,
                chunk_total,
                true,
            );
        }
```

Do the same for the refinement loop (after `let chunks = split_text_chunks(&first_pass, max_refine_source);`):

```rust
        let chunk_total = chunks.len();
        let mut refined_parts = Vec::with_capacity(chunk_total);
        for (chunk_index, chunk) in chunks.iter().enumerate() {
            emit_chunk_progress(
                progress,
                chapter_index,
                chapter_href,
                chunk_index,
                chunk_total,
                false,
            );
            if let Some(cached) = cache.get(&refine_name, chunk) {
                refined_parts.push(cached);
                emit_chunk_progress(
                    progress,
                    chapter_index,
                    chapter_href,
                    chunk_index,
                    chunk_total,
                    true,
                );
                continue;
            }

            let context = TranslateContext {
                system_prompt: &refine_prompt,
                target_lang,
            };
            let result = translator.translate(chunk, &context).await?;
            let tokens = count_tokens(chunk) + count_tokens(&result);
            cache.put(&refine_name, chunk, &result, Some(tokens));
            refined_parts.push(result);
            emit_chunk_progress(
                progress,
                chapter_index,
                chapter_href,
                chunk_index,
                chunk_total,
                true,
            );
        }
```

Add the helper at the bottom of `html.rs`:

```rust
fn emit_chunk_progress(
    progress: Option<&dyn crate::core::ProgressCallback>,
    index: usize,
    href: &str,
    chunk_index: usize,
    chunk_total: usize,
    finished: bool,
) {
    let event = if finished {
        crate::core::ProgressEvent::ChunkFinished {
            index,
            href: href.to_string(),
            chunk_index,
            chunk_total,
        }
    } else {
        crate::core::ProgressEvent::ChunkStarted {
            index,
            href: href.to_string(),
            chunk_index,
            chunk_total,
        }
    };
    if let Some(p) = progress {
        p.on_progress(event);
    }
}
```

- [ ] **Step 2: Update `process_document` and `translate_element_text_and_attributes`**

Change `process_document`:

```rust
pub async fn process_document(
    html: &[u8],
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    chapter_index: usize,
    chapter_href: &str,
    progress: Option<&dyn ProgressCallback>,
) -> Result<Vec<u8>, BabelEbookError> {
```

Pass `chapter_index`, `chapter_href`, `progress` into both `translate_element_text_and_attributes` calls.

Change `translate_element_text_and_attributes` signature and pass the same parameters into both `translate_text` calls.

- [ ] **Step 3: Update `translate_title` in `core.rs`**

Change signature:

```rust
async fn translate_title(
    index: usize,
    title: &str,
    translator: &dyn Translator,
    config: &Config,
    cache: &TranslationCache,
    href: &str,
    progress: Option<&dyn ProgressCallback>,
) -> Result<String, BabelEbookError> {
    let prompt_key = format!("toc:{href}");
    translate_text(title, translator, config, cache, index, &prompt_key, progress).await
}
```

Update its call site inside `translate_epub_with_cancellation` to pass `*index`, `&title`, ..., `*index`, `&href`, `progress`.

- [ ] **Step 4: Update the pipeline call site**

In `crates/babel-ebook/src/pipeline.rs:90`, change:

```rust
process_document(&content, translator, config, cache, index, &href, progress).await
```

- [ ] **Step 5: Verify `cargo check`**

Run: `cargo check -p babel-ebook`
Expected: compiles.

---

## Task 3: Update Rust tests for new signatures

**Files:**
- Modify: `crates/babel-ebook/tests/test_html.rs`

**Interfaces:**
- Consumes: new `translate_text(..., chapter_index, chapter_href, progress)` and `process_document(..., chapter_index, chapter_href, progress)` signatures.

- [ ] **Step 1: Add `0, "", None` to every `translate_text` call**

For example:

```rust
let result = translate_text("hello world", &FakeTranslator, &config, &cache, 0, "", None).await;
```

- [ ] **Step 2: Add `0, "", None` to every `process_document` call**

For example:

```rust
let out = process_document(html.as_bytes(), &FakeTranslator, &config, &cache, 0, "", None).await?;
```

- [ ] **Step 3: Run Rust tests**

Run: `cargo test -p babel-ebook`
Expected: all tests pass.

---

## Task 4: Extend the frontend payload type and progress logic

**Files:**
- Modify: `desktop/src/types.ts`
- Modify: `desktop/src/App.tsx`

**Interfaces:**
- Consumes: `ChunkStarted`/`ChunkFinished` from `translation_progress` and `task_progress` events.
- Produces: updated `Task` with `chapter_progress` and `chapters_completed`; updated `applyProgressToTask`; updated direct-translation progress listener.

- [ ] **Step 1: Extend `Task` and `ProgressPayload` types**

In `desktop/src/types.ts`, append to `Task`:

```ts
  chapter_progress?: Record<number, { chunk_total: number; chunks_done: number }>;
  chapters_completed?: number;
```

In `desktop/src/App.tsx`, update `ProgressPayload`:

```ts
type ProgressPayload =
  | { Started: { total: number } }
  | { ChapterStarted: { index: number; href: string } }
  | { ChapterFinished: { index: number; href: string } }
  | { ChunkStarted: { index: number; href: string; chunk_index: number; chunk_total: number } }
  | { ChunkFinished: { index: number; href: string; chunk_index: number; chunk_total: number } }
  | { Failed: { index: number; href: string; error: string } }
  | "Completed";
```

- [ ] **Step 2: Add a progress helper**

Add before `applyProgressToTask`:

```ts
function computeChunkProgressPercent(
  chapterTotal: number,
  chaptersCompleted: number,
  chapterProgress: Record<number, { chunk_total: number; chunks_done: number }>
): number {
  if (chapterTotal <= 0) return 0;
  const inFlight = Object.values(chapterProgress).reduce((sum, p) => {
    if (p.chunk_total <= 0) return sum;
    return sum + p.chunks_done / p.chunk_total;
  }, 0);
  return Math.min(99, Math.round(((chaptersCompleted + inFlight) / chapterTotal) * 100));
}
```

- [ ] **Step 3: Update `applyProgressToTask`**

Reset state on `Started`:

```ts
  if (typeof payload === "object" && "Started" in payload) {
    return {
      ...task,
      progress_percent: 0,
      status: "running",
      chapter_total: payload.Started.total,
      chapter_progress: {},
      chapters_completed: 0,
      error: undefined,
    };
  }
```

Handle `ChapterStarted`:

```ts
  if (typeof payload === "object" && "ChapterStarted" in payload) {
    const total = task.chapter_total ?? 0;
    const chapterProgress = { ...(task.chapter_progress ?? {}) };
    chapterProgress[payload.ChapterStarted.index] = chapterProgress[payload.ChapterStarted.index] ?? { chunk_total: 1, chunks_done: 0 };
    const completed = task.chapters_completed ?? 0;
    return {
      ...task,
      progress_percent: computeChunkProgressPercent(total, completed, chapterProgress),
      status: "running",
      chapter_progress: chapterProgress,
    };
  }
```

Handle `ChapterFinished`:

```ts
  if (typeof payload === "object" && "ChapterFinished" in payload) {
    const total = task.chapter_total ?? 0;
    const chapterProgress = { ...(task.chapter_progress ?? {}) };
    delete chapterProgress[payload.ChapterFinished.index];
    const completed = (task.chapters_completed ?? 0) + 1;
    return {
      ...task,
      progress_percent: computeChunkProgressPercent(total, completed, chapterProgress),
      status: "running",
      chapter_progress: chapterProgress,
      chapters_completed: completed,
    };
  }
```

Add chunk handling:

```ts
  if (typeof payload === "object" && "ChunkStarted" in payload) {
    const total = task.chapter_total ?? 0;
    const chapterProgress = { ...(task.chapter_progress ?? {}) };
    chapterProgress[payload.ChunkStarted.index] = {
      chunk_total: payload.ChunkStarted.chunk_total,
      chunks_done: payload.ChunkStarted.chunk_index,
    };
    return {
      ...task,
      progress_percent: computeChunkProgressPercent(total, task.chapters_completed ?? 0, chapterProgress),
      status: "running",
      chapter_progress: chapterProgress,
    };
  }
  if (typeof payload === "object" && "ChunkFinished" in payload) {
    const total = task.chapter_total ?? 0;
    const chapterProgress = { ...(task.chapter_progress ?? {}) };
    chapterProgress[payload.ChunkFinished.index] = {
      chunk_total: payload.ChunkFinished.chunk_total,
      chunks_done: payload.ChunkFinished.chunk_index + 1,
    };
    return {
      ...task,
      progress_percent: computeChunkProgressPercent(total, task.chapters_completed ?? 0, chapterProgress),
      status: "running",
      chapter_progress: chapterProgress,
    };
  }
```

- [ ] **Step 4: Update the direct `translation_progress` listener**

Add a ref after existing refs:

```ts
  const chapterProgressRef = useRef<Record<number, { chunk_total: number; chunks_done: number }>>({});
```

Reset it on `Started`:

```ts
        if (typeof payload === "object" && "Started" in payload) {
          totalRef.current = payload.Started.total;
          completedRef.current = 0;
          chapterProgressRef.current = {};
          ...
        }
```

Add chunk log entries:

```ts
        if (typeof payload === "object" && "ChunkStarted" in payload) {
          chapterProgressRef.current[payload.ChunkStarted.index] = {
            chunk_total: payload.ChunkStarted.chunk_total,
            chunks_done: payload.ChunkStarted.chunk_index,
          };
          return [
            ...prev,
            {
              id: generateId(),
              timestamp: Date.now(),
              kind: "chapter",
              message: t("log_chunk_started", {
                chunk_index: payload.ChunkStarted.chunk_index + 1,
                chunk_total: payload.ChunkStarted.chunk_total,
                href: payload.ChunkStarted.href,
              }),
            },
          ];
        }
        if (typeof payload === "object" && "ChunkFinished" in payload) {
          chapterProgressRef.current[payload.ChunkFinished.index] = {
            chunk_total: payload.ChunkFinished.chunk_total,
            chunks_done: payload.ChunkFinished.chunk_index + 1,
          };
          return [
            ...prev,
            {
              id: generateId(),
              timestamp: Date.now(),
              kind: "chapter",
              message: t("log_chunk_finished", {
                chunk_index: payload.ChunkFinished.chunk_index + 1,
                chunk_total: payload.ChunkFinished.chunk_total,
                href: payload.ChunkFinished.href,
              }),
            },
          ];
        }
```

Update the `setProgress` block to use `computeChunkProgressPercent` for chapter/chunk events:

```ts
      setProgress((prev) => {
        if (typeof payload === "string" && payload === "Completed") {
          return { percent: 100, message: t("completed") };
        }
        if (typeof payload === "object" && "Started" in payload) {
          totalRef.current = payload.Started.total;
          completedRef.current = 0;
          chapterProgressRef.current = {};
          return { percent: 0, message: t("started") };
        }
        if (typeof payload === "object" && "ChapterStarted" in payload) {
          return {
            ...prev,
            message: `${t("started")}: ${payload.ChapterStarted.href}`,
          };
        }
        if (typeof payload === "object" && "ChapterFinished" in payload) {
          completedRef.current += 1;
          delete chapterProgressRef.current[payload.ChapterFinished.index];
          const percent = computeChunkProgressPercent(
            totalRef.current,
            completedRef.current,
            chapterProgressRef.current
          );
          return {
            percent,
            message: `${t("completed")}: ${payload.ChapterFinished.href}`,
          };
        }
        if (typeof payload === "object" && "ChunkStarted" in payload) {
          chapterProgressRef.current[payload.ChunkStarted.index] = {
            chunk_total: payload.ChunkStarted.chunk_total,
            chunks_done: payload.ChunkStarted.chunk_index,
          };
          return {
            ...prev,
            percent: computeChunkProgressPercent(
              totalRef.current,
              completedRef.current,
              chapterProgressRef.current
            ),
          };
        }
        if (typeof payload === "object" && "ChunkFinished" in payload) {
          chapterProgressRef.current[payload.ChunkFinished.index] = {
            chunk_total: payload.ChunkFinished.chunk_total,
            chunks_done: payload.ChunkFinished.chunk_index + 1,
          };
          return {
            ...prev,
            percent: computeChunkProgressPercent(
              totalRef.current,
              completedRef.current,
              chapterProgressRef.current
            ),
          };
        }
        if (typeof payload === "object" && "Failed" in payload) {
          return {
            ...prev,
            message: `${t("error")}: ${payload.Failed.error}`,
          };
        }
        return prev;
      });
```

- [ ] **Step 5: Type-check the frontend**

Run: `cd desktop && pnpm exec tsc --noEmit`
Expected: no type errors.

---

## Task 5: Add i18n log messages

**Files:**
- Modify: `desktop/src/locales/en.json`
- Modify: `desktop/src/locales/zh-CN.json`
- Modify: `desktop/src/locales/es.json`, `ja.json`, `ko.json`, `ru.json`

- [ ] **Step 1: Add English strings**

After `log_chapter_failed` in `en.json`:

```json
  "log_chunk_started": "Translating chunk {{chunk_index}}/{{chunk_total}} of {{href}}",
  "log_chunk_finished": "Finished chunk {{chunk_index}}/{{chunk_total}} of {{href}}",
```

- [ ] **Step 2: Add Chinese strings**

After `log_chapter_failed` in `zh-CN.json`:

```json
  "log_chunk_started": "正在翻译切片 {{chunk_index}}/{{chunk_total}}：{{href}}",
  "log_chunk_finished": "已完成切片 {{chunk_index}}/{{chunk_total}}：{{href}}",
```

- [ ] **Step 3: Add English fallbacks to other locales**

Add the same English strings to `es.json`, `ja.json`, `ko.json`, `ru.json` after `log_chapter_failed`.

---

## Task 6: Run the full quality gates

- [ ] **Step 1: Rust gates**

Run:

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected: all pass.

- [ ] **Step 2: Desktop gates**

Run:

```bash
cd desktop
pnpm exec tsc --noEmit
pnpm build
```

Expected: type-check and production build succeed.

---

## Task 7: Release v0.3.2 from develop

- [ ] **Step 1: Create release branch**

```bash
git checkout develop
git pull origin develop
git checkout -b release/v0.3.2
```

- [ ] **Step 2: Bump version**

```bash
cd desktop
pnpm version:bump patch
```

This updates `Cargo.toml`, `Cargo.lock`, `desktop/package.json`, `desktop/src-tauri/tauri.conf.json`, `CHANGELOG.md`, commits, and creates the annotated tag `v0.3.2`.

- [ ] **Step 3: Push branch and tag**

```bash
git push origin release/v0.3.2
git push origin v0.3.2
```

- [ ] **Step 4: Build release artifacts**

```bash
cd desktop
pnpm release:build
```

Expected: artifacts land in `release/v0.3.2/`.

- [ ] **Step 5: Report release URL**

After the CI/artifacts are ready, report the GitHub release URL to the user. Do not merge into `master`.
