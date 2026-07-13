import { useTranslation } from "react-i18next";
import type { Page } from "../types";
import SettingsTabIcon, { type SettingsTabIconProps } from "../components/SettingsTabIcon";
import "./SettingsPage.css";

interface SettingsLayoutProps {
  activePage: Page;
  onNavigate: (page: Page) => void;
  children: React.ReactNode;
}

const settingsTabs: { page: Page; labelKey: string; icon: SettingsTabIconProps["icon"] }[] = [
  { page: "settings-compute", labelKey: "settings_compute", icon: "compute" },
  { page: "settings-model", labelKey: "settings_model", icon: "model" },
  { page: "settings-translation", labelKey: "settings_translation", icon: "translation" },
  { page: "settings-ocr", labelKey: "settings_ocr", icon: "ocr" },
  { page: "settings-prompts", labelKey: "settings_prompts", icon: "prompts" },
  { page: "settings-output", labelKey: "settings_output", icon: "output" },
  { page: "settings-queue", labelKey: "settings_queue", icon: "queue" },
  { page: "settings-general", labelKey: "settings_general", icon: "general" },
];

export default function SettingsLayout({ activePage, onNavigate, children }: SettingsLayoutProps) {
  const { t } = useTranslation();

  return (
    <div className="page settings-layout">
      <div className="settings-header">
        <h2>{t("nav_settings")}</h2>
        <nav className="settings-tabs" role="tablist" aria-label={t("nav_settings")}>
          {settingsTabs.map(({ page, labelKey, icon }) => (
            <button
              key={page}
              type="button"
              role="tab"
              aria-selected={activePage === page}
              className={`settings-tab ${activePage === page ? "active" : ""}`}
              data-testid={`settings-tab-${page.replace("settings-", "")}`}
              onClick={() => onNavigate(page)}
            >
              <SettingsTabIcon icon={icon} className="settings-tab-icon" />
              {t(labelKey)}
            </button>
          ))}
        </nav>
      </div>
      <div className="settings-tab-panel" role="tabpanel">
        {children}
      </div>
    </div>
  );
}
