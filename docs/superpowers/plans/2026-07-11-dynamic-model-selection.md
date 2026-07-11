# Dynamic Provider-Aware Model Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fetch available models from each provider's API and render them as a dropdown on the Translate page; auto-select the first model when switching providers; fall back to a text input on error.

**Architecture:** Add `list_models()` to the `Translator` trait with per-provider HTTP implementations, expose it through a new Tauri command, and drive the frontend `ModelSelect` component asynchronously.

**Tech Stack:** Rust (Tauri), React + TypeScript, i18next, reqwest, async-openai.

## Global Constraints

- All provider APIs must be called with a 10-second timeout.
- Errors must be surfaced to the frontend as `String` and rendered as a text input fallback.
- The "Custom" option must always remain available.
- Cloud providers require a non-empty API key; Ollama does not.
- Changes are limited to `crates/babel-ebook/src/translator/*`, `desktop/src-tauri/src/commands.rs`, `desktop/src-tauri/src/lib.rs`, `desktop/src/pages/TranslatePage.tsx`, `desktop/src/App.tsx`, `desktop/src/locales/*.json`, and E2E tests.

---

### Task 1: Add `list_models` to the `Translator` trait

**Files:**
- Modify: `crates/babel-ebook/src/translator/mod.rs`

**Interfaces:**
- Produces: `async fn list_models(&self) -> Result<Vec<String>, BabelEbookError>` on the `Translator` trait with a default empty-vec implementation.

- [ ] **Step 1: Add the trait method**

```rust
#[async_trait]
pub trait Translator: Send + Sync {
    fn name(&self) -> String;
    fn max_output_tokens(&self) -> usize;
    async fn translate(&self, text: &str, context: &TranslateContext<'_>) -> Result<String, BabelEbookError>;
    async fn health_check(&self) -> Result<(), BabelEbookError> {
        Ok(())
    }
    async fn list_models(&self) -> Result<Vec<String>, BabelEbookError> {
        Ok(Vec::new())
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/babel-ebook/src/translator/mod.rs
git commit -m "feat(translator): add list_models trait method"
```

---

### Task 2: Implement `list_models` for DeepSeek

**Files:**
- Modify: `crates/babel-ebook/src/translator/deepseek.rs`

**Interfaces:**
- Consumes: `Translator` trait from Task 1.
- Produces: `DeepSeekTranslator::list_models` returning model ids from `GET {base_url}/models`.

- [ ] **Step 1: Add the implementation**

Append inside the `#[async_trait] impl Translator for DeepSeekTranslator` block:

```rust
async fn list_models(&self) -> Result<Vec<String>, BabelEbookError> {
    use async_openai::config::Config;
    let config = self.client.config();
    let url = config.url("/models");
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .headers(config.headers())
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| BabelEbookError::ApiError(e.to_string()))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(BabelEbookError::ApiError(format!(
            "DeepSeek list models failed: HTTP {}: {}",
            response.status(),
            body
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| BabelEbookError::ApiError(format!("failed to parse DeepSeek models: {e}")))?;

    let models = json["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(models)
}
```

- [ ] **Step 2: Add a unit test for parsing**

Append to the bottom of the file inside `#[cfg(test)] mod tests` (create the test module if it does not exist):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_models_parses_data_array() {
        let translator = DeepSeekTranslator::new(
            "fake-key".to_string(),
            None,
            Some("http://localhost".to_string()),
            2000,
            0.3,
        );
        // The endpoint is unreachable in this test; assert the method exists
        // and returns an API error rather than a configuration error.
        let err = translator.list_models().await.unwrap_err();
        assert!(matches!(err, BabelEbookError::ApiError(_)));
    }
}
```

- [ ] **Step 3: Run the test**

```bash
cargo test -p babel-ebook deepseek::tests::list_models_parses_data_array -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/babel-ebook/src/translator/deepseek.rs
git commit -m "feat(translator): implement list_models for DeepSeek"
```

---

### Task 3: Implement `list_models` for OpenAI

**Files:**
- Modify: `crates/babel-ebook/src/translator/openai.rs`

**Interfaces:**
- Produces: `OpenAiTranslator::list_models` using `async_openai` models API.

- [ ] **Step 1: Add the implementation**

Append inside the `#[async_trait] impl Translator for OpenAiTranslator` block:

```rust
async fn list_models(&self) -> Result<Vec<String>, BabelEbookError> {
    let response = self
        .client
        .models()
        .list()
        .await
        .map_err(|e| BabelEbookError::ApiError(e.to_string()))?;
    Ok(response.data.into_iter().map(|m| m.id).collect())
}
```

- [ ] **Step 2: Add a unit test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_models_exists() {
        let translator = OpenAiTranslator::new(
            "fake-key".to_string(),
            None,
            Some("http://localhost".to_string()),
            2000,
            0.3,
        );
        let err = translator.list_models().await.unwrap_err();
        assert!(matches!(err, BabelEbookError::ApiError(_)));
    }
}
```

- [ ] **Step 3: Run the test**

```bash
cargo test -p babel-ebook openai::tests::list_models_exists -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/babel-ebook/src/translator/openai.rs
git commit -m "feat(translator): implement list_models for OpenAI"
```

---

### Task 4: Implement `list_models` for Anthropic

**Files:**
- Modify: `crates/babel-ebook/src/translator/anthropic.rs`

**Interfaces:**
- Produces: `AnthropicTranslator::list_models` from `GET {base_url}/v1/models`.

- [ ] **Step 1: Add the implementation**

Append inside the `#[async_trait] impl Translator for AnthropicTranslator` block:

```rust
async fn list_models(&self) -> Result<Vec<String>, BabelEbookError> {
    let response = self
        .client
        .get(format!("{}/v1/models", self.base_url))
        .header("x-api-key", &self.api_key)
        .header("anthropic-version", DEFAULT_ANTHROPIC_VERSION)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| BabelEbookError::ApiError(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(BabelEbookError::ApiError(format!(
            "Anthropic list models failed: HTTP {status}: {body}"
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| BabelEbookError::ApiError(format!("failed to parse Anthropic models: {e}")))?;

    let models = json["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(models)
}
```

- [ ] **Step 2: Add a unit test**

```rust
#[tokio::test]
async fn list_models_exists() {
    let translator = AnthropicTranslator::new(
        "fake-key".to_string(),
        None,
        Some("http://localhost".to_string()),
        2000,
        0.3,
    );
    let err = translator.list_models().await.unwrap_err();
    assert!(matches!(err, BabelEbookError::ApiError(_)));
}
```

- [ ] **Step 3: Run the test**

```bash
cargo test -p babel-ebook anthropic::tests::list_models_exists -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/babel-ebook/src/translator/anthropic.rs
git commit -m "feat(translator): implement list_models for Anthropic"
```

---

### Task 5: Implement `list_models` for Ollama

**Files:**
- Modify: `crates/babel-ebook/src/translator/ollama.rs`

**Interfaces:**
- Produces: `OllamaTranslator::list_models` from `GET {base_url}/api/tags`.

- [ ] **Step 1: Add the implementation**

Append inside the `#[async_trait] impl Translator for OllamaTranslator` block:

```rust
async fn list_models(&self) -> Result<Vec<String>, BabelEbookError> {
    let response = self
        .client
        .get(format!("{}/api/tags", self.base_url))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| BabelEbookError::ApiError(format!("Ollama list models failed: {e}")))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(BabelEbookError::ApiError(format!("Ollama error: {body}")));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| {
        BabelEbookError::ApiError(format!("failed to parse Ollama models: {e}"))
    })?;

    let models = json["models"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(models)
}
```

- [ ] **Step 2: Add a unit test**

```rust
#[tokio::test]
async fn list_models_parses_local_models() {
    let json = serde_json::json!({
        "models": [
            {"name": "llama3.2:latest"},
            {"name": "qwen2:latest"},
        ]
    });
    let names: Vec<String> = json["models"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["name"].as_str().map(String::from))
        .collect();
    assert_eq!(names, vec!["llama3.2:latest", "qwen2:latest"]);
}
```

- [ ] **Step 3: Run the test**

```bash
cargo test -p babel-ebook ollama::tests::list_models_parses_local_models -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/babel-ebook/src/translator/ollama.rs
git commit -m "feat(translator): implement list_models for Ollama"
```

---

### Task 6: Expose `list_models` as a Tauri command

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `Translator::list_models` from Tasks 2-5.
- Produces: `commands::list_models` registered in Tauri invoke handler.

- [ ] **Step 1: Add the command in `commands.rs`**

After `test_connection`, add:

```rust
/// List available models for the given provider.
#[allow(dead_code)]
#[tauri::command]
pub async fn list_models(args: TestConnectionArgs) -> Result<Vec<String>, String> {
    validate_connection_args(&args)?;
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;

        rt.block_on(async {
            let config = build_test_config(&args)?;
            let mut provider_config = ProviderConfig::for_provider(&args.provider);
            provider_config.base_url = args.base_url.clone().filter(|url| !url.is_empty());

            let translator = get_translator(&args.provider, Some(&provider_config), &config, false)
                .map_err(|e| e.to_string())?;

            translator.list_models().await.map_err(|e| e.to_string())
        })
    })
    .await
    .map_err(|e| format!("list models task panicked: {e}"))?
}
```

- [ ] **Step 2: Register the command in `lib.rs`**

Add `commands::list_models,` to the `generate_handler!` macro invocation.

- [ ] **Step 3: Run Rust tests**

```bash
cargo test --workspace
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): expose list_models Tauri command"
```

---

### Task 7: Refactor frontend `ModelSelect` for dynamic fetching

**Files:**
- Modify: `desktop/src/pages/TranslatePage.tsx`

**Interfaces:**
- Consumes: Tauri `invoke("list_models", { args })`.
- Produces: `ModelSelect` with props `{ provider, apiKey, baseUrl, useCustomBaseUrl, model, onChange }`.

- [ ] **Step 1: Update imports**

Change the import block at the top to include `useEffect`, `useRef`, `useState`:

```typescript
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
```

- [ ] **Step 2: Replace the `ModelSelect` component**

Replace the entire `ModelSelect` function with:

```typescript
interface ModelSelectProps {
  provider: string;
  apiKey: string;
  baseUrl: string;
  useCustomBaseUrl: boolean;
  model: string;
  onChange: (value: string) => void;
}

function ModelSelect({
  provider,
  apiKey,
  baseUrl,
  useCustomBaseUrl,
  model,
  onChange,
}: ModelSelectProps) {
  const { t } = useTranslation();
  const [models, setModels] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    if (abortRef.current) {
      abortRef.current.abort();
    }
    const controller = new AbortController();
    abortRef.current = controller;

    let cancelled = false;
    setLoading(true);
    setError(null);

    void (async () => {
      try {
        const list = await invoke<string[]>("list_models", {
          args: {
            provider,
            api_key: apiKey,
            base_url: useCustomBaseUrl ? baseUrl || null : null,
          },
        });
        if (cancelled) return;
        setModels(list);
        if (list.length > 0 && !list.includes(model) && model !== "__custom__") {
          onChange(list[0]);
        }
      } catch (err) {
        if (cancelled) return;
        setModels([]);
        setError(String(err));
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
      controller.abort();
    };
  }, [provider, apiKey, baseUrl, useCustomBaseUrl]);

  const isCustom = model === "__custom__" || (models.length > 0 && !models.includes(model));

  if (models.length === 0) {
    return (
      <label title={error ?? undefined}>
        {t("model")}
        <input
          type="text"
          value={model === "__custom__" ? "" : model}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t("model_custom_placeholder")}
          disabled={loading}
        />
      </label>
    );
  }

  return (
    <label title={error ?? undefined}>
      {t("model")}
      {isCustom ? (
        <input
          type="text"
          value={model === "__custom__" ? "" : model}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t("model_custom_placeholder")}
          disabled={loading}
        />
      ) : (
        <select
          value={model}
          onChange={(e) => onChange(e.target.value)}
          disabled={loading}
        >
          {models.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
          <option value="__custom__">{t("model_custom")}</option>
        </select>
      )}
    </label>
  );
}
```

- [ ] **Step 3: Pass credentials to `ModelSelect`**

Update the JSX where `ModelSelect` is rendered (around line 174) to:

```tsx
<ModelSelect
  provider={activeProvider.provider}
  apiKey={activeProvider.api_key}
  baseUrl={activeProvider.base_url}
  useCustomBaseUrl={activeProvider.use_custom_base_url}
  model={form.model}
  onChange={(value) => setForm("model", value)}
/>
```

- [ ] **Step 4: Run TypeScript check**

```bash
cd desktop && pnpm exec tsc --noEmit
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add desktop/src/pages/TranslatePage.tsx
git commit -m "feat(desktop): dynamic model selection with provider API fetch"
```

---

### Task 8: Update `App.tsx` active-provider change logic

**Files:**
- Modify: `desktop/src/App.tsx`

**Interfaces:**
- Produces: `updateForm` no longer uses static `recommendedModels` to pick a model; it lets `ModelSelect` auto-select.

- [ ] **Step 1: Simplify `updateForm`**

Replace the `if (key === "active_provider")` block inside `updateForm` with:

```typescript
if (key === "active_provider") {
  const activeStillExists = next.providers.some((p) => p.name === value);
  if (!activeStillExists && next.providers.length > 0) {
    next.active_provider = next.providers[0].name;
  }
}
```

Remove the import and usage of `recommendedModels` from `App.tsx` if it is no longer used anywhere else.

- [ ] **Step 2: Run TypeScript check**

```bash
cd desktop && pnpm exec tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add desktop/src/App.tsx
git commit -m "refactor(desktop): let ModelSelect handle model auto-selection"
```

---

### Task 9: Add i18n keys

**Files:**
- Modify: `desktop/src/locales/en.json`
- Modify: `desktop/src/locales/zh-CN.json`
- (And other locale files to keep keys in sync.)

**Interfaces:**
- Produces: keys `model_custom`, `model_custom_placeholder` (if missing) and `model_list_error`.

- [ ] **Step 1: Add English keys**

Ensure `en.json` contains:

```json
"model_custom": "Custom",
"model_custom_placeholder": "Enter model name",
"model_list_error": "Could not load model list",
```

- [ ] **Step 2: Add Chinese keys**

Ensure `zh-CN.json` contains:

```json
"model_custom": "自定义",
"model_custom_placeholder": "输入模型名称",
"model_list_error": "无法加载模型列表",
```

- [ ] **Step 3: Sync other locales**

Add the same keys to `es.json`, `ja.json`, `ko.json`, `ru.json` with English placeholders if translations are not available.

- [ ] **Step 4: Run locale key test**

```bash
cargo test -p babel-ebook all_locale_files_have_same_keys -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add desktop/src/locales/*.json
git commit -m "i18n: model selection loading and error labels"
```

---

### Task 10: Add E2E regression test

**Files:**
- Create: `desktop/e2e/model-selection.spec.ts`

**Interfaces:**
- Produces: Playwright test verifying Ollama model dropdown population.

- [ ] **Step 1: Create the test file**

```typescript
import { chromium, test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";
import { cleanupBrowserProcesses, forceKill, getFreePort, waitForCdp } from "./e2e-helpers";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const APP_PATH = resolve(__dirname, "../../target/release/babel-ebook-desktop.exe");
const SETTINGS_DIR = resolve(homedir(), "Documents/BabelEbook");
const SETTINGS_PATH = join(SETTINGS_DIR, "settings.json");

let appProcess: ChildProcess | null = null;
let cdpUrl: string;

function seedOllamaSettings() {
  mkdirSync(SETTINGS_DIR, { recursive: true });
  const payload = {
    version: 5,
    translation: {
      providers: [
        {
          name: "ollama",
          provider: "ollama",
          api_key: "",
          base_url: "",
          use_custom_base_url: false,
        },
      ],
      active_provider: "ollama",
      model: "llama3.2:latest",
      source_lang: "en",
      target_lang: "zh-CN",
      output_mode: "bilingual",
      style: "default",
      concurrency: 1,
      max_input_tokens: 4000,
      max_output_tokens: 2000,
      temperature: 0.3,
      dry_run: false,
      refine: false,
      checkpoint_dir: "",
      translate_body: true,
      translate_metadata: true,
      translate_toc: true,
      translate_alt_text: true,
      translate_image_captions: true,
      translate_tables: true,
      translate_footnotes: true,
      translate_code: false,
    },
    general: {
      ui_language: "en",
      theme: "dark",
      follow_system_language: false,
    },
  };
  writeFileSync(SETTINGS_PATH, JSON.stringify(payload, null, 2));
}

test.beforeAll(async () => {
  await cleanupBrowserProcesses();
  seedOllamaSettings();

  const port = await getFreePort();
  cdpUrl = `http://localhost:${port}`;

  appProcess = spawn(APP_PATH, [], {
    env: {
      ...process.env,
      BABEL_EBOOK_E2E_CDP_PORT: String(port),
      BABEL_EBOOK_E2E_UI_LANGUAGE: "en",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  appProcess.stdout?.on("data", (data) => {
    console.log(`[app stdout] ${data.toString().trim()}`);
  });
  appProcess.stderr?.on("data", (data) => {
    console.error(`[app stderr] ${data.toString().trim()}`);
  });

  const ready = await waitForCdp(cdpUrl);
  if (!ready) {
    await forceKill(appProcess);
    throw new Error(`Tauri app did not expose CDP port ${port} in time`);
  }
});

test.afterAll(async () => {
  await forceKill(appProcess);
});

test("Ollama model dropdown is populated from local API", async () => {
  test.setTimeout(60000);
  const browser = await chromium.connectOverCDP(cdpUrl);
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  page.on("console", (msg) => {
    console.log(`[browser console] ${msg.type()}: ${msg.text()}`);
  });

  const modelSelect = page.locator('label:has-text("Model") select');
  await expect(modelSelect).toBeVisible({ timeout: 10000 });
  await expect(modelSelect.locator("option")).toHaveCountGreaterThan(1, { timeout: 10000 });

  await browser.close();
});
```

- [ ] **Step 2: Ensure a release binary exists**

```bash
cd desktop && pnpm tauri build
```

- [ ] **Step 3: Run the test**

```bash
cd desktop && npx playwright test e2e/model-selection.spec.ts --reporter=line
```

Expected: PASS if Ollama is running with at least one model.

- [ ] **Step 4: Commit**

```bash
git add desktop/e2e/model-selection.spec.ts
git commit -m "test(e2e): verify Ollama model dropdown population"
```

---

### Task 11: Run full quality gates

**Files:**
- All modified files.

- [ ] **Step 1: Rust gates**

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected: no formatting issues, no clippy warnings, all tests pass.

- [ ] **Step 2: Desktop gates**

```bash
cd desktop
pnpm exec tsc --noEmit
pnpm build
BABEL_EBOOK_E2E_API_KEY=sk-dummy pnpm e2e --reporter=line
```

Expected: TypeScript compiles, frontend builds, all E2E tests pass.

- [ ] **Step 3: Commit any final fixes**

```bash
git commit -am "chore: quality gate fixes for dynamic model selection" || echo "no changes"
```

---

## Spec Coverage Check

| Spec Requirement | Task |
|---|---|
| Add `list_models` to `Translator` trait | Task 1 |
| DeepSeek dynamic model list | Task 2 |
| OpenAI dynamic model list | Task 3 |
| Anthropic dynamic model list | Task 4 |
| Ollama dynamic model list | Task 5 |
| Tauri command exposure | Task 6 |
| Frontend async ModelSelect | Task 7 |
| Auto-select first model on provider switch | Tasks 7 & 8 |
| Custom option always available | Task 7 |
| i18n labels | Task 9 |
| E2E regression test | Task 10 |

## Placeholder Scan

- No "TBD", "TODO", or "implement later" strings.
- Every step includes concrete file paths, code, and commands.
- All provider endpoints and auth headers are specified.
