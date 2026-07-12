export type TaskStatus =
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "paused";

export interface Task {
  id: string;
  source_path: string;
  output_path: string;
  status: TaskStatus;
  progress_percent: number;
  message: string;
  error?: string;
  created_at: number;
  started_at?: number;
  completed_at?: number;
  chapter_total?: number;
  chapter_progress?: Record<number, { chunk_total: number; chunks_done: number }>;
  chapters_completed?: number;
}

export interface QueueState {
  tasks: Task[];
  running: boolean;
  current_task_id?: string;
}

/**
 * A single provider/API configuration.
 *
 * API keys are stored in the OS credential store (keyring / Credential Manager)
 * and are never written to `settings.json`. The `api_key` field here lives only
 * in memory while the app is running.
 */
export interface ProviderConfig {
  name: string;
  provider: string;
  api_key: string;
  base_url: string;
  use_custom_base_url: boolean;
}

/**
 * Custom prompt templates for each translation style.
 */
export interface PromptTemplates {
  default: string;
  literary: string;
  technical: string;
  academic: string;
  refine: string;
}

/**
 * Shared form state used across the desktop application.
 *
 * This is the canonical serializable shape persisted to disk. To reduce
 * coupling and re-renders, individual settings pages receive focused slices
 * (see below) instead of the full object.
 */
export interface FormState {
  source: string;
  output: string;
  system_prompt: string;
  prompts: PromptTemplates;
  source_lang: string;
  target_lang: string;
  dry_run: boolean;
  output_mode: string;
  style: string;
  preserve_classes: boolean;
  exclude_selectors: string;
  translate_attributes: string;
  translate_body: boolean;
  translate_metadata: boolean;
  translate_toc: boolean;
  translate_alt_text: boolean;
  translate_image_captions: boolean;
  translate_tables: boolean;
  translate_footnotes: boolean;
  translate_code: boolean;
  output_font: string;
  output_filename_template: string;

  // Checkpoint / resume
  checkpoint_dir: string;
  resume: string;
  refine: boolean;

  // Provider management
  providers: ProviderConfig[];
  active_provider: string;

  // Model parameters (moved out of the provider tab)
  model: string;
  concurrency: number;
  max_input_tokens: number;
  max_output_tokens: number;
  temperature: number;
}

/** Model / inference parameters shown on the Model settings page. */
export type ModelParams = Pick<
  FormState,
  "model" | "max_input_tokens" | "max_output_tokens" | "temperature"
>;

/** Translation language, mode, style and element scope settings. */
export type TranslationSettingsState = Pick<
  FormState,
  | "source_lang"
  | "target_lang"
  | "output_mode"
  | "style"
  | "preserve_classes"
  | "exclude_selectors"
  | "translate_attributes"
  | "translate_body"
  | "translate_metadata"
  | "translate_toc"
  | "translate_alt_text"
  | "translate_image_captions"
  | "translate_tables"
  | "translate_footnotes"
  | "translate_code"
>;

/** System prompt and per-style prompt templates. */
export type PromptSettingsState = Pick<FormState, "system_prompt" | "prompts">;

/** Output formatting and checkpoint directory settings. */
export type OutputSettingsState = Pick<
  FormState,
  "output_font" | "output_filename_template" | "checkpoint_dir"
>;

/** Queue / concurrency settings. */
export type QueueSettingsState = Pick<FormState, "concurrency">;

/** Main translation inputs shown on the translate page. */
export type TranslateInputs = Pick<
  FormState,
  | "source"
  | "output"
  | "source_lang"
  | "target_lang"
  | "output_mode"
  | "providers"
  | "active_provider"
  | "model"
  | "checkpoint_dir"
  | "resume"
  | "refine"
>;

/**
 * Available application pages.
 */
export type Page =
  | "translate"
  | "tasks"
  | "logs"
  | "settings-compute"
  | "settings-model"
  | "settings-translation"
  | "settings-output"
  | "settings-prompts"
  | "settings-queue"
  | "settings-general"
  | "about"
  | "legal";

/**
 * Translation progress information shown to the user.
 */
export interface ProgressState {
  percent: number;
  message: string;
}

/**
 * Summary of a translation checkpoint exposed to the frontend.
 */
export interface CheckpointInfo {
  job_id: string;
  source_hash: string;
  source_path: string;
  matches_current_source: boolean;
  completed: number;
  total: number;
  failed: number;
  pending: number;
}

/**
 * A single friendly log entry.
 */
export interface LogEntry {
  id: string;
  kind: "info" | "chapter" | "success" | "error";
  message: string;
  details?: string;
  timestamp: number;
}

/**
 * Field-level validation errors.
 */
export interface ValidationErrors {
  source?: string;
  output?: string;
  api_key?: string;
}

/**
 * Result of validating the current form.
 */
export interface ValidationResult {
  valid: boolean;
  errors: ValidationErrors;
  reason?: string;
}

/**
 * Default form values.
 */
export const defaults: FormState = {
  source: "",
  output: "",
  system_prompt: "",
  prompts: {
    default: "",
    literary: "",
    technical: "",
    academic: "",
    refine: "",
  },
  source_lang: "en",
  target_lang: "zh-CN",
  dry_run: false,
  output_mode: "bilingual",
  style: "default",
  preserve_classes: false,
  exclude_selectors: "",
  translate_attributes: "",
  translate_body: true,
  translate_metadata: true,
  translate_toc: true,
  translate_alt_text: true,
  translate_image_captions: true,
  translate_tables: true,
  translate_footnotes: true,
  translate_code: false,
  output_font: '"Noto Serif CJK SC", "Source Han Serif SC", "SimSun", serif',
  output_filename_template: "{stem}_{target_lang}",

  checkpoint_dir: "",
  resume: "",
  refine: false,

  providers: [],
  active_provider: "",

  model: "deepseek-chat",
  concurrency: 3,
  max_input_tokens: 4000,
  max_output_tokens: 2000,
  temperature: 0.3,
};

export const providers = ["deepseek", "openai", "anthropic", "ollama"] as const;
export const outputModes = ["bilingual", "translation_only", "interleaved"] as const;
export const styles = ["default", "literary", "technical", "academic"] as const;
export const themes = ["dark", "light", "midnight", "solarized", "high-contrast"] as const;
export type ThemeId = (typeof themes)[number];

export const languages = [
  { code: "en", label: "English" },
  { code: "es", label: "Español" },
  { code: "ja", label: "日本語" },
  { code: "ko", label: "한국어" },
  { code: "ru", label: "Русский" },
  { code: "zh-CN", label: "简体中文" },
] as const;

export interface TargetLanguage {
  code: string;
  label: string;
}

export const sourceLanguages: TargetLanguage[] = [
  { code: "auto", label: "Auto detect" },
  { code: "en", label: "English" },
  { code: "es", label: "Español" },
  { code: "ja", label: "日本語" },
  { code: "ko", label: "한국어" },
  { code: "ru", label: "Русский" },
  { code: "zh-CN", label: "简体中文" },
  { code: "zh-TW", label: "繁體中文" },
  { code: "de", label: "Deutsch" },
  { code: "fr", label: "Français" },
] as const;

export const targetLanguages: TargetLanguage[] = [
  { code: "zh-CN", label: "简体中文" },
  { code: "zh-TW", label: "繁體中文" },
  { code: "en", label: "English" },
  { code: "es", label: "Español" },
  { code: "ja", label: "日本語" },
  { code: "ko", label: "한국어" },
  { code: "ru", label: "Русский" },
  { code: "de", label: "Deutsch" },
  { code: "fr", label: "Français" },
] as const;

export const defaultFonts: Record<string, string> = {
  "zh-CN": '"Noto Serif CJK SC", "Source Han Serif SC", "SimSun", serif',
  "zh-TW": '"Noto Serif CJK TC", "Source Han Serif TC", "PMingLiU", serif',
  ja: '"Noto Serif CJK JP", "Yu Mincho", "Hiragino Mincho ProN", serif',
  ko: '"Noto Serif CJK KR", "Batang", "Gungsuh", serif',
  ru: '"Georgia", "Times New Roman", "Literaturnaya", serif',
  en: '"Georgia", "Times New Roman", serif',
  es: '"Georgia", "Times New Roman", serif',
  de: '"Georgia", "Times New Roman", serif',
  fr: '"Georgia", "Times New Roman", serif',
};

export interface ModelOption {
  value: string;
  label: string;
}

export const recommendedModels: Record<string, ModelOption[]> = {
  deepseek: [
    { value: "deepseek-chat", label: "DeepSeek Chat" },
    { value: "deepseek-coder", label: "DeepSeek Coder" },
  ],
  openai: [
    { value: "gpt-4o", label: "GPT-4o" },
    { value: "gpt-4o-mini", label: "GPT-4o Mini" },
    { value: "gpt-4-turbo", label: "GPT-4 Turbo" },
  ],
  anthropic: [
    { value: "claude-3-5-sonnet-20241022", label: "Claude 3.5 Sonnet" },
    { value: "claude-3-opus-20240229", label: "Claude 3 Opus" },
  ],
  ollama: [
    { value: "llama3", label: "Llama 3" },
    { value: "qwen2", label: "Qwen 2" },
    { value: "mistral", label: "Mistral" },
  ],
};

export const providerApiKeyHints: Record<string, string> = {
  deepseek: "sk-",
  openai: "sk-",
  anthropic: "sk-ant",
  ollama: "",
};

export function providerDefaultBaseUrl(provider: string): string {
  switch (provider) {
    case "deepseek":
      return "https://api.deepseek.com";
    case "openai":
      return "https://api.openai.com/v1";
    case "anthropic":
      return "https://api.anthropic.com";
    default:
      return "";
  }
}

export const excludeSelectorExamples = [".no-translate", ".code", "pre", "code"];
export const translateAttributeExamples = ["alt", "title"];
