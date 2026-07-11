import { useTranslation } from "react-i18next";
import type { Page, ProviderConfig, ValidationResult } from "../types";

interface ValidationBannerProps {
  validation: ValidationResult;
  providers: ProviderConfig[];
  activeProvider?: ProviderConfig;
  onNavigate: (page: Page) => void;
}

export default function ValidationBanner({
  validation,
  providers,
  activeProvider,
  onNavigate,
}: ValidationBannerProps) {
  const { t } = useTranslation();

  if (validation.valid) {
    return null;
  }

  const missingProvider = providers.length === 0 || !activeProvider;
  const missingApiKey =
    !missingProvider &&
    activeProvider.provider !== "ollama" &&
    !activeProvider.api_key;

  return (
    <div className="validation-banner" role="alert" aria-live="polite">
      {missingProvider && (
        <div
          className="validation-banner-prominent"
          style={{
            padding: "0.75rem 1rem",
            marginBottom: "1rem",
            borderRadius: "0.5rem",
            border: "1px solid var(--warning, #f59e0b)",
            background: "rgba(245, 158, 11, 0.12)",
          }}
        >
          <p style={{ margin: "0 0 0.5rem" }}>{t("validation_no_provider")}</p>
          <button
            type="button"
            className="button-primary"
            onClick={() => onNavigate("settings-compute")}
          >
            {t("validation_configure_provider")}
          </button>
        </div>
      )}

      {missingApiKey && (
        <div
          className="validation-banner-prominent"
          style={{
            padding: "0.75rem 1rem",
            marginBottom: "1rem",
            borderRadius: "0.5rem",
            border: "1px solid var(--warning, #f59e0b)",
            background: "rgba(245, 158, 11, 0.12)",
          }}
        >
          <p style={{ margin: "0 0 0.5rem" }}>
            {t("validation_missing_api_key")}
          </p>
          <button
            type="button"
            className="button-primary"
            onClick={() => onNavigate("settings-compute")}
          >
            {t("validation_configure_provider")}
          </button>
        </div>
      )}

      {validation.errors.source && (
        <p className="validation-message inline-error">
          {t("validation_missing_source")}
        </p>
      )}

      {validation.errors.output && (
        <p className="validation-message inline-error">
          {t("validation_missing_output")}
        </p>
      )}

      {validation.reason && !missingProvider && !missingApiKey && (
        <p className="validation-reason inline-error">{validation.reason}</p>
      )}
    </div>
  );
}
