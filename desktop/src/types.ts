/**
 * Shared form state used across the desktop application.
 */
export interface FormState {
  source: string;
  output: string;
  provider: string;
  api_key: string;
  model: string;
  concurrency: number;
  max_input_tokens: number;
  max_output_tokens: number;
  temperature: number;
  source_lang: string;
  target_lang: string;
  dry_run: boolean;
  base_url: string;
  output_mode: string;
  style: string;
  preserve_classes: boolean;
  exclude_selectors: string;
  translate_attributes: string;
  remember_api_key: boolean;
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
}

/**
 * Available application pages.
 */
export type Page =
  | "translate"
  | "logs"
  | "settings-compute"
  | "settings-translation"
  | "settings-output"
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
  provider: "deepseek",
  api_key: "",
  model: "deepseek-chat",
  concurrency: 3,
  max_input_tokens: 4000,
  max_output_tokens: 2000,
  temperature: 0.3,
  source_lang: "en",
  target_lang: "zh-CN",
  dry_run: false,
  base_url: "",
  output_mode: "bilingual",
  style: "default",
  preserve_classes: false,
  exclude_selectors: "",
  translate_attributes: "",
  remember_api_key: true,
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
};

export const providers = ["deepseek", "openai", "anthropic", "ollama"] as const;
export const outputModes = ["bilingual", "translation_only", "interleaved"] as const;
export const styles = ["default", "literary", "technical", "academic"] as const;

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
