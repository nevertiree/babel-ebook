import { useTranslation } from "react-i18next";
import type { ProviderConfig } from "../types";
import { providerApiKeyHints, providers as knownProviders } from "../types";
import LoadingSpinner from "./LoadingSpinner";

interface ProviderCardProps {
  provider: ProviderConfig;
  isActive: boolean;
  showKey: boolean;
  testing: boolean;
  testResult?: { ok: boolean; message: string };
  onChange: (patch: Partial<ProviderConfig>) => void;
  onRemove: () => void;
  onSetActive: () => void;
  onTest: () => void;
  onToggleShowKey: () => void;
}

function EyeIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

function EyeOffIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" />
      <line x1="1" y1="1" x2="23" y2="23" />
    </svg>
  );
}

export default function ProviderCard({
  provider,
  isActive,
  showKey,
  testing,
  testResult,
  onChange,
  onRemove,
  onSetActive,
  onTest,
  onToggleShowKey,
}: ProviderCardProps) {
  const { t } = useTranslation();

  const expectedPrefix = providerApiKeyHints[provider.provider];
  const keyLooksWrong =
    expectedPrefix && provider.api_key && !provider.api_key.startsWith(expectedPrefix);

  const handleProviderTypeChange = (newProviderType: string) => {
    if (newProviderType === provider.provider) return;
    onChange({
      provider: newProviderType,
      base_url: "",
      use_custom_base_url: false,
    });
  };

  const handleUseCustomBaseUrlChange = (checked: boolean) => {
    onChange({
      use_custom_base_url: checked,
      base_url: checked ? provider.base_url : "",
    });
  };

  return (
    <div className={`provider-card ${isActive ? "active" : ""}`}>
      <div className="provider-card-row provider-card-meta">
        <label className="provider-type-label">
          <span>{t("provider_type")}</span>
          <select
            value={provider.provider}
            onChange={(e) => handleProviderTypeChange(e.target.value)}
          >
            {knownProviders.map((kp) => (
              <option key={kp} value={kp}>
                {t(`provider_${kp}`)}
              </option>
            ))}
          </select>
        </label>

        <label className="provider-name-label" title={provider.name}>
          <span>{t("provider_name")}</span>
          <input
            type="text"
            value={provider.name}
            onChange={(e) => onChange({ name: e.target.value })}
          />
        </label>
      </div>

      <div className="provider-card-credentials">
        <div className="provider-card-row provider-card-credentials-row">
          <label className="provider-api-key" title={provider.api_key}>
            <span>{t("api_key")}</span>
            <div className="input-with-toggle">
              <input
                type={showKey ? "text" : "password"}
                value={provider.api_key}
                onChange={(e) => onChange({ api_key: e.target.value })}
                placeholder="sk-..."
              />
              <button
                type="button"
                className="input-toggle"
                onClick={onToggleShowKey}
                title={showKey ? t("hide") : t("show")}
                aria-label={showKey ? t("hide") : t("show")}
              >
                {showKey ? <EyeOffIcon /> : <EyeIcon />}
              </button>
            </div>
          </label>

          <label className="provider-base-url-toggle checkbox">
            <input
              type="checkbox"
              checked={provider.use_custom_base_url}
              onChange={(e) => handleUseCustomBaseUrlChange(e.target.checked)}
            />
            {t("use_custom_base_url")}
          </label>
        </div>

        {provider.use_custom_base_url && (
          <div className="provider-card-row provider-card-credentials-row">
            <label className="provider-base-url">
              <span>{t("base_url")}</span>
              <input
                type="text"
                value={provider.base_url}
                onChange={(e) => onChange({ base_url: e.target.value })}
                placeholder={t("base_url_placeholder")}
              />
            </label>
          </div>
        )}
      </div>

      <div className="provider-card-row provider-card-actions">
        <button
          type="button"
          className="test-button"
          onClick={onTest}
          disabled={testing}
          aria-busy={testing}
        >
          {testing && <LoadingSpinner size={14} />}
          {testing ? t("testing_connection") : t("test_connection")}
        </button>

        <button
          type="button"
          className={`text-button ${isActive ? "active-provider-label" : ""}`}
          onClick={onSetActive}
          disabled={isActive}
          aria-pressed={isActive}
        >
          {isActive ? t("active_provider") : t("set_active_provider")}
        </button>

        <button type="button" className="text-button danger" onClick={onRemove}>
          {t("remove_provider")}
        </button>
      </div>

      <div className="provider-card-hints">
        {!provider.use_custom_base_url && (
          <span className="hint default-base-url-hint">{t("default_base_url")}</span>
        )}

        {keyLooksWrong && (
          <span className="format-hint">{t("api_key_format_hint")}</span>
        )}

        {testResult && (
          <span
            className={`test-result ${
              testResult.ok ? "test-result-ok" : "test-result-error"
            }`}
          >
            {testResult.message}
          </span>
        )}
      </div>
    </div>
  );
}
