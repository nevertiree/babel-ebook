import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { confirm } from "@tauri-apps/plugin-dialog";
import "./App.css";
import type { FormState, LogEntry, Page, ProgressState, ProviderConfig, ValidationResult } from "./types";
import { defaults, recommendedModels, targetLanguages } from "./types";
import TranslatePage from "./pages/TranslatePage";
import ComputeSettingsPage from "./pages/ComputeSettingsPage";
import ModelParamsPage from "./pages/ModelParamsPage";
import TranslationSettingsPage from "./pages/TranslationSettingsPage";
import OutputSettingsPage from "./pages/OutputSettingsPage";
import GeneralSettingsPage from "./pages/GeneralSettingsPage";
import AboutPage from "./pages/AboutPage";
import LegalPage from "./pages/LegalPage";
import LogsPage from "./pages/LogsPage";
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
  return form.providers.find((p) => p.provider === form.active_provider);
}

function ensureProvider(form: FormState, providerName: string): FormState {
  if (form.providers.some((p) => p.provider === providerName)) {
    return form;
  }
  return {
    ...form,
    providers: [
      ...form.providers,
      {
        provider: providerName,
        api_key: "",
        base_url: "",
        use_custom_base_url: false,
      },
    ],
    active_provider: providerName,
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
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState<ProgressState>({
    percent: 0,
    message: t("waiting"),
  });
  const [general, setGeneral] = useState<GeneralSettings>(initialGeneralFromLocalStorage);
  const [detectedLocale, setDetectedLocale] = useState<string>("en");

  const completedRef = useRef(0);
  const totalRef = useRef(0);

  // Load persisted settings on mount, optionally overridden by E2E env args.
  useEffect(() => {
    void (async () => {
      const [settings, generalSettings, e2e] = await Promise.all([
        loadSettings(),
        loadGeneralSettings(),
        invoke<E2EArgs>("get_e2e_args").catch(() => ({}) as E2EArgs),
      ]);
      setGeneral(generalSettings);

      let merged = { ...form, ...settings } as FormState;

      // If no providers exist (fresh install), seed a default provider config so
      // the UI is never in a broken state. E2E can override the API key below.
      if (!merged.providers || merged.providers.length === 0) {
        const defaultProvider = "deepseek";
        merged = ensureProvider(merged, defaultProvider);
      }
      if (!merged.active_provider || !merged.providers.some((p) => p.provider === merged.active_provider)) {
        merged = { ...merged, active_provider: merged.providers[0].provider };
      }

      // If E2E injects an API key, apply it to the active provider.
      if (e2e.api_key) {
        merged = {
          ...merged,
          providers: merged.providers.map((p) =>
            p.provider === merged.active_provider ? { ...p, api_key: e2e.api_key ?? "" } : p
          ),
        };
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

  // Force re-render on window resize so flex/grid layouts recalculate.
  const [, setWindowSize] = useState({ width: window.innerWidth, height: window.innerHeight });
  useEffect(() => {
    const handleResize = () => {
      setWindowSize({ width: window.innerWidth, height: window.innerHeight });
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

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
          completedRef.current += 1;
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
              ? Math.round((completedRef.current / totalRef.current) * 100)
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

  const parseCommaList = (value: string) =>
    value
      .split(",")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);

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

    setLoading(true);
    setProgress({ percent: 0, message: t("started") });
    setLogs((prev) => [...prev, { id: generateId(), timestamp: Date.now(), kind: "info", message: t("started") }]);

    const baseUrl = provider.use_custom_base_url ? provider.base_url : "";
    const args = {
      ...form,
      provider: provider.provider,
      api_key: provider.api_key,
      base_url: baseUrl || null,
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
    };

    try {
      const result = await invoke<string>("translate_epub", { args });
      if (form.dry_run && result.toLowerCase().includes("estimated source tokens")) {
        setProgress({ percent: 100, message: result });
      }
      setLogs((prev) => [
        ...prev,
        { id: generateId(), timestamp: Date.now(), kind: "success", message: `${t("completed")}: ${result}` },
      ]);
    } catch (err) {
      const message = `${t("error")}: ${err}`;
      setProgress({ percent: 0, message });
      setLogs((prev) => [
        ...prev,
        { id: generateId(), timestamp: Date.now(), kind: "error", message },
      ]);
    } finally {
      setLoading(false);
    }
  }

  const updateForm = <K extends keyof FormState>(key: K, value: FormState[K]) => {
    setForm((prev) => {
      const next = { ...prev, [key]: value } as FormState;
      if (key === "active_provider") {
        const providerName = value as string;
        const models = recommendedModels[providerName] ?? [];
        if (models.length > 0 && !models.some((m) => m.value === next.model)) {
          next.model = models[0].value;
        }
      }
      return next;
    });
  };

  const updateProviders = (providers: ProviderConfig[]) => {
    setForm((prev) => {
      const activeStillExists = providers.some((p) => p.provider === prev.active_provider);
      return {
        ...prev,
        providers,
        active_provider: activeStillExists ? prev.active_provider : providers[0]?.provider ?? "",
      };
    });
  };

  const clearLogs = () => setLogs([]);

  const renderPage = () => {
    switch (page) {
      case "translate":
        return (
          <TranslatePage
            form={form}
            setForm={updateForm}
            onStart={handleStart}
            loading={loading}
            progress={progress}
            validation={validation}
            onPageChange={setPage}
          />
        );
      case "logs":
        return <LogsPage entries={logs} onClear={clearLogs} />;
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
          <p>{t("subtitle")}</p>
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
