import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { confirm } from "@tauri-apps/plugin-dialog";
import "./App.css";
import type { FormState, LogEntry, Page, ProgressState, ProviderConfig, QueueState, Task, ValidationResult } from "./types";
import { defaults, recommendedModels, targetLanguages } from "./types";
import TranslatePage from "./pages/TranslatePage";
import ComputeSettingsPage from "./pages/ComputeSettingsPage";
import ModelParamsPage from "./pages/ModelParamsPage";
import TranslationSettingsPage from "./pages/TranslationSettingsPage";
import PromptsPage from "./pages/PromptsPage";
import OutputSettingsPage from "./pages/OutputSettingsPage";
import GeneralSettingsPage from "./pages/GeneralSettingsPage";
import AboutPage from "./pages/AboutPage";
import LegalPage from "./pages/LegalPage";
import LogsPage from "./pages/LogsPage";
import TasksPage from "./pages/TasksPage";
import {
  loadGeneralSettings,
  loadSettings,
  saveGeneralSettings,
  saveSettings,
  type GeneralSettings,
} from "./config";

interface E2EArgs {
  source?: string;
  output?: string;
  checkpoint_dir?: string;
  api_key?: string;
  dry_run?: boolean;
  ui_language?: string;
}

type ProgressPayload =
  | { Started: { total: number } }
  | { ChapterStarted: { index: number; href: string } }
  | { ChapterFinished: { index: number; href: string } }
  | { Failed: { index: number; href: string; error: string } }
  | "Completed";

const settingsPages: { page: Page; labelKey: string }[] = [
  { page: "settings-compute", labelKey: "settings_compute" },
  { page: "settings-model", labelKey: "settings_model" },
  { page: "settings-translation", labelKey: "settings_translation" },
  { page: "settings-prompts", labelKey: "settings_prompts" },
  { page: "settings-output", labelKey: "settings_output" },
  { page: "settings-general", labelKey: "settings_general" },
];

function generateId() {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

function initialGeneralFromLocalStorage(): GeneralSettings {
  const ui_language = localStorage.getItem("ui_language");
  const ui_theme = localStorage.getItem("ui_theme");
  return {
    ui_language:
      ui_language && targetLanguages.some((l) => l.code === ui_language) ? ui_language : "en",
    theme: ui_theme === "light" || ui_theme === "dark" ? ui_theme : "dark",
    follow_system_language: localStorage.getItem("follow_system_language") !== "false",
  };
}

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

function applyProgressToTask(task: Task, payload: ProgressPayload): Task {
  if (typeof payload === "string" && payload === "Completed") {
    return { ...task, progress_percent: 100 };
  }
  if (typeof payload === "object" && "Started" in payload) {
    return {
      ...task,
      progress_percent: 0,
      status: "running",
      chapter_total: payload.Started.total,
    };
  }
  if (typeof payload === "object" && "ChapterStarted" in payload) {
    const total = task.chapter_total ?? 0;
    const percent =
      total > 0
        ? Math.round(((payload.ChapterStarted.index + 1) / total) * 100)
        : task.progress_percent;
    return { ...task, progress_percent: Math.min(99, percent), status: "running" };
  }
  if (typeof payload === "object" && "ChapterFinished" in payload) {
    const total = task.chapter_total ?? 0;
    const percent =
      total > 0
        ? Math.round(((payload.ChapterFinished.index + 1) / total) * 100)
        : task.progress_percent;
    return { ...task, progress_percent: Math.min(99, percent), status: "running" };
  }
  if (typeof payload === "object" && "Failed" in payload) {
    return { ...task, message: payload.Failed.error };
  }
  return task;
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

function App() {
  const { t, i18n } = useTranslation();
  const [form, setForm] = useState<FormState>(() => ({
    ...defaults,
    target_lang: defaults.target_lang,
  }));
  const [page, setPage] = useState<Page>("translate");
  const [logs, setLogs] = useState<LogEntry[]>([]);

  const [progress, setProgress] = useState<ProgressState>({
    percent: 0,
    message: t("waiting"),
  });
  const [general, setGeneral] = useState<GeneralSettings>(initialGeneralFromLocalStorage);
  const [detectedLocale, setDetectedLocale] = useState<string>("en");
  const [queue, setQueue] = useState<QueueState>({
    tasks: [],
    running: false,
  });

  const completedRef = useRef(0);
  const totalRef = useRef(0);
  const e2eOutputRef = useRef<string | null>(null);

  // Load persisted settings on mount, optionally overridden by E2E env args.
  useEffect(() => {
    void (async () => {
      try {
        const [settings, generalSettings, e2e] = await Promise.all([
          loadSettings(),
          loadGeneralSettings(),
          invoke<E2EArgs>("get_e2e_args").catch((err) => {
            console.error("get_e2e_args failed:", err);
            return {} as E2EArgs;
          }),
        ]);
        setGeneral(generalSettings);

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

        // If E2E injects an API key, apply it to the active provider.
        if (e2e.api_key) {
          merged = {
            ...merged,
            providers: merged.providers.map((p) =>
              p.name === merged.active_provider ? { ...p, api_key: e2e.api_key ?? "" } : p
            ),
          };
        }

        // If E2E injects source/output/checkpoint paths, apply them to the form.
        if (e2e.source) {
          merged = { ...merged, source: e2e.source };
        }
        if (e2e.output) {
          merged = { ...merged, output: e2e.output };
          e2eOutputRef.current = e2e.output;
        }
        if (e2e.checkpoint_dir) {
          merged = { ...merged, checkpoint_dir: e2e.checkpoint_dir };
        }
        if (e2e.dry_run !== undefined) {
          merged = { ...merged, dry_run: e2e.dry_run };
        }

        // If no target language was saved, default to the UI language.
        if (!settings.target_lang) {
          merged = { ...merged, target_lang: generalSettings.ui_language ?? merged.target_lang };
        }

        setForm(merged);

        if (e2e.ui_language) {
          void i18n.changeLanguage(e2e.ui_language);
        } else if (!generalSettings.follow_system_language) {
          void i18n.changeLanguage(generalSettings.ui_language);
        }
      } catch (err) {
        console.error("[E2E] settings initialization failed:", err);
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
    void saveGeneralSettings(general);
  }, [general]);

  // System language detection.
  useEffect(() => {
    void invoke<string>("get_system_locale").then((locale) => {
      setDetectedLocale(locale);
      if (general.follow_system_language) {
        void i18n.changeLanguage(locale);
      }
    });
  }, [general.follow_system_language, i18n]);

  useEffect(() => {
    if (!general.follow_system_language) {
      void i18n.changeLanguage(general.ui_language);
    }
  }, [general.ui_language, general.follow_system_language, i18n]);

  useEffect(() => {
    void (async () => {
      const initial = await invoke<QueueState>("get_queue_state").catch(() => ({
        tasks: [],
        running: false,
      }));
      setQueue(initial);
    })();
  }, []);

  useEffect(() => {
    const unlistenProgress = listen<{ task_id: string; event: ProgressPayload }>(
      "task_progress",
      (event) => {
        const { task_id, event: progressEvent } = event.payload;
        setQueue((prev) => {
          const tasks = prev.tasks.map((t) => {
            if (t.id !== task_id) return t;
            return applyProgressToTask(t, progressEvent);
          });
          return { ...prev, tasks };
        });
      }
    );

    const unlistenChanged = listen<unknown>("queue_state_changed", () => {
      void (async () => {
        const state = await invoke<QueueState>("get_queue_state").catch(() => ({
          tasks: [],
          running: false,
        }));
        setQueue(state);
      })();
    });

    return () => {
      void unlistenProgress.then((f) => f());
      void unlistenChanged.then((f) => f());
    };
  }, []);

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

  useEffect(() => {
    const unlisten = listen<ProgressPayload>("translation_progress", (event) => {
      const payload = event.payload;

      setLogs((prev) => {
        if (typeof payload === "string" && payload === "Completed") {
          return [...prev, { id: generateId(), timestamp: Date.now(), kind: "success", message: t("completed") }];
        }
        if (typeof payload === "object" && "Started" in payload) {
          totalRef.current = payload.Started.total;
          completedRef.current = 0;
          return [
            ...prev,
            {
              id: generateId(), timestamp: Date.now(),
              kind: "info",
              message: t("log_started", { total: payload.Started.total }),
            },
          ];
        }
        if (typeof payload === "object" && "ChapterStarted" in payload) {
          return [
            ...prev,
            {
              id: generateId(), timestamp: Date.now(),
              kind: "chapter",
              message: t("log_chapter_started", { href: payload.ChapterStarted.href }),
            },
          ];
        }
        if (typeof payload === "object" && "ChapterFinished" in payload) {
          return [
            ...prev,
            {
              id: generateId(), timestamp: Date.now(),
              kind: "chapter",
              message: t("log_chapter_finished", {
                href: payload.ChapterFinished.href,
                current: completedRef.current,
                total: totalRef.current,
              }),
            },
          ];
        }
        if (typeof payload === "object" && "Failed" in payload) {
          return [
            ...prev,
            {
              id: generateId(), timestamp: Date.now(),
              kind: "error",
              message: t("log_chapter_failed", {
                href: payload.Failed.href,
                error: payload.Failed.error,
              }),
              details: payload.Failed.error,
            },
          ];
        }
        return prev;
      });

      setProgress((prev) => {
        if (typeof payload === "string" && payload === "Completed") {
          return { percent: 100, message: t("completed") };
        }
        if (typeof payload === "object" && "Started" in payload) {
          totalRef.current = payload.Started.total;
          completedRef.current = 0;
          return { percent: 0, message: t("started") };
        }
        if (typeof payload === "object" && "ChapterStarted" in payload) {
          return {
            ...prev,
            message: `${t("started")}: ${payload.ChapterStarted.href}`,
          };
        }
        if (typeof payload === "object" && "ChapterFinished" in payload) {
          completedRef.current += 1;
          const percent =
            totalRef.current > 0
              ? Math.min(
                  100,
                  Math.round((completedRef.current / totalRef.current) * 100)
                )
              : prev.percent;
          return {
            percent,
            message: `${t("completed")}: ${payload.ChapterFinished.href}`,
          };
        }
        if (typeof payload === "object" && "Failed" in payload) {
          return {
            ...prev,
            message: `${t("error")}: ${payload.Failed.error}`,
          };
        }
        return prev;
      });
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [t]);

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
      setPage("tasks");
    } catch (err) {
      const message = `${t("error")}: ${err}`;
      setLogs((prev) => [
        ...prev,
        { id: generateId(), timestamp: Date.now(), kind: "error", message },
      ]);
    }
  }

  const updateForm = <K extends keyof FormState>(key: K, value: FormState[K]) => {
    setForm((prev) => {
      const next = { ...prev, [key]: value } as FormState;
      if (key === "active_provider") {
        const active = next.providers.find((p) => p.name === value);
        const providerType = active?.provider ?? (value as string);
        const models = recommendedModels[providerType] ?? [];
        if (models.length > 0 && !models.some((m) => m.value === next.model)) {
          next.model = models[0].value;
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

  const clearLogs = () => setLogs([]);

  const refreshQueue = async () => {
    const state = await invoke<QueueState>("get_queue_state").catch(() => ({
      tasks: [],
      running: false,
    }));
    setQueue(state);
  };

  const enqueueTask = async (args: object) => {
    await invoke("enqueue_task", { args });
    await refreshQueue();
  };

  const removeTask = async (id: string) => {
    await invoke("remove_task", { id });
    await refreshQueue();
  };

  const retryTask = async (id: string) => {
    await invoke("retry_task", { id });
    await refreshQueue();
  };

  const cancelTask = async (id: string) => {
    await invoke("cancel_task", { id });
    await refreshQueue();
  };

  const startQueue = async () => {
    await invoke("start_queue");
    await refreshQueue();
  };

  const pauseQueue = async () => {
    await invoke("pause_queue");
    await refreshQueue();
  };

  // enqueueTask is reserved for the upcoming task-creation flow.
  void enqueueTask;

  const renderPage = () => {
    switch (page) {
      case "translate":
        return (
          <TranslatePage
            form={form}
            setForm={updateForm}
            onStart={handleStart}
            progress={progress}
            validation={validation}
            onPageChange={setPage}
          />
        );
      case "logs":
        return <LogsPage entries={logs} onClear={clearLogs} />;
      case "tasks":
        return (
          <TasksPage
            queue={queue}
            onRefresh={refreshQueue}
            onRemove={removeTask}
            onRetry={retryTask}
            onCancel={cancelTask}
            onStart={startQueue}
            onPause={pauseQueue}
          />
        );
      case "settings-compute":
        return (
          <ComputeSettingsPage
            providers={form.providers}
            activeProvider={form.active_provider}
            onChangeProviders={updateProviders}
            onChangeActiveProvider={(provider) => updateForm("active_provider", provider)}
          />
        );
      case "settings-model":
        return <ModelParamsPage form={form} setForm={updateForm} />;
      case "settings-translation":
        return <TranslationSettingsPage form={form} setForm={updateForm} />;
      case "settings-prompts":
        return <PromptsPage form={form} setForm={updateForm} />;
      case "settings-output":
        return <OutputSettingsPage form={form} setForm={updateForm} />;
      case "settings-general":
        return (
          <GeneralSettingsPage
            general={general}
            setGeneral={setGeneral}
            detectedLocale={detectedLocale}
          />
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
      <aside className="sidebar">
        <div className="brand">
          <h1>{t("app_title")}</h1>
        </div>

        <nav>
          <button
            type="button"
            className={`nav-item ${page === "translate" ? "active" : ""}`}
            onClick={() => setPage("translate")}
          >
            {t("nav_translate")}
          </button>

          <button
            type="button"
            className={`nav-item ${page === "logs" ? "active" : ""}`}
            onClick={() => setPage("logs")}
          >
            {t("nav_logs")}
          </button>

          <button
            type="button"
            className={`nav-item ${page === "tasks" ? "active" : ""}`}
            onClick={() => setPage("tasks")}
          >
            {t("nav_tasks")}
          </button>

          <div className="nav-group">
            <span className="nav-group-label">{t("nav_settings")}</span>
            {settingsPages.map(({ page: p, labelKey }) => (
              <button
                key={p}
                type="button"
                className={`nav-item ${page === p ? "active" : ""}`}
                onClick={() => setPage(p)}
              >
                {t(labelKey)}
              </button>
            ))}
          </div>

          <button
            type="button"
            className={`nav-item ${page === "about" ? "active" : ""}`}
            onClick={() => setPage("about")}
          >
            {t("nav_about")}
          </button>
        </nav>
      </aside>

      <main className="main-content">{renderPage()}</main>
    </div>
  );
}

export default App;
