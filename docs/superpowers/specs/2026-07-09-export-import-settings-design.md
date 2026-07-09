# Export/Import Settings Design

## Problem

After reinstalling BabelEbook, users lose their provider configurations and API keys.
The app stores general settings in `Documents/BabelEbook/settings.json` but persists
API keys in the OS keyring (Windows Credential Manager on Windows). Reinstalling or
uninstalling can leave the keyring empty or disconnected from the saved provider list,
so the "Compute" settings tab appears blank after reinstall.

## Goal

Allow users to back up and restore their complete BabelEbook configuration to a
single file, so a reinstall can be recovered with one import action.

## Scope

- Export all settings: provider configs (with API keys), active provider, model params,
  translation options, prompts, output options, and general UI settings.
- Import the same file and restore both `settings.json` and the OS keyring entries.
- No encryption for the first iteration; rely on a clear security warning and user-chosen storage location.

## Out of Scope

- Encrypted backups.
- Partial imports (e.g., only providers).
- Cloud or automatic backups.

## Design

### Export File Format

A JSON file with this structure:

```json
{
  "version": 5,
  "exported_at": "2026-07-09T09:45:54Z",
  "app_version": "0.2.0",
  "translation": {
    "providers": [
      {
        "name": "deepseek",
        "provider": "deepseek",
        "api_key": "sk-...",
        "base_url": "",
        "use_custom_base_url": false
      }
    ],
    "active_provider": "deepseek",
    "model": "deepseek-chat",
    "source_lang": "en",
    "target_lang": "zh-CN",
    "output_mode": "bilingual",
    "style": "default",
    "system_prompt": "",
    "prompts": { "default": "", "literary": "", "technical": "", "academic": "", "refine": "" },
    "exclude_selectors": "",
    "translate_attributes": "",
    "preserve_classes": false,
    "concurrency": 3,
    "max_input_tokens": 4000,
    "max_output_tokens": 2000,
    "temperature": 0.3,
    "dry_run": null,
    "remember_api_key": true,
    "translate_body": true,
    "translate_metadata": true,
    "translate_toc": true,
    "translate_alt_text": true,
    "translate_image_captions": true,
    "translate_tables": true,
    "translate_footnotes": true,
    "translate_code": false,
    "output_font": "\"Noto Serif CJK SC\", \"Source Han Serif SC\", \"SimSun\", serif",
    "output_filename_template": "{stem}_{target_lang}",
    "checkpoint_dir": "C:\\Users\\...\\Documents\\BabelEbook\\checkpoints",
    "resume": "",
    "refine": false
  },
  "general": {
    "ui_language": "zh-CN",
    "theme": "dark",
    "follow_system_language": true
  }
}
```

- `version` matches the internal `SETTINGS_VERSION` constant (currently `5`). Imports must
  reject files with a different version.
- `exported_at` and `app_version` are metadata for the user.
- `api_key` is included in plaintext because the file itself is the backup medium.

### UI Placement

Add a "Backup & Restore" section at the bottom of **Settings → General**:

- **Export Settings** button — opens a save dialog.
- **Import Settings** button — opens an open dialog filtered to `.json`.

### Export Flow

1. Call `loadSettings()` and `loadGeneralSettings()`.
2. Open a save dialog with default filename `babel-ebook-settings-YYYY-MM-DD.json`.
3. If the user confirms, write the JSON payload to the selected path.
4. Show a success or failure message.
5. Show a security warning before writing: "This file contains your API keys. Store it securely."

### Import Flow

1. Open an open dialog filtered to `.json`.
2. Read and parse the selected file.
3. Validate that `version === 5` and that `translation.providers` is an array.
4. Show a confirmation dialog: "Importing will overwrite your current settings. Continue?"
5. If confirmed, update the in-memory form and general state with the imported values.
6. Trigger `saveSettings(form)` and `saveGeneralSettings(general)`, which will:
   - Write non-secret fields to `Documents/BabelEbook/settings.json`.
   - Persist each provider's `api_key` to the OS keyring.
7. Apply UI language/theme immediately.

### Error Handling

| Error | Message |
|---|---|
| User cancels dialog | Silent return |
| File read or parse fails | "Unable to read backup file" |
| `version` mismatch | "Backup file version is incompatible (expected 5, got X)" |
| Missing required fields | "Backup file is corrupted" |
| Save after import fails | Show native error; user can retry |

### Implementation Approach

Implement entirely in the frontend because the existing persistence layer already bridges the keyring:

- `loadSettings()` hydrates provider configs with API keys from the keyring.
- `saveSettings()` strips secrets from the JSON on disk and writes each API key to the keyring.

Files to change:

- `desktop/src/pages/GeneralSettingsPage.tsx` — add UI and handlers.
- `desktop/src/config.ts` — add `exportSettings(path)` and `importSettings(path)` helpers.
- `desktop/src/locales/*.json` — add translation keys for buttons, warnings, and errors.

No new Rust commands are required. Existing `fs` and `dialog` plugin permissions are sufficient.

## Security Considerations

- The exported file contains plaintext API keys. A clear warning must be shown before
  export.
- The user chooses where to save the file; the app should not default to a shared or synced location.
- Do not commit or upload the backup file. Consider adding `.babel-ebook-settings*.json`
  to `.gitignore` if users commonly keep backups near the repo.

## Testing Notes

- Export a config, uninstall the app, reinstall, import the file, and verify that:
  - Provider list appears in the Compute tab.
  - API keys are populated (test connection works).
  - Model, translation, prompt, and general settings are restored.
- Verify version mismatch is rejected gracefully.
- Verify malformed JSON shows an error.
