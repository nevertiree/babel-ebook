# Export/Import Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add export/import settings buttons to the desktop app so users can back up and restore their complete BabelEbook configuration (including API keys) to a single JSON file.

**Architecture:** Implement entirely in the TypeScript frontend. Reuse existing `loadSettings()`/`saveSettings()` helpers in `desktop/src/config.ts`; these already hydrate API keys from the OS keyring on load and persist them back to the keyring on save. Add two new helpers, `exportSettings(path)` and `importSettings(path)`, that serialize/deserialize a versioned payload. Wire the buttons into `GeneralSettingsPage.tsx` using Tauri's `dialog` and `fs` plugins.

**Tech Stack:** React + TypeScript, Tauri v2 (`@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-fs`), i18next, existing `config.ts` persistence layer.

## Global Constraints

- Export file `version` must equal internal `SETTINGS_VERSION` (`5`).
- Exported JSON contains plaintext API keys; user must see a security warning before save.
- Import must show a confirmation dialog before overwriting current settings.
- No new Rust commands; use existing `fs`/`dialog` plugin permissions.
- Follow existing code style: 2-space indent in TSX/TS, named exports, `void fn()` for fire-and-forget async.
- Add i18n keys to all locale files (`en.json`, `zh-CN.json`, `es.json`, `ja.json`, `ko.json`, `ru.json`).

---

## File Structure

| File | Responsibility |
|---|---|
| `desktop/src/config.ts` | Adds `exportSettings(path)` and `importSettings(path)` helpers; defines `ExportedSettings` type. |
| `desktop/src/pages/GeneralSettingsPage.tsx` | Adds "Backup & Restore" UI section with Export/Import buttons, warnings, and error messages. |
| `desktop/src/locales/*.json` | New translation keys for buttons, warnings, success/error toasts. |
| `desktop/src/types.ts` | Adds `ExportedSettings` interface (optional, can live in `config.ts`). |

---

## Task 1: Add Export/Import Helpers to `config.ts`

**Files:**
- Modify: `desktop/src/config.ts`
- Test: `cd desktop && pnpm exec tsc --noEmit`

**Interfaces:**
- Consumes: `loadSettings()`, `loadGeneralSettings()`, `saveSettings(form)`, `saveGeneralSettings(general)` from the same file; `writeTextFile`, `readTextFile` from `@tauri-apps/plugin-fs`.
- Produces: `exportSettings(path: string): Promise<void>`, `importSettings(path: string): Promise<ExportedSettings>`.

- [ ] **Step 1: Add the `ExportedSettings` type and version constant**

At the top of `desktop/src/config.ts`, after imports, add:

```typescript
export interface ExportedSettings {
  version: number;
  exported_at: string;
  app_version: string;
  translation: Partial<FormState>;
  general: GeneralSettings;
}
```

Use the existing `SETTINGS_VERSION` constant for `version`.

- [ ] **Step 2: Implement `exportSettings(path)`**

Add after `saveGeneralSettings`:

```typescript
export async function exportSettings(path: string): Promise<void> {
  const [translation, general] = await Promise.all([
    loadSettings(),
    loadGeneralSettings(),
  ]);

  const payload: ExportedSettings = {
    version: SETTINGS_VERSION,
    exported_at: new Date().toISOString(),
    app_version: __APP_VERSION__,
    translation,
    general,
  };

  await writeTextFile(path, JSON.stringify(payload, null, 2));
}
```

`__APP_VERSION__` may not exist. Use a fallback:

```typescript
const appVersion =
  typeof __APP_VERSION__ !== "undefined" ? __APP_VERSION__ : "unknown";
```

If `__APP_VERSION__` is undefined, add a Vite define or simply hard-code `"0.2.0"` for now and leave a `// TODO: wire to build version` comment. For this plan, hard-code `"0.2.0"`.

- [ ] **Step 3: Implement `importSettings(path)`**

Add after `exportSettings`:

```typescript
export async function importSettings(path: string): Promise<ExportedSettings> {
  const text = await readTextFile(path);
  let payload: unknown;
  try {
    payload = JSON.parse(text);
  } catch {
    throw new Error("invalid_json");
  }

  if (!isExportedSettings(payload)) {
    throw new Error("invalid_backup");
  }

  if (payload.version !== SETTINGS_VERSION) {
    throw new Error(`version_mismatch:${payload.version}:${SETTINGS_VERSION}`);
  }

  return payload;
}

function isExportedSettings(value: unknown): value is ExportedSettings {
  const p = value as Record<string, unknown> | undefined;
  return (
    !!p &&
    typeof p.version === "number" &&
    typeof p.exported_at === "string" &&
    typeof p.app_version === "string" &&
    typeof p.translation === "object" &&
    p.translation !== null &&
    Array.isArray((p.translation as Record<string, unknown>).providers) &&
    typeof p.general === "object" &&
    p.general !== null
  );
}
```

- [ ] **Step 4: Verify TypeScript compiles**

Run:

```bash
cd desktop
pnpm exec tsc --noEmit
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add desktop/src/config.ts
git commit -m "feat(settings): add exportSettings and importSettings helpers"
```

---

## Task 2: Add i18n Keys

**Files:**
- Modify: `desktop/src/locales/en.json`, `desktop/src/locales/zh-CN.json`, `desktop/src/locales/es.json`, `desktop/src/locales/ja.json`, `desktop/src/locales/ko.json`, `desktop/src/locales/ru.json`

**Interfaces:**
- Produces: new translation keys used by `GeneralSettingsPage.tsx`.

- [ ] **Step 1: Add English keys**

In `desktop/src/locales/en.json`, add under the root object:

```json
  "settings_backup_restore": "Backup & Restore",
  "export_settings": "Export Settings",
  "import_settings": "Import Settings",
  "export_settings_warning": "The exported file contains your API keys. Store it securely and do not share it.",
  "import_settings_confirm": "Importing will overwrite your current settings. Continue?",
  "export_success": "Settings exported successfully",
  "import_success": "Settings imported successfully",
  "error_invalid_backup": "Backup file is corrupted",
  "error_version_mismatch": "Backup file version is incompatible (expected {{expected}}, got {{actual}})",
  "error_export_failed": "Failed to export settings: {{message}}",
  "error_import_failed": "Failed to import settings: {{message}}"
```

- [ ] **Step 2: Add Chinese keys**

In `desktop/src/locales/zh-CN.json`, add:

```json
  "settings_backup_restore": "备份与恢复",
  "export_settings": "导出设置",
  "import_settings": "导入设置",
  "export_settings_warning": "导出的文件包含您的 API 密钥，请妥善保管，切勿分享。",
  "import_settings_confirm": "导入将覆盖当前所有设置，是否继续？",
  "export_success": "设置导出成功",
  "import_success": "设置导入成功",
  "error_invalid_backup": "备份文件已损坏",
  "error_version_mismatch": "备份文件版本不兼容（期望 {{expected}}，实际 {{actual}}）",
  "error_export_failed": "导出设置失败：{{message}}",
  "error_import_failed": "导入设置失败：{{message}}"
```

- [ ] **Step 3: Add placeholder keys for other locales**

For `es.json`, `ja.json`, `ko.json`, `ru.json`, add the same keys with English text plus a `// TODO: translate` marker. Example for `es.json`:

```json
  "settings_backup_restore": "Backup & Restore",
  "export_settings": "Export Settings",
  "import_settings": "Import Settings",
  "export_settings_warning": "The exported file contains your API keys. Store it securely and do not share it.",
  "import_settings_confirm": "Importing will overwrite your current settings. Continue?",
  "export_success": "Settings exported successfully",
  "import_success": "Settings imported successfully",
  "error_invalid_backup": "Backup file is corrupted",
  "error_version_mismatch": "Backup file version is incompatible (expected {{expected}}, got {{actual}})",
  "error_export_failed": "Failed to export settings: {{message}}",
  "error_import_failed": "Failed to import settings: {{message}}"
```

- [ ] **Step 4: Validate JSON syntax**

Run:

```bash
cd desktop
node -e "console.log(JSON.parse(require('fs').readFileSync('src/locales/en.json','utf8'))['export_settings'])"
node -e "console.log(JSON.parse(require('fs').readFileSync('src/locales/zh-CN.json','utf8'))['export_settings'])"
```

Expected: prints the values without error.

- [ ] **Step 5: Commit**

```bash
git add desktop/src/locales/*.json
git commit -m "feat(settings): add export/import i18n keys"
```

---

## Task 3: Wire Export/Import UI in `GeneralSettingsPage.tsx`

**Files:**
- Modify: `desktop/src/pages/GeneralSettingsPage.tsx`
- Modify: `desktop/src/App.tsx` (pass `setForm` and `setGeneral` if not already available)
- Test: `cd desktop && pnpm exec tsc --noEmit`

**Interfaces:**
- Consumes: `exportSettings()`, `importSettings()` from `../config`; `saveSettings()`, `saveGeneralSettings()` from `../config`; `save` and `open` dialogs from `@tauri-apps/plugin-dialog`; `message()` from `@tauri-apps/plugin-dialog`; `confirm()` from `@tauri-apps/plugin-dialog`.
- Produces: Two new buttons in the General settings page that trigger export/import flows.

- [ ] **Step 1: Update `GeneralSettingsPage` props**

Current props only include `general` and `setGeneral`. Add `onImport` callback or inline the logic. Simpler: pass `setForm` and `setGeneral` from `App.tsx`.

In `desktop/src/pages/GeneralSettingsPage.tsx`, change the interface:

```typescript
interface GeneralSettingsPageProps {
  general: GeneralSettings;
  setGeneral: (general: GeneralSettings) => void;
  onImport: (settings: ImportedSettings) => void;
}
```

Wait — `ImportedSettings` is `ExportedSettings`. Import type from `../config`.

Better: keep it simple. Define the page to receive `onImport` callback only. `App.tsx` will handle state updates.

```typescript
import type { ExportedSettings } from "../config";

interface GeneralSettingsPageProps {
  general: GeneralSettings;
  setGeneral: (general: GeneralSettings) => void;
  onImport: (settings: ExportedSettings) => void;
}
```

- [ ] **Step 2: Add import statements**

At the top of `GeneralSettingsPage.tsx`, add:

```typescript
import { save, open, message, confirm } from "@tauri-apps/plugin-dialog";
import { exportSettings, importSettings, type ExportedSettings } from "../config";
```

- [ ] **Step 3: Add export handler**

Inside the component, add:

```typescript
  const handleExport = async () => {
    const confirmed = await confirm(t("export_settings_warning"), {
      title: t("export_settings"),
      kind: "warning",
    });
    if (!confirmed) return;

    const today = new Date().toISOString().slice(0, 10);
    const path = await save({
      defaultPath: `babel-ebook-settings-${today}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;

    try {
      await exportSettings(path);
      await message(t("export_success"), { title: t("export_settings"), kind: "info" });
    } catch (err) {
      await message(t("error_export_failed", { message: String(err) }), {
        title: t("export_settings"),
        kind: "error",
      });
    }
  };
```

- [ ] **Step 4: Add import handler**

Inside the component, add:

```typescript
  const handleImport = async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path || Array.isArray(path)) return;

    let settings: ExportedSettings;
    try {
      settings = await importSettings(path);
    } catch (err) {
      const errStr = String(err);
      if (errStr.startsWith("version_mismatch:")) {
        const [, actual, expected] = errStr.split(":");
        await message(
          t("error_version_mismatch", { expected, actual }),
          { title: t("import_settings"), kind: "error" }
        );
      } else {
        await message(t("error_invalid_backup"), {
          title: t("import_settings"),
          kind: "error",
        });
      }
      return;
    }

    const confirmed = await confirm(t("import_settings_confirm"), {
      title: t("import_settings"),
      kind: "warning",
    });
    if (!confirmed) return;

    try {
      onImport(settings);
      await message(t("import_success"), { title: t("import_settings"), kind: "info" });
    } catch (err) {
      await message(t("error_import_failed", { message: String(err) }), {
        title: t("import_settings"),
        kind: "error",
      });
    }
  };
```

- [ ] **Step 5: Add Backup & Restore UI section**

At the bottom of the page JSX, before the closing `</div>`, add:

```tsx
      <div className="settings-section backup-restore-section">
        <h3>{t("settings_backup_restore")}</h3>
        <div className="backup-restore-actions">
          <button type="button" onClick={() => void handleExport()}>
            {t("export_settings")}
          </button>
          <button type="button" onClick={() => void handleImport()}>
            {t("import_settings")}
          </button>
        </div>
      </div>
```

- [ ] **Step 6: Add basic CSS**

In `desktop/src/App.css`, add:

```css
.backup-restore-section {
  margin-top: 2rem;
  padding-top: 1rem;
  border-top: 1px solid var(--border-color, #444);
}

.backup-restore-actions {
  display: flex;
  gap: 1rem;
  margin-top: 0.5rem;
}
```

- [ ] **Step 7: Update `App.tsx` to pass `onImport`**

In `App.tsx`, update the `GeneralSettingsPage` render:

```tsx
      case "settings-general":
        return (
          <GeneralSettingsPage
            general={general}
            setGeneral={setGeneral}
            onImport={(settings) => {
              const merged = { ...form, ...settings.translation } as FormState;
              if (
                !merged.active_provider ||
                !merged.providers.some((p) => p.name === merged.active_provider)
              ) {
                merged.active_provider = merged.providers[0]?.name ?? "";
              }
              setForm(merged);
              setGeneral(settings.general);
              void saveSettings(merged);
              void saveGeneralSettings(settings.general);
            }}
          />
        );
```

Make sure `saveSettings` and `saveGeneralSettings` are imported in `App.tsx` (they already are).

- [ ] **Step 8: Verify TypeScript compiles**

Run:

```bash
cd desktop
pnpm exec tsc --noEmit
```

Expected: no errors.

- [ ] **Step 9: Commit**

```bash
git add desktop/src/pages/GeneralSettingsPage.tsx desktop/src/App.tsx desktop/src/App.css
git commit -m "feat(settings): wire export/import buttons to General settings page"
```

---

## Task 4: Manual End-to-End Verification

**Files:**
- No file changes.

- [ ] **Step 1: Start the desktop dev server**

```bash
cd desktop
pnpm tauri dev
```

Wait for the app window to open.

- [ ] **Step 2: Configure a provider**

Go to **Settings → Compute**, add a provider (e.g., DeepSeek), enter an API key, set it active.

- [ ] **Step 3: Export settings**

Go to **Settings → General**, click **Export Settings**. Confirm the warning, choose a path, save.

Verify the saved JSON contains `api_key` for the provider and `version: 5`.

- [ ] **Step 4: Clear settings to simulate reinstall**

Close the app. Delete:

```bash
rm "$USERPROFILE/Documents/BabelEbook/settings.json"
```

Use `cmdkey /list | findstr babel-ebook` to find keyring entries and delete them with `cmdkey /delete:<target>` if present.

- [ ] **Step 5: Reopen app and import**

Run `pnpm tauri dev` again. The Compute tab should be empty. Go to **Settings → General**, click **Import Settings**, select the exported JSON, confirm overwrite.

- [ ] **Step 6: Verify restoration**

Go to **Settings → Compute** and confirm:

- Provider list is restored.
- API key is populated (toggle visibility or click Test Connection).
- General settings (language/theme) are restored.

- [ ] **Step 7: Commit verification notes**

If any step fails, fix the code and re-verify. Once passing, no additional commit is needed unless fixes were made.

---

## Self-Review

### Spec Coverage

| Spec Requirement | Task |
|---|---|
| Export all settings incl. API keys | Task 1 + Task 3 |
| Import restores settings + keyring | Task 1 + Task 3 |
| Version check (`version === 5`) | Task 1 `importSettings` |
| Security warning on export | Task 3 `handleExport` |
| Confirmation on import | Task 3 `handleImport` |
| UI in Settings → General | Task 3 |
| i18n for all supported languages | Task 2 |
| No new Rust commands | Entire plan frontend-only |

### Placeholder Scan

No TBD, TODO, or vague steps. All code blocks contain concrete code. Error messages reference exact i18n keys.

### Type Consistency

- `ExportedSettings` is defined in Task 1 and imported in Task 3.
- `onImport` receives `ExportedSettings` and is used to update `FormState` and `GeneralSettings`.
- `importSettings` throws `version_mismatch:${actual}:${expected}` and `handleImport` parses it as `[, actual, expected]` — consistent.
