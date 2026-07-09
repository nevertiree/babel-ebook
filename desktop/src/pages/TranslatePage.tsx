import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { CheckpointInfo, FormState, Page, ProgressState, ValidationResult } from "../types";
import {
  outputModes,
  recommendedModels,
  sourceLanguages,
  targetLanguages,
} from "../types";

interface ModelSelectProps {
  provider: string;
  model: string;
  onChange: (value: string) => void;
}

function ModelSelect({ provider, model, onChange }: ModelSelectProps) {
  const { t } = useTranslation();
  const models = recommendedModels[provider] ?? [];
  const isCustom = !models.some((m) => m.value === model);

  if (models.length === 0) {
    return (
      <label>
        {t("model")}
        <input
          type="text"
          value={model}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t("model_custom_placeholder")}
        />
      </label>
    );
  }

  return (
    <label>
      {t("model")}
      {isCustom ? (
        <input
          type="text"
          value={model}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t("model_custom_placeholder")}
        />
      ) : (
        <select value={model} onChange={(e) => onChange(e.target.value)}>
          {models.map((m) => (
            <option key={m.value} value={m.value}>
              {m.label}
            </option>
          ))}
          <option value="__custom__">{t("model_custom")}</option>
        </select>
      )}
    </label>
  );
}

interface TranslatePageProps {
  form: FormState;
  setForm: <K extends keyof FormState>(key: K, value: FormState[K]) => void;
  onStart: () => void;
  onEnqueue: () => void;
  loading: boolean;
  progress: ProgressState;
  validation: ValidationResult;
  onPageChange: (page: Page) => void;
}

export default function TranslatePage({
  form,
  setForm,
  onStart,
  onEnqueue,
  loading,
  progress,
  validation,
  onPageChange,
}: TranslatePageProps) {
  const { t } = useTranslation();
  const [checkpoints, setCheckpoints] = useState<CheckpointInfo[]>([]);
  const [showCheckpoints, setShowCheckpoints] = useState(false);

  const hasProviders = form.providers.length > 0;
  const activeProvider = form.providers.find((p) => p.provider === form.active_provider);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      if (!form.checkpoint_dir) {
        setCheckpoints([]);
        return;
      }
      try {
        const list = await invoke<CheckpointInfo[]>("list_checkpoints", {
          checkpoint_dir: form.checkpoint_dir,
        });
        if (!cancelled) {
          setCheckpoints(list);
        }
      } catch {
        if (!cancelled) {
          setCheckpoints([]);
        }
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [form.checkpoint_dir, form.resume, showCheckpoints]);

  const selectSource = async () => {
    const path = await open({
      filters: [
        {
          name: t("ebook_files"),
          extensions: ["epub", "mobi", "azw3", "txt", "srt", "docx"],
        },
      ],
    });
    if (path) {
      setForm("source", path);
      // Clear any previous resume selection when a new source is chosen.
      if (form.resume) {
        setForm("resume", "");
      }
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
            <>
              <label>
                {t("provider")}
                <select
                  value={form.active_provider}
                  onChange={(e) => setForm("active_provider", e.target.value)}
                >
                  {form.providers.map((p) => (
                    <option key={p.name} value={p.name}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </label>

              <ModelSelect
                provider={activeProvider.provider}
                model={form.model}
                onChange={(value) => setForm("model", value)}
              />
            </>
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

      <section className="advanced-section">
        <div className="row">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={form.refine}
              onChange={(e) => setForm("refine", e.target.checked)}
            />
            {t("refine_translation")}
          </label>
        </div>

        <div className="checkpoint-section">
          <button
            type="button"
            className="secondary-button"
            onClick={() => setShowCheckpoints((prev) => !prev)}
            disabled={!form.checkpoint_dir}
          >
            {showCheckpoints ? t("hide_checkpoints") : t("show_checkpoints")}
          </button>

          {showCheckpoints && (
            <div className="checkpoint-list">
              {checkpoints.length === 0 ? (
                <p className="checkpoint-empty">{t("no_checkpoints")}</p>
              ) : (
                <>
                  <p className="checkpoint-hint">{t("checkpoint_hint")}</p>
                  {checkpoints.map((cp) => (
                    <div
                      key={cp.job_id}
                      className={`checkpoint-item ${form.resume === cp.job_id ? "selected" : ""}`}
                      onClick={() => setForm("resume", cp.job_id)}
                      role="button"
                      tabIndex={0}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          setForm("resume", cp.job_id);
                        }
                      }}
                    >
                      <div className="checkpoint-meta">
                        <span className="checkpoint-id">{cp.job_id}</span>
                        <span className="checkpoint-progress">
                          {cp.completed}/{cp.total} {t("chapters_done")}
                        </span>
                      </div>
                      {cp.failed > 0 && (
                        <span className="checkpoint-failed">
                          {cp.failed} {t("chapters_failed")}
                        </span>
                      )}
                    </div>
                  ))}
                  {form.resume && (
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => setForm("resume", "")}
                    >
                      {t("clear_resume_selection")}
                    </button>
                  )}
                </>
              )}
            </div>
          )}
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

      <div className="start-row">
        <button
          className="start-button"
          type="button"
          onClick={onStart}
          disabled={loading || !validation.valid}
          data-testid="start-button"
        >
          {loading ? t("loading") : t("start")}
        </button>

        <button
          type="button"
          onClick={onEnqueue}
          disabled={!validation.valid}
          data-testid="enqueue-button"
        >
          {t("add_to_queue")}
        </button>
      </div>

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
