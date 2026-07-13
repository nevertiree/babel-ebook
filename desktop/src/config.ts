import { invoke } from "@tauri-apps/api/core";
import { documentDir, join } from "@tauri-apps/api/path";
import { readTextFile, writeTextFile, mkdir, exists } from "@tauri-apps/plugin-fs";
import type { FormState, ProviderConfig, ThemeId } from "./types";
import { themes } from "./types";

declare const __APP_VERSION__: string;

const SETTINGS_DIR = "BabelEbook";
const SETTINGS_FILE = "settings.json";
const SETTINGS_VERSION = 5;
const DEFAULT_CHECKPOINT_DIR = "BabelEbook/checkpoints";

export interface ExportedSettings {
  version: number;
  exported_at: string;
  app_version: string;
  translation: Partial<FormState>;
  general: GeneralSettings;
  ocr: OcrSettings;
}

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

/**
 * OCR engine defaults stored under the `ocr` key. These are "set once" tuning
 * knobs (concurrency, DPI, verify/refine defaults); per-job choices (source
 * PDF, output path, OCR provider+model, verify/refine on/off) stay on the OCR
 * page, mirroring how the translate page keeps provider/model on the page and
 * pushes detailed config to settings tabs.
 */
export interface OcrSettings {
  concurrency: number;
  dpi: number;
  verify: { threshold: number; maxAttempts: number };
  refine: { rounds: number; withImage: boolean };
}

export const DEFAULT_OCR_SETTINGS: OcrSettings = {
  concurrency: 3,
  dpi: 200,
  verify: { threshold: 0.7, maxAttempts: 3 },
  refine: { rounds: 1, withImage: false },
};

interface VersionedSettings {
  version: number;
  translation: Partial<FormState>;
  general: GeneralSettings;
  ocr?: OcrSettings;
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
];

const DEFAULT_GENERAL: GeneralSettings = {
  ui_language: "en",
  theme: "dark" as ThemeId,
  follow_system_language: true,
};

async function loadProviderApiKey(name: string): Promise<string> {
  return (await invoke<string | null>("load_api_key", { name }).catch(() => null)) ?? "";
}

async function storeProviderApiKey(name: string, apiKey: string): Promise<boolean> {
  if (apiKey.trim().length === 0) {
    await invoke("delete_api_key", { name }).catch(() => null);
    return true;
  }
  try {
    await invoke("store_api_key", { name, apiKey });
    return true;
  } catch (err) {
    console.error(`[keyring] failed to store API key for ${name}:`, err);
    return false;
  }
}

export function normalizeTheme(value: unknown): ThemeId {
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
  // Legacy API keys remain in the OS keyring; the normal load path will populate
  // the in-memory api_key from the keyring using the provider config name.

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
      api_key: "",
      base_url: oldBaseUrl,
      use_custom_base_url: oldBaseUrl.trim().length > 0,
    },
  ];
  migrated.active_provider = oldProvider;

  return migrated;
}

function normalizeProvider(p: unknown): ProviderConfig {
  const raw = p as Record<string, unknown>;
  return {
    name: typeof raw.name === "string" && raw.name.trim().length > 0 ? raw.name : "Provider",
    provider: typeof raw.provider === "string" && raw.provider.trim().length > 0 ? raw.provider : "deepseek",
    api_key: typeof raw.api_key === "string" ? raw.api_key : "",
    base_url: typeof raw.base_url === "string" ? raw.base_url : "",
    use_custom_base_url: typeof raw.use_custom_base_url === "boolean" ? raw.use_custom_base_url : false,
  };
}

function normalizeProviders(providers: unknown): ProviderConfig[] {
  if (!Array.isArray(providers)) return [];
  return providers.map(normalizeProvider);
}

/**
 * Load the translation-related subset of FormState from persistent storage.
 */
export async function loadSettings(): Promise<Partial<FormState>> {
  const versioned = await readSettingsFile();

  if (versioned?.version === SETTINGS_VERSION) {
    const translation = versioned.translation ?? {};
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
    translation.providers = normalizeProviders(translation.providers);

    // One-time migration: any plaintext API keys left in settings.json are moved
    // to the OS keyring and then cleared from the persisted file. If the keyring
    // is unreachable, leave the plaintext key in place so the next load can retry.
    let needsRewrite = false;
    translation.providers = await Promise.all(
      translation.providers.map(async (p) => {
        const rawKey = p.api_key;
        if (rawKey.trim().length > 0) {
          const stored = await storeProviderApiKey(p.name, rawKey);
          if (stored) {
            needsRewrite = true;
          }
          return { ...p, api_key: rawKey };
        }
        const keyringKey = await loadProviderApiKey(p.name);
        return { ...p, api_key: keyringKey };
      })
    );

    if (needsRewrite) {
      await writeSettingsFile({
        version: SETTINGS_VERSION,
        translation,
        general: versioned.general,
      });
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
  migrated.providers = normalizeProviders(migrated.providers);
  migrated.providers = await Promise.all(
    migrated.providers.map(async (p) => ({
      ...p,
      api_key: await loadProviderApiKey(p.name),
    }))
  );
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
    (translation[key] as FormState[typeof key]) = form[key];
  }

  // Persist API keys in the OS keyring, never in settings.json.
  if (translation.providers) {
    translation.providers = (translation.providers as ProviderConfig[]).map((p) => ({
      ...p,
      api_key: "",
    }));

    for (const p of form.providers) {
      await storeProviderApiKey(p.name, p.api_key);
    }
  }

  await writeSettingsFile({
    version: SETTINGS_VERSION,
    translation,
    general: versioned?.general ?? DEFAULT_GENERAL,
  });
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

/** Load OCR engine defaults, falling back to sensible defaults. */
export async function loadOcrSettings(): Promise<OcrSettings> {
  const versioned = await readSettingsFile();
  if (versioned?.ocr) {
    return { ...DEFAULT_OCR_SETTINGS, ...versioned.ocr };
  }
  return DEFAULT_OCR_SETTINGS;
}

/** Persist OCR engine defaults. */
export async function saveOcrSettings(ocr: OcrSettings): Promise<void> {
  const versioned = (await readSettingsFile()) ?? {
    version: SETTINGS_VERSION,
    translation: {},
    general: DEFAULT_GENERAL,
  };
  await writeSettingsFile({ ...versioned, ocr });
}

export async function exportSettings(path: string): Promise<void> {
  const translation = await loadSettings();
  const general = await loadGeneralSettings();
  const ocr = await loadOcrSettings();

  // TODO: wire to build version
  const appVersion =
    typeof __APP_VERSION__ !== "undefined" ? __APP_VERSION__ : "0.2.0";

  const payload: ExportedSettings = {
    version: SETTINGS_VERSION,
    exported_at: new Date().toISOString(),
    app_version: appVersion,
    translation,
    general,
    ocr,
  };

  await writeTextFile(path, JSON.stringify(payload, null, 2));
}

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
  if (!p) return false;

  const translation = p.translation as Record<string, unknown> | undefined;
  const general = p.general as Record<string, unknown> | undefined;
  const providers = translation?.providers as unknown[] | undefined;

  return (
    typeof p.version === "number" &&
    typeof p.exported_at === "string" &&
    typeof p.app_version === "string" &&
    typeof translation === "object" &&
    translation !== null &&
    Array.isArray(providers) &&
    providers.every((item) => typeof item === "object" && item !== null) &&
    typeof general === "object" &&
    general !== null &&
    typeof general.ui_language === "string" &&
    themes.includes(general.theme as ThemeId) &&
    typeof general.follow_system_language === "boolean"
  );
}
