import { useTranslation } from "react-i18next";
import { save, open, message, confirm } from "@tauri-apps/plugin-dialog";
import { languages, themes } from "../types";
import type { ThemeId } from "../types";
import { exportSettings, importSettings, type GeneralSettings, type ExportedSettings } from "../config";

interface GeneralSettingsPageProps {
  general: GeneralSettings;
  setGeneral: (value: GeneralSettings) => void;
  detectedLocale: string;
  onImport: (settings: ExportedSettings) => Promise<void>;
}

export default function GeneralSettingsPage({
  general,
  setGeneral,
  detectedLocale,
  onImport,
}: GeneralSettingsPageProps) {
  const { t, i18n } = useTranslation();

  const changeLanguage = (code: string) => {
    void i18n.changeLanguage(code);
    setGeneral({ ...general, ui_language: code });
  };

  const toggleFollowSystem = (checked: boolean) => {
    setGeneral({ ...general, follow_system_language: checked });
  };

  const setTheme = (theme: ThemeId) => {
    setGeneral({ ...general, theme });
  };

  const handleExport = async () => {
    const confirmed = await confirm(t("export_settings_warning"), {
      title: t("export_settings"),
      kind: "warning",
    });
    if (!confirmed) return;

    const today = new Date().toISOString().slice(0, 10);
    const path = await save({
      defaultPath: `babel-ebook-settings-${today}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;

    try {
      await exportSettings(path);
      await message(t("export_success"), { title: t("export_settings"), kind: "info" });
    } catch (err) {
      await message(t("error_export_failed", { message: String(err) }), {
        title: t("export_settings"),
        kind: "error",
      });
    }
  };

  const handleImport = async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path || Array.isArray(path)) return;

    let settings: ExportedSettings;
    try {
      settings = await importSettings(path);
    } catch (err) {
      const errStr = String(err);
      if (errStr.startsWith("version_mismatch:")) {
        const [, actual, expected] = errStr.split(":");
        await message(
          t("error_version_mismatch", { expected, actual }),
          { title: t("import_settings"), kind: "error" }
        );
      } else {
        await message(t("error_invalid_backup"), {
          title: t("import_settings"),
          kind: "error",
        });
      }
      return;
    }

    const confirmed = await confirm(t("import_settings_confirm"), {
      title: t("import_settings"),
      kind: "warning",
    });
    if (!confirmed) return;

    try {
      await onImport(settings);
      await message(t("import_success"), { title: t("import_settings"), kind: "info" });
    } catch (err) {
      await message(t("error_import_failed", { message: String(err) }), {
        title: t("import_settings"),
        kind: "error",
      });
    }
  };

  return (
    <div className="page settings-page">
      <h2>{t("settings_general")}</h2>

      <label>
        {t("language")}
        <select
          value={i18n.resolvedLanguage || "en"}
          onChange={(e) => changeLanguage(e.target.value)}
          disabled={general.follow_system_language}
        >
          {languages.map((lang) => (
            <option key={lang.code} value={lang.code}>
              {lang.label}
            </option>
          ))}
        </select>
      </label>

      <label className="checkbox">
        <input
          type="checkbox"
          checked={general.follow_system_language}
          onChange={(e) => toggleFollowSystem(e.target.checked)}
        />
        {t("follow_system_language")}
      </label>

      <p className="hint">
        {general.follow_system_language
          ? t("follow_system_active", { locale: detectedLocale })
          : t("language_restart_hint")}
      </p>

      <label>
        {t("theme")}
        <select
          value={general.theme}
          onChange={(e) => setTheme(e.target.value as ThemeId)}
        >
          {themes.map((theme) => (
            <option key={theme} value={theme}>
              {t(`theme_${theme.replace("-", "_")}`)}
            </option>
          ))}
        </select>
      </label>

      <div className="settings-section backup-restore-section">
        <h3>{t("settings_backup_restore")}</h3>
        <div className="backup-restore-actions">
          <button type="button" onClick={() => void handleExport()}>
            {t("export_settings")}
          </button>
          <button type="button" onClick={() => void handleImport()}>
            {t("import_settings")}
          </button>
        </div>
      </div>
    </div>
  );
}
