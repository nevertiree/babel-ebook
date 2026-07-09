import { invoke } from "@tauri-apps/api/core";
import { documentDir, join } from "@tauri-apps/api/path";
import { readTextFile, writeTextFile, mkdir, exists } from "@tauri-apps/plugin-fs";
import type { FormState, ProviderConfig, ThemeId } from "./types";
import { themes } from "./types";

const SETTINGS_DIR = "BabelEbook";
const SETTINGS_FILE = "settings.json";
const SETTINGS_VERSION = 5;
const DEFAULT_CHECKPOINT_DIR = "BabelEbook/checkpoints";

/**
 * Settings that are not part of the translation form and are stored under the
 * `general` key. Keeping them separate makes it easy to load UI state before
 * the first render and avoids mixing translation defaults with interface prefs.
 */
export interface GeneralSettings {
  ui_language: string;
  theme: ThemeId;
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
  "system_prompt",
  "prompts",
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
  "checkpoint_dir",
  "resume",
  "refine",
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
  theme: "dark" as ThemeId,
  follow_system_language: true,
};

function normalizeTheme(value: unknown): ThemeId {
  return themes.includes(value as ThemeId) ? (value as ThemeId) : DEFAULT_GENERAL.theme;
}

async function settingsPath(): Promise<string> {
  const docs = await documentDir();
  return await join(docs, SETTINGS_DIR, SETTINGS_FILE);
}

async function ensureSettingsDir(): Promise<void> {
  const docs = await documentDir();
  const dirPath = await join(docs, SETTINGS_DIR);
  const dirExists = await exists(dirPath);
  if (!dirExists) {
    await mkdir(dirPath, { recursive: true });
  }
}

async function readSettingsFile(): Promise<VersionedSettings | null> {
  const path = await settingsPath();
  const fileExists = await exists(path);
  if (!fileExists) {
    return null;
  }
  try {
    const text = await readTextFile(path);
    return JSON.parse(text) as VersionedSettings;
  } catch {
    return null;
  }
}

async function writeSettingsFile(payload: VersionedSettings): Promise<void> {
  await ensureSettingsDir();
  const path = await settingsPath();
  await writeTextFile(path, JSON.stringify(payload, null, 2));
}

/**
 * Strip secrets from provider configs before writing to disk.
 */
function providersForStorage(providers: ProviderConfig[]): Omit<ProviderConfig, "api_key">[] {
  return providers.map((p) => ({
    name: p.name,
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
async function migrateFromFlatSettings(versioned: VersionedSettings): Promise<Partial<FormState>> {
  const raw: Record<string, unknown> = {};
  // The old flat layout stored keys directly under the root of the JSON object.
  // If we encounter such a layout, the versioned.translation object will contain
  // those flat keys (with the exception of provider/base_url handled below).
  for (const key of OLD_FLAT_KEYS) {
    const value = (versioned.translation as Record<string, unknown>)[key];
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

  const used = new Set<string>();
  const makeUniqueName = (p: ProviderConfig) => {
    const base = p.provider;
    let candidate = base;
    let index = 1;
    while (used.has(candidate)) {
      index += 1;
      candidate = `${base} ${index}`;
    }
    used.add(candidate);
    return candidate;
  };

  migrated.providers = [
    {
      name: makeUniqueName({ provider: oldProvider } as ProviderConfig),
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
  const versioned = await readSettingsFile();

  if (versioned?.version === SETTINGS_VERSION) {
    const translation = versioned.translation ?? {};
    if (Array.isArray(translation.providers)) {
      translation.providers = await hydrateApiKeys(translation.providers);
    }
    const docs = await documentDir();
    if (!translation.checkpoint_dir || typeof translation.checkpoint_dir !== "string" || translation.checkpoint_dir.trim().length === 0) {
      translation.checkpoint_dir = await join(docs, DEFAULT_CHECKPOINT_DIR);
    }
    if (typeof translation.refine !== "boolean") {
      translation.refine = false;
    }
    if (typeof translation.resume !== "string") {
      translation.resume = "";
    }
    return translation;
  }

  // Migration from the old flat key-value layout.
  const migrated = await migrateFromFlatSettings(versioned ?? { version: 1, translation: {}, general: DEFAULT_GENERAL });
  const docs = await documentDir();
  if (!migrated.checkpoint_dir || migrated.checkpoint_dir.trim().length === 0) {
    migrated.checkpoint_dir = await join(docs, DEFAULT_CHECKPOINT_DIR);
  }
  if (migrated.refine === undefined) {
    migrated.refine = false;
  }
  if (migrated.resume === undefined) {
    migrated.resume = "";
  }
  await writeSettingsFile({
    version: SETTINGS_VERSION,
    translation: migrated,
    general: {
      ...(versioned?.general ?? DEFAULT_GENERAL),
      theme: normalizeTheme(versioned?.general?.theme),
    },
  });

  return migrated;
}

/**
 * Persist the translation-related subset of FormState.
 */
export async function saveSettings(form: FormState): Promise<void> {
  const versioned = await readSettingsFile();

  const translation: Partial<FormState> = {};
  for (const key of TRANSLATION_KEYS) {
    if (key === "providers") {
      translation.providers = providersForStorage(form.providers) as ProviderConfig[];
    } else {
      (translation[key] as FormState[typeof key]) = form[key];
    }
  }

  await writeSettingsFile({
    version: SETTINGS_VERSION,
    translation,
    general: versioned?.general ?? DEFAULT_GENERAL,
  });
  await saveApiKeys(form.providers);
}

/**
 * Load general UI settings from persistent storage.
 */
export async function loadGeneralSettings(): Promise<GeneralSettings> {
  const versioned = await readSettingsFile();
  if (versioned?.general) {
    return {
      ...DEFAULT_GENERAL,
      ...versioned.general,
      theme: normalizeTheme(versioned.general.theme),
    };
  }

  // Fall back to localStorage for users upgrading from pre-versioned builds.
  const ui_language = localStorage.getItem("ui_language") ?? DEFAULT_GENERAL.ui_language;
  const theme = normalizeTheme(localStorage.getItem("ui_theme"));
  const follow_system_language = localStorage.getItem("follow_system_language") !== "false";

  return {
    ui_language,
    theme,
    follow_system_language,
  };
}

/**
 * Persist general UI settings.
 */
export async function saveGeneralSettings(general: GeneralSettings): Promise<void> {
  const versioned = (await readSettingsFile()) ?? {
    version: SETTINGS_VERSION,
    translation: {},
    general: DEFAULT_GENERAL,
  };
  await writeSettingsFile({ ...versioned, general });
}
