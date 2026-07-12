import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { memo, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { CheckpointInfo, LogEntry, Page, Task, TranslateInputs, ValidationResult } from "../types";
import {
  outputModes,
  targetLanguages,
} from "../types";
import RunningPanel from "../components/RunningPanel";
import LoadingSpinner from "../components/LoadingSpinner";
import EmptyStateIcon from "../components/EmptyStateIcon";
import ProviderIcon from "../components/ProviderIcon";
import "./TranslatePage.css";

interface ModelSelectProps {
  provider: string;
  apiKey: string;
  baseUrl: string;
  useCustomBaseUrl: boolean;
  model: string;
  onChange: (value: string) => void;
}

function ModelSelect({
  provider,
  apiKey,
  baseUrl,
  useCustomBaseUrl,
  model,
  onChange,
}: ModelSelectProps) {
  const { t } = useTranslation();
  const [models, setModels] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestIdRef = useRef(0);

  useEffect(() => {
    const requestId = (requestIdRef.current += 1);
    setLoading(true);
    setError(null);

    void (async () => {
      try {
        const list = await invoke<string[]>("list_models", {
          args: {
            provider,
            api_key: apiKey,
            base_url: useCustomBaseUrl ? baseUrl || null : null,
          },
        });
        if (requestId !== requestIdRef.current) return;
        setModels(list);
        if (list.length > 0 && !list.includes(model) && model !== "__custom__") {
          onChange(list[0]);
        }
      } catch (err) {
        if (requestId !== requestIdRef.current) return;
        setModels([]);
        setError(String(err));
      } finally {
        if (requestId === requestIdRef.current) {
          setLoading(false);
        }
      }
    })();

    return () => {
      // Increment the id so that any in-flight response for this effect is
      // discarded when the dependencies change.
      requestIdRef.current += 1;
    };
  }, [provider, apiKey, baseUrl, useCustomBaseUrl]);

  const isCustom = model === "__custom__" || (models.length > 0 && !models.includes(model));

  const showSpinner = loading && models.length === 0;

  return (
    <label title={error ?? undefined}>
      <span className="field-row">
        {t("model")}
        {showSpinner && <LoadingSpinner size={14} />}
      </span>
      {models.length === 0 ? (
        <input
          type="text"
          value={model === "__custom__" ? "" : model}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t("model_custom_placeholder")}
          disabled={loading}
        />
      ) : isCustom ? (
        <input
          type="text"
          value={model === "__custom__" ? "" : model}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t("model_custom_placeholder")}
          disabled={loading}
        />
      ) : (
        <select
          value={model}
          onChange={(e) => onChange(e.target.value)}
          disabled={loading}
        >
          {models.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
          <option value="__custom__">{t("model_custom")}</option>
        </select>
      )}
    </label>
  );
}

interface TranslatePageProps {
  inputs: TranslateInputs;
  setInputs: (update: Partial<TranslateInputs>) => void;
  onStart: () => void;
  onDryRun: () => void;
  currentTask?: Task;
  validation: ValidationResult;
  onPageChange: (page: Page) => void;
  logs: LogEntry[];
  onClearLogs: () => void;
}

function TranslatePage({
  inputs,
  setInputs,
  onStart,
  onDryRun,
  currentTask,
  validation,
  onPageChange,
  logs,
  onClearLogs,
}: TranslatePageProps) {
  const { t } = useTranslation();
  const [checkpoints, setCheckpoints] = useState<CheckpointInfo[]>([]);

  const hasProviders = inputs.providers.length > 0;
  const activeProvider = inputs.providers.find((p) => p.name === inputs.active_provider);
  const selectedCheckpoint = checkpoints.find((cp) => cp.job_id === inputs.resume);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      if (!inputs.checkpoint_dir) {
        setCheckpoints([]);
        return;
      }
      try {
        const list = await invoke<CheckpointInfo[]>("list_checkpoints", {
          checkpoint_dir: inputs.checkpoint_dir,
          current_source: inputs.source || null,
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
  }, [inputs.checkpoint_dir, inputs.source, inputs.resume]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Enter" || !event.ctrlKey) return;
      const active = document.activeElement;
      if (
        active instanceof HTMLInputElement ||
        active instanceof HTMLTextAreaElement ||
        active instanceof HTMLSelectElement
      ) {
        return;
      }
      if (!validation.valid) return;
      event.preventDefault();
      onStart();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [validation.valid, onStart]);

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
      setInputs({ source: path });
      // Clear any previous resume selection when a new source is chosen.
      if (inputs.resume) {
        setInputs({ resume: "" });
      }
    }
  };

  const selectOutput = async () => {
    const path = await save({
      filters: [{ name: "EPUB", extensions: ["epub"] }],
      defaultPath: inputs.output || "output.epub",
    });
    if (path) {
      setInputs({ output: path });
    }
  };

  const basename = (path: string) => {
    const sep = path.includes("/") ? "/" : "\\";
    return path.split(sep).pop() || path;
  };

  const sourceIsEpub = inputs.source?.toLowerCase().endsWith(".epub") ?? false;
  const showSourceFormatWarning = Boolean(inputs.source && !sourceIsEpub);

  // Drag-and-drop of files is intentionally not advertised: browsers do not
  // expose full filesystem paths through web drop events, so a dropped file
  // cannot be passed to the Tauri backend. We only prevent the default
  // browser behaviour to avoid navigating away when the user drags over the
  // window.
  useEffect(() => {
    const handleDragOver = (e: DragEvent) => {
      e.preventDefault();
    };
    const handleDrop = (e: DragEvent) => {
      e.preventDefault();
    };
    window.addEventListener("dragover", handleDragOver);
    window.addEventListener("drop", handleDrop);
    return () => {
      window.removeEventListener("dragover", handleDragOver);
      window.removeEventListener("drop", handleDrop);
    };
  }, []);

  return (
    <div className="page translate-page">
      <h2>{t("nav_translate")}</h2>

      <section className="quick-settings">
        <div className="row">
          {hasProviders && activeProvider && (
            <>
              <label>
                {t("provider")}
                <div className="provider-select">
                  {activeProvider && (
                    <ProviderIcon provider={activeProvider.provider} className="provider-select-icon" />
                  )}
                  <select
                    value={inputs.active_provider}
                    onChange={(e) => setInputs({ active_provider: e.target.value })}
                  >
                    {inputs.providers.map((p) => (
                      <option key={p.name} value={p.name}>
                        {p.name}
                      </option>
                    ))}
                  </select>
                </div>
              </label>

              <ModelSelect
                provider={activeProvider.provider}
                apiKey={activeProvider.api_key}
                baseUrl={activeProvider.base_url}
                useCustomBaseUrl={activeProvider.use_custom_base_url}
                model={inputs.model}
                onChange={(value) => setInputs({ model: value })}
              />
            </>
          )}

          <label>
            {t("target_lang")}
            <select
              value={inputs.target_lang}
              onChange={(e) => setInputs({ target_lang: e.target.value })}
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
              value={inputs.output_mode}
              onChange={(e) => setInputs({ output_mode: e.target.value })}
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
          <EmptyStateIcon variant="provider" className="empty-state-icon" />
          <p>{t("no_provider_configured")}</p>
          <button type="button" onClick={() => onPageChange("settings-compute")}>
            {t("configure_provider")}
          </button>
        </div>
      )}

      <section className="file-section">
        <div
          className="file-row file-row-source"
          role="button"
          tabIndex={0}
          onClick={selectSource}
          aria-label={t("drop_source_hint")}
        >
          <div className="file-info">
            <span className="file-label">{t("source")}</span>
            <span className="file-path" title={inputs.source || undefined} data-testid="source-path">
              {inputs.source || t("drop_source_hint")}
            </span>
            {showSourceFormatWarning && (
              <span className="inline-warning">{t("source_format_warning")}</span>
            )}
          </div>
          <div className="file-row-actions">
            {inputs.source && (
              <button
                type="button"
                className="icon-button"
                onClick={(e) => { e.stopPropagation(); setInputs({ source: "" }); }}
                title={t("clear")}
                aria-label={t("clear")}
              >
                ×
              </button>
            )}
            <button type="button" onClick={(e) => { e.stopPropagation(); selectSource(); }}>
              {t("select_file")}
            </button>
          </div>
        </div>

        <div className="file-row">
          <div className="file-info">
            <span className="file-label">{t("output")}</span>
            <span className="file-path" title={inputs.output || undefined} data-testid="output-path">
              {inputs.output || t("no_file_selected")}
            </span>
          </div>
          <div className="file-row-actions">
            {inputs.output && (
              <button
                type="button"
                className="icon-button"
                onClick={(e) => { e.stopPropagation(); setInputs({ output: "" }); }}
                title={t("clear")}
                aria-label={t("clear")}
              >
                ×
              </button>
            )}
            <button type="button" onClick={selectOutput}>
              {t("save_as")}
            </button>
          </div>
        </div>
      </section>

      <section className="advanced-section">
        <div className="refine-option">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={inputs.refine}
              onChange={(e) => setInputs({ refine: e.target.checked })}
              data-testid="refine-checkbox"
            />
            {t("refine_translation")}
          </label>
          {inputs.refine && (
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
          {!inputs.checkpoint_dir ? (
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
                      className={`checkpoint-item ${inputs.resume === cp.job_id ? "selected" : ""} ${
                        cp.matches_current_source ? "matches-source" : ""
                      }`}
                      onClick={() => setInputs({ resume: cp.job_id })}
                      role="button"
                      tabIndex={0}
                      data-testid={`checkpoint-item-${cp.job_id}`}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          setInputs({ resume: cp.job_id });
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
        <button
          type="button"
          className="secondary-button"
          onClick={onDryRun}
          disabled={!validation.valid}
          data-testid="dry-run-button"
          title={t("dry_run_hint")}
        >
          {t("dry_run")}
        </button>
        {inputs.resume && (
          <button
            type="button"
            className="secondary-button resume-clear-button"
            onClick={() => setInputs({ resume: "" })}
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

      <RunningPanel
        currentTask={currentTask}
        logs={logs}
        onClearLogs={onClearLogs}
      />
    </div>
  );
}

export default memo(TranslatePage);
