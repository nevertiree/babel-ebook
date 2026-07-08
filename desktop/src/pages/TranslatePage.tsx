import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import type { FormState, Page, ProgressState, ValidationResult } from "../types";
import {
  outputModes,
  sourceLanguages,
  targetLanguages,
} from "../types";

interface TranslatePageProps {
  form: FormState;
  setForm: <K extends keyof FormState>(key: K, value: FormState[K]) => void;
  onStart: () => void;
  loading: boolean;
  progress: ProgressState;
  validation: ValidationResult;
  onPageChange: (page: Page) => void;
}

export default function TranslatePage({
  form,
  setForm,
  onStart,
  loading,
  progress,
  validation,
  onPageChange,
}: TranslatePageProps) {
  const { t } = useTranslation();

  const hasProviders = form.providers.length > 0;
  const activeProvider = form.providers.find((p) => p.provider === form.active_provider);

  const selectSource = async () => {
    const path = await open({
      filters: [
        {
          name: t("ebook_files"),
          extensions: ["epub", "mobi", "azw3", "txt"],
        },
      ],
    });
    if (path) {
      setForm("source", path);
    }
  };

  const selectOutput = async () => {
    const path = await save({
      filters: [{ name: "EPUB", extensions: ["epub"] }],
      defaultPath: form.output || "output.epub",
    });
    if (path) {
      setForm("output", path);
    }
  };

  return (
    <div className="page translate-page">
      <h2>{t("nav_translate")}</h2>

      <section className="quick-settings">
        <div className="row">
          {hasProviders && activeProvider && (
            <label>
              {t("provider")}
              <select
                value={form.active_provider}
                onChange={(e) => setForm("active_provider", e.target.value)}
              >
                {form.providers.map((p) => (
                  <option key={p.provider} value={p.provider}>
                    {t(`provider_${p.provider}`)}
                  </option>
                ))}
              </select>
            </label>
          )}

          <label>
            {t("source_lang")}
            <select
              value={form.source_lang}
              onChange={(e) => setForm("source_lang", e.target.value)}
            >
              {sourceLanguages.map((lang) => (
                <option key={lang.code} value={lang.code}>
                  {t(`target_lang_${lang.code}`)}
                </option>
              ))}
            </select>
          </label>

          <label>
            {t("target_lang")}
            <select
              value={form.target_lang}
              onChange={(e) => setForm("target_lang", e.target.value)}
            >
              {targetLanguages.map((lang) => (
                <option key={lang.code} value={lang.code}>
                  {t(`target_lang_${lang.code}`)}
                </option>
              ))}
            </select>
          </label>

          <label>
            {t("output_mode")}
            <select
              value={form.output_mode}
              onChange={(e) => setForm("output_mode", e.target.value)}
            >
              {outputModes.map((mode) => (
                <option key={mode} value={mode}>
                  {t(`output_mode_${mode}`)}
                </option>
              ))}
            </select>
          </label>
        </div>
      </section>

      {!hasProviders && (
        <div className="empty-state">
          <p>{t("no_provider_configured")}</p>
          <button type="button" onClick={() => onPageChange("settings-compute")}>
            {t("configure_provider")}
          </button>
        </div>
      )}

      <section className="file-section">
        <div className="file-row">
          <div className="file-info">
            <span className="file-label">{t("source")}</span>
            <span className="file-path" title={form.source || undefined} data-testid="source-path">
              {form.source || t("no_file_selected")}
            </span>
            {validation.errors.source && (
              <span className="inline-error">{validation.errors.source}</span>
            )}
          </div>
          <button type="button" onClick={selectSource} disabled={loading}>
            {t("select_file")}
          </button>
        </div>

        <div className="file-row">
          <div className="file-info">
            <span className="file-label">{t("output")}</span>
            <span className="file-path" title={form.output || undefined} data-testid="output-path">
              {form.output || t("no_file_selected")}
            </span>
            {validation.errors.output && (
              <span className="inline-error">{validation.errors.output}</span>
            )}
          </div>
          <button type="button" onClick={selectOutput} disabled={loading}>
            {t("save_as")}
          </button>
        </div>
      </section>

      {validation.reason && (
        <p className="next-step">
          {validation.reason}{" "}
          <span onClick={() => onPageChange("settings-compute")}>
            {t("open_settings")}
          </span>
        </p>
      )}

      <button
        className="start-button"
        type="button"
        onClick={onStart}
        disabled={loading || !validation.valid}
        data-testid="start-button"
      >
        {loading ? t("loading") : t("start")}
      </button>

      {(loading || progress.percent > 0 || progress.message) && (
        <section className="progress-section" data-testid="progress-section">
          <div className="progress-header">
            <span>{t("progress")}</span>
            <span>{progress.percent}%</span>
          </div>
          <div className="progress-bar">
            <div
              className="progress-fill"
              style={{ width: `${progress.percent}%` }}
            />
          </div>
          <p className="progress-message" data-testid="progress-message">{progress.message}</p>
        </section>
      )}
    </div>
  );
}
