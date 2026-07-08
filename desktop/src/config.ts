import { Store } from "@tauri-apps/plugin-store";
import { invoke } from "@tauri-apps/api/core";
import type { FormState, ProviderConfig } from "./types";

const STORE_NAME = "settings.json";
const SETTINGS_VERSION = 2;

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
 * `source`, `output`, and per-provider `api_key` are intentionally excluded:
 * paths are per-translation inputs and API keys are handled by the OS keyring.
 * `provider`/`base_url` are now stored inside the `providers` array.
 */
const TRANSLATION_KEYS: Array<keyof FormState> = [
  "providers",
  "active_provider",
  "model",
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

/**
 * Keys from the original flat settings layout. Used for one-time migration to
 * the versioned provider-array format.
 */
const OLD_FLAT_KEYS: string[] = [
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
 * Strip secrets from provider configs before writing to disk.
 */
function providersForStorage(providers: ProviderConfig[]): Omit<ProviderConfig, "api_key">[] {
  return providers.map((p) => ({
    provider: p.provider,
    base_url: p.base_url,
    use_custom_base_url: p.use_custom_base_url,
  }));
}

/**
 * Persist each provider's API key to the OS keyring.
 */
async function saveApiKeys(providers: ProviderConfig[]): Promise<void> {
  for (const p of providers) {
    if (p.api_key) {
      await invoke("store_api_key", { provider: p.provider, apiKey: p.api_key });
    } else {
      await invoke("delete_api_key", { provider: p.provider }).catch(() => undefined);
    }
  }
}

/**
 * Load API keys from the OS keyring and inject them into provider configs.
 */
async function hydrateApiKeys(providers: ProviderConfig[]): Promise<ProviderConfig[]> {
  const hydrated: ProviderConfig[] = [];
  for (const p of providers) {
    const apiKey = await invoke<string | null>("load_api_key", { provider: p.provider }).catch(() => null);
    hydrated.push({ ...p, api_key: apiKey ?? "" });
  }
  return hydrated;
}

/**
 * Migrate legacy flat settings to the provider-array layout.
 */
async function migrateFromFlatSettings(store: Store): Promise<Partial<FormState>> {
  const raw: Record<string, unknown> = {};
  for (const key of OLD_FLAT_KEYS) {
    const value = await store.get<unknown>(key);
    if (value !== null && value !== undefined) {
      raw[key] = value;
    }
  }

  const migrated: Partial<FormState> = {};
  for (const key of TRANSLATION_KEYS) {
    if (key !== "providers" && key !== "active_provider" && raw[key] !== undefined) {
      (migrated[key] as FormState[typeof key]) = raw[key] as FormState[typeof key];
    }
  }

  const oldProvider = typeof raw.provider === "string" ? raw.provider : "deepseek";
  const oldBaseUrl = typeof raw.base_url === "string" ? raw.base_url : "";
  const oldApiKey = await invoke<string | null>("load_api_key", { provider: oldProvider }).catch(() => null);

  migrated.providers = [
    {
      provider: oldProvider,
      api_key: oldApiKey ?? "",
      base_url: oldBaseUrl,
      use_custom_base_url: oldBaseUrl.trim().length > 0,
    },
  ];
  migrated.active_provider = oldProvider;

  return migrated;
}

/**
 * Load the translation-related subset of FormState from persistent storage.
 */
export async function loadSettings(): Promise<Partial<FormState>> {
  return withStore(async (store) => {
    const versioned = await store.get<VersionedSettings>("settings");

    if (versioned?.version === SETTINGS_VERSION) {
      const translation = versioned.translation ?? {};
      if (Array.isArray(translation.providers)) {
        translation.providers = await hydrateApiKeys(translation.providers);
      }
      return translation;
    }

    // Migration from the old flat key-value layout.
    const migrated = await migrateFromFlatSettings(store);
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
      if (key === "providers") {
        translation.providers = providersForStorage(form.providers) as ProviderConfig[];
      } else {
        (translation[key] as FormState[typeof key]) = form[key];
      }
    }

    const general = await loadGeneralSettingsRaw(store);
    await saveVersioned(store, translation, general);
    await store.save();
    await saveApiKeys(form.providers);
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
    localStorage.getItem("follow_system_language") !== "false";

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
