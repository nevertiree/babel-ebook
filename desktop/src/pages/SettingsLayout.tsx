import { useTranslation } from "react-i18next";
import type { Page } from "../types";

interface SettingsLayoutProps {
  activePage: Page;
  onNavigate: (page: Page) => void;
  children: React.ReactNode;
}

const settingsTabs: { page: Page; labelKey: string }[] = [
  { page: "settings-compute", labelKey: "settings_compute" },
  { page: "settings-model", labelKey: "settings_model" },
  { page: "settings-translation", labelKey: "settings_translation" },
  { page: "settings-prompts", labelKey: "settings_prompts" },
  { page: "settings-output", labelKey: "settings_output" },
  { page: "settings-general", labelKey: "settings_general" },
];

export default function SettingsLayout({ activePage, onNavigate, children }: SettingsLayoutProps) {
  const { t } = useTranslation();

  return (
    <div className="page settings-layout">
      <h2>{t("nav_settings")}</h2>
      <nav className="settings-tabs" role="tablist" aria-label={t("nav_settings")}>
        {settingsTabs.map(({ page, labelKey }) => (
          <button
            key={page}
            type="button"
            role="tab"
            aria-selected={activePage === page}
            className={`settings-tab ${activePage === page ? "active" : ""}`}
            onClick={() => onNavigate(page)}
          >
            {t(labelKey)}
          </button>
        ))}
      </nav>
      <div className="settings-tab-panel" role="tabpanel">
        {children}
      </div>
    </div>
  );
}
