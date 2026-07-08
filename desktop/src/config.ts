import { Store } from "@tauri-apps/plugin-store";
import type { FormState } from "./types";

const STORE_NAME = "settings.json";
const SETTINGS_VERSION = 1;

/**
 * Settings that are not part of the translation form and are stored under the
 * `general` key. Keeping them separate makes it easy to load UI state before
 * the first render and avoids mixing translation defaults with interface prefs.
 */
export interface GeneralSettings {
  ui_language: string;
  theme: "light" | "dark";
  follow_system_language: boolean;
}

interface VersionedSettings {
  version: number;
  translation: Partial<FormState>;
  general: GeneralSettings;
}

/**
 * Form fields that should be persisted across sessions.
 *
 * `source`, `output`, and `api_key` are intentionally excluded: paths are
 * per-translation inputs and the API key is handled by the OS keyring.
 */
const TRANSLATION_KEYS: Array<keyof FormState> = [
  "provider",
  "model",
  "base_url",
  "source_lang",
  "target_lang",
  "output_mode",
  "style",
  "exclude_selectors",
  "translate_attributes",
  "preserve_classes",
  "concurrency",
  "max_input_tokens",
  "max_output_tokens",
  "temperature",
  "dry_run",
  "remember_api_key",
  "translate_body",
  "translate_metadata",
  "translate_toc",
  "translate_alt_text",
  "translate_image_captions",
  "translate_tables",
  "translate_footnotes",
  "translate_code",
  "output_font",
  "output_filename_template",
];

const OLD_FLAT_KEYS: Array<keyof FormState> = [
  "provider",
  "model",
  "base_url",
  "source_lang",
  "target_lang",
  "output_mode",
  "style",
  "exclude_selectors",
  "translate_attributes",
  "preserve_classes",
  "concurrency",
  "max_input_tokens",
  "max_output_tokens",
  "temperature",
  "dry_run",
  "remember_api_key",
];

const DEFAULT_GENERAL: GeneralSettings = {
  ui_language: "en",
  theme: "dark",
  follow_system_language: true,
};

async function withStore<T>(fn: (store: Store) => Promise<T>): Promise<T> {
  const store = await Store.load(STORE_NAME);
  try {
    return await fn(store);
  } finally {
    // `Store.load` caches the instance; explicit cleanup is not required.
  }
}

/**
 * Load the translation-related subset of FormState from persistent storage.
 */
export async function loadSettings(): Promise<Partial<FormState>> {
  return withStore(async (store) => {
    const versioned = await store.get<VersionedSettings>("settings");
    if (versioned?.version === SETTINGS_VERSION) {
      return versioned.translation ?? {};
    }

    // Migration from the old flat key-value layout.
    const migrated: Partial<FormState> = {};
    for (const key of OLD_FLAT_KEYS) {
      const value = await store.get<FormState[typeof key]>(key);
      if (value !== null && value !== undefined) {
        (migrated[key] as FormState[typeof key]) = value;
      }
    }

    // Save as versioned settings and remove stale flat keys.
    await saveVersioned(store, migrated, await loadGeneralSettingsRaw(store));
    for (const key of OLD_FLAT_KEYS) {
      await store.delete(key);
    }
    await store.save();

    return migrated;
  });
}

/**
 * Persist the translation-related subset of FormState.
 */
export async function saveSettings(form: FormState): Promise<void> {
  return withStore(async (store) => {
    const translation: Partial<FormState> = {};
    for (const key of TRANSLATION_KEYS) {
      (translation[key] as FormState[typeof key]) = form[key];
    }
    const general = await loadGeneralSettingsRaw(store);
    await saveVersioned(store, translation, general);
    await store.save();
  });
}

/**
 * Load general UI settings from persistent storage.
 */
export async function loadGeneralSettings(): Promise<GeneralSettings> {
  return withStore((store) => loadGeneralSettingsRaw(store));
}

/**
 * Persist general UI settings.
 */
export async function saveGeneralSettings(general: GeneralSettings): Promise<void> {
  return withStore(async (store) => {
    const translation = (await loadSettingsFromStore(store)).translation;
    await saveVersioned(store, translation, general);
    await store.save();
  });
}

async function loadGeneralSettingsRaw(store: Store): Promise<GeneralSettings> {
  const versioned = await store.get<VersionedSettings>("settings");
  if (versioned?.version === SETTINGS_VERSION && versioned.general) {
    return { ...DEFAULT_GENERAL, ...versioned.general };
  }

  // Fall back to localStorage for users upgrading from pre-versioned builds.
  const ui_language = localStorage.getItem("ui_language") ?? DEFAULT_GENERAL.ui_language;
  const theme = (localStorage.getItem("ui_theme") as "light" | "dark" | null) ?? DEFAULT_GENERAL.theme;
  const follow_system_language =
    localStorage.getItem("follow_system_language") === "true" || DEFAULT_GENERAL.follow_system_language;

  return {
    ui_language,
    theme,
    follow_system_language,
  };
}

async function loadSettingsFromStore(store: Store): Promise<VersionedSettings> {
  const versioned = await store.get<VersionedSettings>("settings");
  if (versioned?.version === SETTINGS_VERSION) {
    return versioned;
  }
  return {
    version: SETTINGS_VERSION,
    translation: {},
    general: DEFAULT_GENERAL,
  };
}

async function saveVersioned(
  store: Store,
  translation: Partial<FormState>,
  general: GeneralSettings
): Promise<void> {
  const payload: VersionedSettings = {
    version: SETTINGS_VERSION,
    translation,
    general,
  };
  await store.set("settings", payload);
}
