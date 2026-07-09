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
  progress: ProgressState;
  validation: ValidationResult;
  onPageChange: (page: Page) => void;
}

export default function TranslatePage({
  form,
  setForm,
  onStart,
  progress,
  validation,
  onPageChange,
}: TranslatePageProps) {
  const { t } = useTranslation();
  const [checkpoints, setCheckpoints] = useState<CheckpointInfo[]>([]);

  const hasProviders = form.providers.length > 0;
  const activeProvider = form.providers.find((p) => p.provider === form.active_provider);
  const selectedCheckpoint = checkpoints.find((cp) => cp.job_id === form.resume);

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
          current_source: form.source || null,
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
    // Reload when a translation run finishes so newly created checkpoints appear.
  }, [form.checkpoint_dir, form.source, form.resume]);

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

  const basename = (path: string) => {
    const sep = path.includes("/") ? "/" : "\\";
    return path.split(sep).pop() || path;
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
          <button type="button" onClick={selectSource}>
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
          <button type="button" onClick={selectOutput}>
            {t("save_as")}
          </button>
        </div>
      </section>

      <section className="advanced-section">
        <div className="refine-option">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={form.refine}
              onChange={(e) => setForm("refine", e.target.checked)}
              data-testid="refine-checkbox"
            />
            {t("refine_translation")}
          </label>
          {form.refine && (
            <p className="refine-hint">
              {t("refine_translation_hint")}{" "}
              <button
                type="button"
                className="text-button"
                onClick={() => onPageChange("settings-prompts")}
              >
                {t("refine_translation_prompt_link")}
              </button>
            </p>
          )}
        </div>

        <div className="checkpoint-section">
          {!form.checkpoint_dir ? (
            <div className="checkpoint-setup-prompt">
              <p>{t("checkpoint_setup_prompt")}</p>
              <button
                type="button"
                className="secondary-button"
                onClick={() => onPageChange("settings-output")}
              >
                {t("checkpoint_setup_link")}
              </button>
            </div>
          ) : (
            <div className="checkpoint-list" data-testid="checkpoint-list">
              {checkpoints.length === 0 ? (
                <p className="checkpoint-empty">{t("no_checkpoints")}</p>
              ) : (
                <>
                  <p className="checkpoint-hint">{t("checkpoint_hint")}</p>
                  {checkpoints.map((cp) => (
                    <div
                      key={cp.job_id}
                      className={`checkpoint-item ${form.resume === cp.job_id ? "selected" : ""} ${
                        cp.matches_current_source ? "matches-source" : ""
                      }`}
                      onClick={() => setForm("resume", cp.job_id)}
                      role="button"
                      tabIndex={0}
                      data-testid={`checkpoint-item-${cp.job_id}`}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          setForm("resume", cp.job_id);
                        }
                      }}
                      title={cp.source_path}
                    >
                      <div className="checkpoint-meta">
                        <span className="checkpoint-id">{basename(cp.source_path)}</span>
                        <span className="checkpoint-progress">
                          {cp.completed}/{cp.total} {t("chapters_done")}
                        </span>
                      </div>
                      <div className="checkpoint-stats">
                        {cp.pending > 0 && (
                          <span className="checkpoint-pending">
                            {cp.pending} {t("chapters_pending")}
                          </span>
                        )}
                        {cp.failed > 0 && (
                          <span className="checkpoint-failed">
                            {cp.failed} {t("chapters_failed")}
                          </span>
                        )}
                        {cp.matches_current_source && (
                          <span className="checkpoint-match-badge">
                            {t("checkpoint_matches_current_source")}
                          </span>
                        )}
                      </div>
                    </div>
                  ))}
                </>
              )}
            </div>
          )}
        </div>
      </section>

      <div className="start-row">
        <button
          className="start-button"
          type="button"
          onClick={onStart}
          disabled={!validation.valid}
          data-testid="start-button"
        >
          {t("start")}
        </button>
        {form.resume && (
          <button
            type="button"
            className="secondary-button resume-clear-button"
            onClick={() => setForm("resume", "")}
            data-testid="clear-resume-selection"
            title={t("clear_resume_selection")}
          >
            {t("clear_resume_selection")}
          </button>
        )}
      </div>

      {selectedCheckpoint && !selectedCheckpoint.matches_current_source && (
        <div className="checkpoint-mismatch-warning">
          {t("checkpoint_source_mismatch_warning")}
        </div>
      )}

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
        <p className="progress-message" data-testid="progress-message">
          {progress.message || t("waiting")}
        </p>
      </section>
    </div>
  );
}
