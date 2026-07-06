import { useTranslation } from "react-i18next";
import { languages } from "../types";
import type { GeneralSettings } from "../config";

interface GeneralSettingsPageProps {
  general: GeneralSettings;
  setGeneral: (value: GeneralSettings) => void;
  detectedLocale: string;
}

export default function GeneralSettingsPage({
  general,
  setGeneral,
  detectedLocale,
}: GeneralSettingsPageProps) {
  const { t, i18n } = useTranslation();

  const changeLanguage = (code: string) => {
    void i18n.changeLanguage(code);
    setGeneral({ ...general, ui_language: code });
  };

  const toggleFollowSystem = (checked: boolean) => {
    setGeneral({ ...general, follow_system_language: checked });
  };

  const setTheme = (theme: "light" | "dark") => {
    setGeneral({ ...general, theme });
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
          onChange={(e) => setTheme(e.target.value as "light" | "dark")}
        >
          <option value="light">{t("theme_light")}</option>
          <option value="dark">{t("theme_dark")}</option>
        </select>
      </label>
    </div>
  );
}
