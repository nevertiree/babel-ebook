import { useTranslation } from "react-i18next";
import type { Page } from "../types";

interface SettingsLayoutProps {
  activePage: Page;
  onNavigate: (page: Page) => void;
  children: React.ReactNode;
}

const settingsTabs: { page: Page; labelKey: string; icon: string }[] = [
  { page: "settings-compute", labelKey: "settings_compute", icon: "⚙️" },
  { page: "settings-model", labelKey: "settings_model", icon: "🧠" },
  { page: "settings-translation", labelKey: "settings_translation", icon: "🌐" },
  { page: "settings-prompts", labelKey: "settings_prompts", icon: "📝" },
  { page: "settings-output", labelKey: "settings_output", icon: "📁" },
  { page: "settings-general", labelKey: "settings_general", icon: "🎛️" },
];

export default function SettingsLayout({ activePage, onNavigate, children }: SettingsLayoutProps) {
  const { t } = useTranslation();

  return (
    <div className="page settings-layout">
      <h2>{t("nav_settings")}</h2>
      <nav className="settings-tabs" role="tablist" aria-label={t("nav_settings")}>
        {settingsTabs.map(({ page, labelKey, icon }) => (
          <button
            key={page}
            type="button"
            role="tab"
            aria-selected={activePage === page}
            className={`settings-tab ${activePage === page ? "active" : ""}`}
            onClick={() => onNavigate(page)}
          >
            <span className="settings-tab-icon" aria-hidden="true">{icon}</span>
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
