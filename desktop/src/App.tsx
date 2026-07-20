import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import "./styles/theme.css";
import "./styles/base.css";
import "./styles/form.css";
import type {
  FormState,
  ModelParams,
  OutputSettingsState,
  Page,
  ProviderConfig,
  PromptSettingsState,
  QueueSettingsState,
  TranslateInputs,
  TranslationSettingsState,
  ValidationResult,
} from "./types";
import { defaults } from "./types";
import TranslatePage from "./pages/TranslatePage";
import OcrPage from "./pages/OcrPage";
import OcrSettingsPage from "./pages/OcrSettingsPage";
import ComputeSettingsPage from "./pages/ComputeSettingsPage";
import ModelParamsPage from "./pages/ModelParamsPage";
import TranslationSettingsPage from "./pages/TranslationSettingsPage";
import PromptsPage from "./pages/PromptsPage";
import OutputSettingsPage from "./pages/OutputSettingsPage";
import QueueSettingsPage from "./pages/QueueSettingsPage";
import GeneralSettingsPage from "./pages/GeneralSettingsPage";
import AboutPage from "./pages/AboutPage";
import LegalPage from "./pages/LegalPage";
import LogsPage from "./pages/LogsPage";
import TasksPage from "./pages/TasksPage";
import SettingsLayout from "./pages/SettingsLayout";
import ToastContainer from "./components/ToastContainer";
import type { Toast } from "./components/ToastContainer";
import NavIcon from "./components/NavIcon";
import { useQueue } from "./hooks/useQueue";
import { useLogState } from "./hooks/useLogState";
import {
  DEFAULT_OCR_SETTINGS,
  loadGeneralSettings,
  loadOcrSettings,
  loadSettings,
  normalizeTheme,
  saveGeneralSettings,
  saveOcrSettings,
  saveSettings,
  type GeneralSettings,
  type OcrSettings,
} from "./config";
import { generateId } from "./utils";

interface E2EArgs {
  source?: string;
  output?: string;
  checkpoint_dir?: string;
  api_key?: string;
  dry_run?: boolean;
  ui_language?: string;
}

const DEFAULT_GENERAL: GeneralSettings = {
  ui_language: "en",
  theme: "dark",
  follow_system_language: true,
};

function activeProvider(form: FormState): ProviderConfig | undefined {
  return form.providers.find((p) => p.name === form.active_provider);
}

function uniqueProviderName(providers: ProviderConfig[], providerType: string): string {
  const used = new Set(providers.map((p) => p.name));
  let candidate = providerType;
  let index = 1;
  while (used.has(candidate)) {
    index += 1;
    candidate = `${providerType} ${index}`;
  }
  return candidate;
}

function ensureProvider(form: FormState, providerType: string): FormState {
  if (form.providers.some((p) => p.provider === providerType)) {
    return form;
  }
  const name = uniqueProviderName(form.providers, providerType);
  return {
    ...form,
    providers: [
      ...form.providers,
      {
        name,
        provider: providerType,
        api_key: "",
        base_url: "",
        use_custom_base_url: false,
      },
    ],
    active_provider: name,
  };
}

function parseCommaList(value: string) {
  return value
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

function buildTranslateArgs(form: FormState): object {
  const provider = activeProvider(form);
  if (!provider) throw new Error("no provider");
  return {
    ...form,
    provider: provider.provider,
    api_key: provider.api_key,
    base_url: provider.use_custom_base_url ? provider.base_url || null : null,
    system_prompt: form.system_prompt || null,
    prompts: form.prompts,
    output_font: form.output_font || null,
    exclude_selectors: parseCommaList(form.exclude_selectors),
    translate_attributes: parseCommaList(form.translate_attributes),
    dry_run: !!form.dry_run,
    preserve_classes: !!form.preserve_classes,
    translate_body: !!form.translate_body,
    translate_metadata: !!form.translate_metadata,
    translate_toc: !!form.translate_toc,
    translate_alt_text: !!form.translate_alt_text,
    translate_image_captions: !!form.translate_image_captions,
    translate_tables: !!form.translate_tables,
    translate_footnotes: !!form.translate_footnotes,
    translate_code: !!form.translate_code,
    refine: !!form.refine,
    checkpoint_dir: form.checkpoint_dir,
    resume: form.resume || null,
  };
}

/**
 * Apply E2E-injected values to a form state and return the merged form plus the
 * injected output path, if any. Keeping this logic isolated makes the settings
 * loading effect easier to follow and test.
 */
function applyE2EOverrides(
  form: FormState,
  e2e: E2EArgs
): { form: FormState; injectedOutput: string | null } {
  let merged = form;
  let injectedOutput: string | null = null;

  if (e2e.api_key) {
    merged = {
      ...merged,
      providers: merged.providers.map((p) =>
        p.name === merged.active_provider ? { ...p, api_key: e2e.api_key ?? "" } : p
      ),
    };
  }
  if (e2e.source) {
    merged = { ...merged, source: e2e.source };
  }
  if (e2e.output) {
    merged = { ...merged, output: e2e.output };
    injectedOutput = e2e.output;
  }
  if (e2e.checkpoint_dir) {
    merged = { ...merged, checkpoint_dir: e2e.checkpoint_dir };
  }
  if (e2e.dry_run !== undefined) {
    merged = { ...merged, dry_run: e2e.dry_run };
  }

  return { form: merged, injectedOutput };
}

function App() {
  const { t, i18n } = useTranslation();
  const [form, setForm] = useState<FormState>(() => ({
    ...defaults,
    target_lang: defaults.target_lang,
  }));
  const [page, setPage] = useState<Page>("translate");
  const [toasts, setToasts] = useState<Toast[]>([]);

  const [general, setGeneral] = useState<GeneralSettings>(DEFAULT_GENERAL);
  const [ocrSettings, setOcrSettings] = useState<OcrSettings>(DEFAULT_OCR_SETTINGS);
  const [detectedLocale, setDetectedLocale] = useState<string>("en");

  const {
    queue,
    currentTask,
    runningTaskCount,
    refreshQueue,
    removeTask,
    retryTask,
    cancelTask,
    reorderTasks,
    pauseTask,
    resumeTask,
    startQueue,
    pauseQueue,
  } = useQueue();
  const { logs, clearLogs, appendError } = useLogState();

  // Focused slices derived from the serializable FormState. Passing slices to
  // settings pages instead of the whole FormState reduces coupling and, together
  // with memo(), prevents re-renders when unrelated fields change.
  const modelParams: ModelParams = useMemo(
    () => ({
      model: form.model,
      max_input_tokens: form.max_input_tokens,
      max_output_tokens: form.max_output_tokens,
      temperature: form.temperature,
    }),
    [form.model, form.max_input_tokens, form.max_output_tokens, form.temperature]
  );

  const translationSettings: TranslationSettingsState = useMemo(
    () => ({
      source_lang: form.source_lang,
      target_lang: form.target_lang,
      output_mode: form.output_mode,
      style: form.style,
      preserve_classes: form.preserve_classes,
      exclude_selectors: form.exclude_selectors,
      translate_attributes: form.translate_attributes,
      translate_body: form.translate_body,
      translate_metadata: form.translate_metadata,
      translate_toc: form.translate_toc,
      translate_alt_text: form.translate_alt_text,
      translate_image_captions: form.translate_image_captions,
      translate_tables: form.translate_tables,
      translate_footnotes: form.translate_footnotes,
      translate_code: form.translate_code,
    }),
    [
      form.source_lang,
      form.target_lang,
      form.output_mode,
      form.style,
      form.preserve_classes,
      form.exclude_selectors,
      form.translate_attributes,
      form.translate_body,
      form.translate_metadata,
      form.translate_toc,
      form.translate_alt_text,
      form.translate_image_captions,
      form.translate_tables,
      form.translate_footnotes,
      form.translate_code,
    ]
  );

  const promptSettings: PromptSettingsState = useMemo(
    () => ({ system_prompt: form.system_prompt, prompts: form.prompts }),
    [form.system_prompt, form.prompts]
  );

  const outputSettings: OutputSettingsState = useMemo(
    () => ({
      output_font: form.output_font,
      output_filename_template: form.output_filename_template,
      checkpoint_dir: form.checkpoint_dir,
    }),
    [form.output_font, form.output_filename_template, form.checkpoint_dir]
  );

  const queueSettings: QueueSettingsState = useMemo(
    () => ({ concurrency: form.concurrency }),
    [form.concurrency]
  );

  const translateInputs: TranslateInputs = useMemo(
    () => ({
      source: form.source,
      output: form.output,
      source_lang: form.source_lang,
      target_lang: form.target_lang,
      output_mode: form.output_mode,
      providers: form.providers,
      active_provider: form.active_provider,
      model: form.model,
      checkpoint_dir: form.checkpoint_dir,
      resume: form.resume,
      refine: form.refine,
    }),
    [
      form.source,
      form.output,
      form.source_lang,
      form.target_lang,
      form.output_mode,
      form.providers,
      form.active_provider,
      form.model,
      form.checkpoint_dir,
      form.resume,
      form.refine,
    ]
  );

  const setModelParams = useCallback(
    (update: Partial<ModelParams>) => setForm((prev) => ({ ...prev, ...update })),
    []
  );
  const setTranslationSettings = useCallback(
    (update: Partial<TranslationSettingsState>) => setForm((prev) => ({ ...prev, ...update })),
    []
  );
  const setPromptSettings = useCallback(
    (update: Partial<PromptSettingsState>) => setForm((prev) => ({ ...prev, ...update })),
    []
  );
  const setOutputSettings = useCallback(
    (update: Partial<OutputSettingsState>) => setForm((prev) => ({ ...prev, ...update })),
    []
  );
  const setQueueSettings = useCallback(
    (update: Partial<QueueSettingsState>) => setForm((prev) => ({ ...prev, ...update })),
    []
  );
  const setInputs = useCallback(
    (update: Partial<TranslateInputs>) => setForm((prev) => ({ ...prev, ...update })),
    []
  );

  const [sidebarWidth, setSidebarWidth] = useState(180);
  const isResizing = useRef(false);
  const startX = useRef(0);
  const startWidth = useRef(0);

  const beginResize = (e: React.MouseEvent) => {
    isResizing.current = true;
    startX.current = e.clientX;
    startWidth.current = sidebarWidth;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing.current) return;
      const delta = e.clientX - startX.current;
      const next = Math.max(140, Math.min(320, startWidth.current + delta));
      setSidebarWidth(next);
    };
    const handleMouseUp = () => {
      if (!isResizing.current) return;
      isResizing.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [sidebarWidth]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (!e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
      switch (e.key) {
        case "1":
          e.preventDefault();
          setPage("translate");
          break;
        case "2":
          e.preventDefault();
          setPage("logs");
          break;
        case "3":
          e.preventDefault();
          setPage("tasks");
          break;
        case "4":
          e.preventDefault();
          setPage("settings-compute");
          break;
        case "5":
          e.preventDefault();
          setPage("about");
          break;
        default:
          break;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const e2eOutputRef = useRef<string | null>(null);
  const generalLoadedRef = useRef(false);
  const ocrLoadedRef = useRef(false);
  const initialLanguageAppliedRef = useRef(false);

  // Load persisted settings on mount, optionally overridden by E2E env args.
  useEffect(() => {
    void (async () => {
      try {
        const [settings, generalSettings, ocrSettingsLoaded, e2e] = await Promise.all([
          loadSettings(),
          loadGeneralSettings(),
          loadOcrSettings(),
          invoke<E2EArgs>("get_e2e_args").catch((err) => {
            console.error("get_e2e_args failed:", err);
            return {} as E2EArgs;
          }),
        ]);
        setGeneral(generalSettings);
        generalLoadedRef.current = true;
        setOcrSettings(ocrSettingsLoaded);
        ocrLoadedRef.current = true;

        let merged = { ...form, ...settings } as FormState;

        // If no providers exist (fresh install), seed a default provider config so
        // the UI is never in a broken state. E2E can override the API key below.
        if (!merged.providers || merged.providers.length === 0) {
          const defaultProvider = "deepseek";
          merged = ensureProvider(merged, defaultProvider);
        }
        if (!merged.active_provider || !merged.providers.some((p) => p.name === merged.active_provider)) {
          merged = { ...merged, active_provider: merged.providers[0].name };
        }

        const e2eResult = applyE2EOverrides(merged, e2e);
        merged = e2eResult.form;
        if (e2eResult.injectedOutput) {
          e2eOutputRef.current = e2eResult.injectedOutput;
        }

        // If no target language was saved, default to the UI language.
        if (!settings.target_lang) {
          merged = { ...merged, target_lang: generalSettings.ui_language ?? merged.target_lang };
        }

        setForm(merged);

        // Apply UI language after settings are loaded so we use the persisted
        // preference rather than the hard-coded default. When following the
        // system language, resolve the system locale first.
        if (e2e.ui_language) {
          await i18n.changeLanguage(e2e.ui_language);
        } else if (generalSettings.follow_system_language) {
          const locale = await invoke<string>("get_system_locale");
          setDetectedLocale(locale);
          await i18n.changeLanguage(locale);
        } else {
          await i18n.changeLanguage(generalSettings.ui_language);
        }
        initialLanguageAppliedRef.current = true;
      } catch (err) {
        console.error("[settings] initialization failed:", err);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Save settings when form changes (debounced). API keys are persisted to the
  // OS keyring inside saveSettings.
  useEffect(() => {
    const timer = setTimeout(() => {
      void saveSettings(form);
    }, 500);
    return () => clearTimeout(timer);
  }, [form]);

  // Auto-suggest the output path whenever the source file or output naming
  // settings change. The user can still override it with the "Save as" dialog.
  useEffect(() => {
    const timer = setTimeout(() => {
      if (!form.source) {
        setForm((prev) => (prev.output ? { ...prev, output: "" } : prev));
        return;
      }
      void (async () => {
        const suggested = await invoke<string>("suggest_output_path", {
          source: form.source,
          sourceLang: form.source_lang,
          targetLang: form.target_lang,
          outputMode: form.output_mode,
          outputFilenameTemplate: form.output_filename_template,
        }).catch(() => null);
        if (suggested) {
          // Preserve output paths injected by E2E on the first suggestion cycle.
          if (form.output === e2eOutputRef.current) {
            e2eOutputRef.current = null;
            return;
          }
          setForm((prev) => ({ ...prev, output: suggested }));
        }
      })();
    }, 100);
    return () => clearTimeout(timer);
  }, [
    form.source,
    form.source_lang,
    form.target_lang,
    form.output_mode,
    form.output_filename_template,
  ]);

  // Persist general UI settings when they change.
  useEffect(() => {
    if (!generalLoadedRef.current) return;
    void saveGeneralSettings(general);
  }, [general]);

  // Persist OCR engine defaults when they change.
  useEffect(() => {
    if (!ocrLoadedRef.current) return;
    void saveOcrSettings(ocrSettings);
  }, [ocrSettings]);

  // Sync the active theme to the document root so that every element inherits
  // the correct CSS variables, not just descendants of .app-shell.
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", general.theme);
  }, [general.theme]);

  // Keep the UI language in sync with general settings. When following the
  // system language, re-resolve the locale whenever the toggle is turned on.
  // Skip the initial mount because the settings-loading effect above already
  // applied the persisted (or E2E-injected) language.
  useEffect(() => {
    if (!initialLanguageAppliedRef.current) return;
    if (general.follow_system_language) {
      void invoke<string>("get_system_locale")
        .then((locale) => {
          setDetectedLocale(locale);
          return i18n.changeLanguage(locale);
        })
        .catch((err) => {
          console.error("[locale] failed to apply system language:", err);
        });
    } else {
      void i18n.changeLanguage(general.ui_language).catch((err) => {
        console.error("[locale] failed to apply UI language:", err);
      });
    }
  }, [general.follow_system_language, general.ui_language, i18n]);

  const validation: ValidationResult = useMemo(() => {
    const errors: ValidationResult["errors"] = {};
    const provider = activeProvider(form);

    if (!form.source) errors.source = t("error_source_required");
    if (!form.output) errors.output = t("error_output_required");
    if (!provider) {
      errors.api_key = t("error_no_provider");
    } else if (provider.provider !== "ollama" && !provider.api_key) {
      errors.api_key = t("error_api_key_required");
    }

    let reason: string | undefined;
    if (!form.source || !form.output) {
      reason = t("error_required");
    } else if (!provider) {
      reason = t("error_no_provider");
    } else if (provider.provider !== "ollama" && !provider.api_key) {
      reason = t("error_api_key");
    }

    return { valid: Object.keys(errors).length === 0, errors, reason };
  }, [form, t]);

  async function handleStart() {
    if (!validation.valid) return;

    const provider = activeProvider(form);
    if (!provider) return;

    if (!form.dry_run) {
      const exists = await invoke<boolean>("check_file_exists", { path: form.output });
      if (exists) {
        const yes = await confirm(t("confirm_overwrite"), {
          title: t("confirm_overwrite_title"),
          kind: "warning",
        });
        if (!yes) return;
      }
    }

    try {
      const args = buildTranslateArgs(form);
      await invoke("enqueue_task", { args });
      await invoke("start_queue");
      await refreshQueue();
    } catch (err) {
      appendError(`${t("error")}: ${err}`);
    }
  }

  async function handleDryRun() {
    if (!validation.valid) return;
    try {
      const args = { ...buildTranslateArgs(form), dry_run: true };
      await invoke("enqueue_task", { args });
      await invoke("start_queue");
      await refreshQueue();
    } catch (err) {
      appendError(`${t("error")}: ${err}`);
    }
  }

  const updateForm = <K extends keyof FormState>(key: K, value: FormState[K]) => {
    setForm((prev) => {
      const next = { ...prev, [key]: value } as FormState;
      if (key === "active_provider") {
        const activeStillExists = next.providers.some((p) => p.name === value);
        if (!activeStillExists && next.providers.length > 0) {
          next.active_provider = next.providers[0].name;
        }
      }
      return next;
    });
  };

  const updateProviders = (providers: ProviderConfig[]) => {
    setForm((prev) => {
      const activeStillExists = providers.some((p) => p.name === prev.active_provider);
      return {
        ...prev,
        providers,
        active_provider: activeStillExists ? prev.active_provider : providers[0]?.provider ?? "",
      };
    });
  };

  const showToast = (message: string, kind: Toast["kind"] = "info") => {
    const id = generateId();
    setToasts((prev) => [...prev, { id, message, kind }]);
  };

  const dismissToast = (id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  };

  const renderPage = () => {
    switch (page) {
      case "ocr":
        return (
          <OcrPage
            inputs={translateInputs}
            setInputs={setInputs}
            onPageChange={setPage}
            ocrSettings={ocrSettings}
          />
        );
      case "translate":
        return (
          <TranslatePage
            inputs={translateInputs}
            setInputs={setInputs}
            onStart={handleStart}
            onDryRun={handleDryRun}
            currentTask={currentTask}
            validation={validation}
            onPageChange={setPage}
            logs={logs}
            onClearLogs={clearLogs}
          />
        );
      case "logs":
        return <LogsPage entries={logs} onClear={clearLogs} />;
      case "tasks":
        return (
          <TasksPage
            queue={queue}
            onRemove={removeTask}
            onRetry={retryTask}
            onCancel={cancelTask}
            onPauseTask={pauseTask}
            onResumeTask={resumeTask}
            onStart={startQueue}
            onPause={pauseQueue}
            onReorder={reorderTasks}
            onNavigate={setPage}
          />
        );
      case "settings-compute":
      case "settings-model":
      case "settings-translation":
      case "settings-prompts":
      case "settings-output":
      case "settings-queue":
      case "settings-general":
      case "settings-ocr":
        return (
          <SettingsLayout activePage={page} onNavigate={setPage}>
            {page === "settings-compute" && (
              <ComputeSettingsPage
                providers={form.providers}
                activeProvider={form.active_provider}
                onChangeProviders={updateProviders}
                onChangeActiveProvider={(provider) => updateForm("active_provider", provider)}
              />
            )}
            {page === "settings-model" && (
              <ModelParamsPage modelParams={modelParams} setModelParams={setModelParams} />
            )}
            {page === "settings-translation" && (
              <TranslationSettingsPage
                settings={translationSettings}
                setSettings={setTranslationSettings}
              />
            )}
            {page === "settings-prompts" && (
              <PromptsPage promptSettings={promptSettings} setPromptSettings={setPromptSettings} />
            )}
            {page === "settings-output" && (
              <OutputSettingsPage
                outputSettings={outputSettings}
                setOutputSettings={setOutputSettings}
                targetLang={form.target_lang}
              />
            )}
            {page === "settings-queue" && (
              <QueueSettingsPage queueSettings={queueSettings} setQueueSettings={setQueueSettings} />
            )}
            {page === "settings-ocr" && (
              <OcrSettingsPage ocrSettings={ocrSettings} setOcrSettings={setOcrSettings} />
            )}
            {page === "settings-general" && (
              <GeneralSettingsPage
                general={general}
                setGeneral={setGeneral}
                detectedLocale={detectedLocale}
                onToast={showToast}
                onImport={async (settings) => {
                  const merged = { ...form, ...settings.translation } as FormState;
                  if (
                    !merged.active_provider ||
                    !merged.providers.some((p) => p.name === merged.active_provider)
                  ) {
                    merged.active_provider = merged.providers[0]?.name ?? "";
                  }
                  const general = {
                    ...settings.general,
                    theme: normalizeTheme(settings.general.theme),
                  };
                  setForm(merged);
                  setGeneral(general);
                  await saveSettings(merged);
                  await saveGeneralSettings(general);
                  if (settings.ocr) {
                    setOcrSettings(settings.ocr);
                    await saveOcrSettings(settings.ocr);
                  }
                }}
              />
            )}
          </SettingsLayout>
        );
      case "about":
        return <AboutPage onOpenLegal={() => setPage("legal")} />;
      case "legal":
        return <LegalPage onBack={() => setPage("about")} />;
      default:
        return null;
    }
  };

  return (
    <div className="app-shell" data-theme={general.theme}>
      <aside
        className="sidebar"
        style={{ width: sidebarWidth, minWidth: sidebarWidth }}
      >
        <div className="brand">
          <h1>{t("app_title")}</h1>
        </div>

        <nav>
          <div className="nav-group-top">
            <button
              type="button"
              className={`nav-item ${page === "translate" ? "active" : ""}`}
              onClick={() => setPage("translate")}
              data-testid="nav-translate"
            >
              <NavIcon icon="translate" className="nav-item-icon" />
              <span className="nav-item-label">{t("nav_translate")}</span>
            </button>

            <button
              type="button"
              className={`nav-item ${page === "ocr" ? "active" : ""}`}
              onClick={() => setPage("ocr")}
              data-testid="nav-ocr"
            >
              <NavIcon icon="ocr" className="nav-item-icon" />
              <span className="nav-item-label">{t("nav_ocr")}</span>
            </button>

            <button
              type="button"
              className={`nav-item ${page === "tasks" ? "active" : ""}`}
              onClick={() => setPage("tasks")}
              data-testid="nav-tasks"
            >
              <NavIcon icon="tasks" className="nav-item-icon" />
              <span className="nav-item-label">{t("nav_tasks")}</span>
              {runningTaskCount > 0 && (
                <span className="nav-badge" aria-label={t("task_status_running")}>
                  {runningTaskCount}
                </span>
              )}
            </button>

            <button
              type="button"
              className={`nav-item ${page === "logs" ? "active" : ""}`}
              onClick={() => setPage("logs")}
              data-testid="nav-logs"
            >
              <NavIcon icon="logs" className="nav-item-icon" />
              <span className="nav-item-label">{t("nav_logs")}</span>
            </button>
          </div>

          <div className="nav-group-bottom">
            <button
              type="button"
              className={`nav-item ${page.startsWith("settings-") ? "active" : ""}`}
              onClick={() => setPage("settings-compute")}
              data-testid="nav-settings"
            >
              <NavIcon icon="settings" className="nav-item-icon" />
              <span className="nav-item-label">{t("nav_settings")}</span>
            </button>

            <button
              type="button"
              className={`nav-item ${page === "about" ? "active" : ""}`}
              onClick={() => setPage("about")}
            >
              <NavIcon icon="about" className="nav-item-icon" />
              <span className="nav-item-label">{t("nav_about")}</span>
            </button>
          </div>
        </nav>

        <div
          className="sidebar-resizer"
          onMouseDown={beginResize}
          title="Drag to resize sidebar"
        />
      </aside>

      <main className="main-content">{renderPage()}</main>
      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
}

export default App;
