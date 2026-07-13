# Dynamic Provider-Aware Model Selection

## Problem

The Translate page currently uses a static `recommendedModels` map to decide whether
to show a model `<select>` or a plain text `<input>`. This causes two issues:

1. **Provider switching does not update the model list.** If the saved `model`
   value is not in `recommendedModels[provider]`, the UI falls back to a text
   input even for providers that should have a dropdown.
2. **Static lists are stale or incomplete.** Ollama models are locally installed
   and dynamic; cloud providers also add new models regularly. A hard-coded list
   cannot reflect reality.

Users expect the model field on the home page to automatically present a
selectable list of models whenever they switch providers.

## Goal

When the active provider changes on the Translate page, fetch that provider's
available models from its API and render them as a dropdown. If the fetch fails,
fallback to a manual text input. When the fetch succeeds, automatically select
the first model if the current model is not in the returned list.

## Non-Goals

- Do not cache model lists across app restarts.
- Do not fetch models on the Compute settings page.
- Do not remove the "Custom" option; users must still be able to type arbitrary
  model names.

## Design

### Backend

1. **Extend the `Translator` trait** in
   `crates/babel-ebook/src/translator/mod.rs` with:

   ```rust
   async fn list_models(&self) -> Result<Vec<String>, BabelEbookError> {
       Ok(Vec::new())
   }
   ```

2. **Implement `list_models` per provider.**

   | Provider  | Endpoint                         | Auth / Headers |
   |-----------|----------------------------------|----------------|
   | DeepSeek  | `GET {base_url}/models`          | `Authorization: Bearer {api_key}` |
   | OpenAI    | `GET {base_url}/v1/models`       | `Authorization: Bearer {api_key}` |
   | Anthropic | `GET {base_url}/v1/models`       | `x-api-key`, `anthropic-version: 2023-06-01` |
   | Ollama    | `GET {base_url}/api/tags`        | none |

   All implementations parse a JSON response and return a `Vec<String>` of model
   ids/names. Errors are returned as `BabelEbookError::ApiError`.

3. **Add Tauri command** `list_models` in
   `desktop/src-tauri/src/commands.rs`:

   ```rust
   #[tauri::command]
   pub async fn list_models(args: TestConnectionArgs) -> Result<Vec<String>, String>
   ```

   It validates the args the same way `test_connection` does, builds a temporary
   provider config, obtains a translator via `get_translator`, and calls
   `translator.list_models().await`.

### Frontend

1. **Refactor `ModelSelect`** in `desktop/src/pages/TranslatePage.tsx`:

   - Props expand to include `apiKey`, `baseUrl`, `useCustomBaseUrl`.
   - Maintain local state: `models`, `loading`, `error`.
   - Use `useEffect` to call `invoke("list_models", ...)` whenever
     `provider`, `apiKey`, `baseUrl`, or `useCustomBaseUrl` changes.
   - On success:
     - Set `models` to the returned list.
     - If the current `model` value is not in the list and the list is non-empty,
       call `onChange(models[0])` to auto-select the first available model.
   - On error:
     - Clear `models` so the component renders a text input.
     - Surface a subtle hint that the list could not be loaded.
   - While loading:
     - Disable the dropdown and show a loading indicator.
   - Always append a `"__custom__"` option so users can type a model name.

2. **Update `TranslatePage`** to pass the active provider's credentials into
   `ModelSelect`.

3. **Update `App.tsx`** `updateForm` so that when `active_provider` changes it
   does not try to pick from the static `recommendedModels` map; the dynamic
   fetch in `ModelSelect` will handle auto-selection.

4. **Add i18n keys** for loading/error/custom labels in
   `desktop/src/locales/*.json`.

### Testing

1. **Unit tests** for each provider's `list_models` parsing using mocked
   `reqwest` responses or a test helper that parses sample JSON.
2. **E2E test** `desktop/e2e/model-selection.spec.ts`:
   - Launch the app against Ollama.
   - Switch to the Translate page.
   - Assert the model dropdown is populated with at least one locally installed
     model.
   - Switch provider config to a cloud provider (using a dummy key) and assert
     the dropdown updates or falls back to text input.

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| API key missing / invalid for cloud providers | Validate args and render text input on error; do not block the page |
| Ollama not running | Show text input with a hint; user can still type a model name |
| Provider API returns many models | Cap/truncate display if needed; not expected for local Ollama |
| Race condition when switching providers quickly | Abort previous fetch in `useEffect` cleanup |

## Open Questions

- Should the model list be sorted alphabetically or by provider-native order?
  Initial implementation preserves provider order.
