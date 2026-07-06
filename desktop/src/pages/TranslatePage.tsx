import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import type { FormState, Page, ProgressState, ValidationResult } from "../types";
import {
  outputModes,
  providers,
  recommendedModels,
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

  const modelList = recommendedModels[form.provider] ?? [];
  const modelIsCustom = !modelList.some((m) => m.value === form.model);

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

  const handleModelChange = (value: string) => {
    if (value === "__custom__") {
      setForm("model", "");
    } else {
      setForm("model", value);
    }
  };

  return (
    <div className="page translate-page">
      <h2>{t("nav_translate")}</h2>

      <section className="quick-settings">
        <div className="row">
          <label>
            {t("provider")}
            <select
              value={form.provider}
              onChange={(e) => setForm("provider", e.target.value)}
            >
              {providers.map((p) => (
                <option key={p} value={p}>
                  {t(`provider_${p}`)}
                </option>
              ))}
            </select>
          </label>

          <label>
            {t("model")}
            {!modelIsCustom ? (
              <select
                value={form.model}
                onChange={(e) => handleModelChange(e.target.value)}
              >
                {modelList.map((m) => (
                  <option key={m.value} value={m.value}>
                    {m.label}
                  </option>
                ))}
                <option value="__custom__">{t("model_custom")}</option>
              </select>
            ) : (
              <input
                type="text"
                value={form.model}
                onChange={(e) => setForm("model", e.target.value)}
                placeholder={t("model_custom_placeholder")}
              />
            )}
          </label>

          <label>
            {t("source_lang")}
            <select
              value={form.source_lang}
              onChange={(e) => setForm("source_lang", e.target.value)}
            >
              {sourceLanguages.map((lang) => (
                <option key={lang.code} value={lang.code}>
                  {lang.label}
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
                  {lang.label}
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
