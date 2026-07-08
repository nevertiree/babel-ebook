import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { ProviderConfig } from "../types";
import { providerApiKeyHints, providerDefaultBaseUrl, providers as knownProviders } from "../types";

interface ComputeSettingsPageProps {
  providers: ProviderConfig[];
  activeProvider: string;
  onChangeProviders: (providers: ProviderConfig[]) => void;
  onChangeActiveProvider: (provider: string) => void;
}

export default function ComputeSettingsPage({
  providers,
  activeProvider,
  onChangeProviders,
  onChangeActiveProvider,
}: ComputeSettingsPageProps) {
  const { t } = useTranslation();
  const [showKeyFor, setShowKeyFor] = useState<string | null>(null);
  const [testingFor, setTestingFor] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, { ok: boolean; message: string }>>({});

  const updateProvider = (providerName: string, patch: Partial<ProviderConfig>) => {
    onChangeProviders(
      providers.map((p) => (p.provider === providerName ? { ...p, ...patch } : p))
    );
  };

  const addProvider = () => {
    const configured = new Set(providers.map((p) => p.provider));
    const remaining = knownProviders.find((p) => !configured.has(p));
    if (!remaining) return;

    const newProvider: ProviderConfig = {
      provider: remaining,
      api_key: "",
      base_url: "",
      use_custom_base_url: false,
    };
    onChangeProviders([...providers, newProvider]);
    onChangeActiveProvider(remaining);
  };

  const removeProvider = (providerName: string) => {
    const next = providers.filter((p) => p.provider !== providerName);
    onChangeProviders(next);
    if (activeProvider === providerName && next.length > 0) {
      onChangeActiveProvider(next[0].provider);
    }
    setTestResults((prev) => {
      const copy = { ...prev };
      delete copy[providerName];
      return copy;
    });
  };

  const handleProviderTypeChange = (oldName: string, newName: string) => {
    if (oldName === newName) return;
    if (providers.some((p) => p.provider === newName)) return;

    onChangeProviders(
      providers.map((p) =>
        p.provider === oldName
          ? {
              ...p,
              provider: newName,
              base_url: "",
              use_custom_base_url: false,
            }
          : p
      )
    );
    if (activeProvider === oldName) {
      onChangeActiveProvider(newName);
    }
  };

  const runTest = async (providerName: string) => {
    const p = providers.find((x) => x.provider === providerName);
    if (!p) return;

    setTestingFor(providerName);
    setTestResults((prev) => ({
      ...prev,
      [providerName]: { ok: false, message: t("testing_connection") },
    }));

    try {
      await invoke("test_connection", {
        args: {
          provider: p.provider,
          api_key: p.api_key,
          base_url: p.use_custom_base_url ? p.base_url || null : null,
        },
      });
      setTestResults((prev) => ({
        ...prev,
        [providerName]: { ok: true, message: t("connection_ok") },
      }));
    } catch (err) {
      setTestResults((prev) => ({
        ...prev,
        [providerName]: { ok: false, message: `${t("connection_failed")}: ${err}` },
      }));
    } finally {
      setTestingFor(null);
    }
  };

  const configuredProviders = new Set(providers.map((p) => p.provider));
  const canAddProvider = providers.length < knownProviders.length;

  return (
    <div className="page settings-page">
      <h2>{t("settings_compute")}</h2>

      {providers.length === 0 && (
        <div className="empty-state">
          <p>{t("no_provider_configured")}</p>
        </div>
      )}

      {providers.map((p) => {
        const isActive = activeProvider === p.provider;
        const expectedPrefix = providerApiKeyHints[p.provider];
        const keyLooksWrong =
          expectedPrefix && p.api_key && !p.api_key.startsWith(expectedPrefix);
        const result = testResults[p.provider];

        return (
          <div
            key={p.provider}
            className={`provider-config ${isActive ? "active" : ""}`}
          >
            <div className="provider-config-header">
              <select
                value={p.provider}
                onChange={(e) => handleProviderTypeChange(p.provider, e.target.value)}
              >
                {knownProviders.map((kp) => (
                  <option
                    key={kp}
                    value={kp}
                    disabled={kp !== p.provider && configuredProviders.has(kp)}
                  >
                    {t(`provider_${kp}`)}
                  </option>
                ))}
              </select>

              <div className="provider-config-actions">
                <button
                  type="button"
                  className={`text-button ${isActive ? "active-provider-label" : ""}`}
                  onClick={() => onChangeActiveProvider(p.provider)}
                  disabled={isActive}
                >
                  {isActive ? t("active_provider") : t("set_active_provider")}
                </button>
                <button
                  type="button"
                  className="text-button danger"
                  onClick={() => removeProvider(p.provider)}
                >
                  {t("remove_provider")}
                </button>
              </div>
            </div>

            <label>
              {t("api_key")}
              <div className="input-with-toggle">
                <input
                  type={showKeyFor === p.provider ? "text" : "password"}
                  value={p.api_key}
                  onChange={(e) => updateProvider(p.provider, { api_key: e.target.value })}
                  placeholder="sk-..."
                />
                <button
                  type="button"
                  className="input-toggle"
                  onClick={() =>
                    setShowKeyFor((prev) => (prev === p.provider ? null : p.provider))
                  }
                  title={showKeyFor === p.provider ? t("hide") : t("show")}
                >
                  {showKeyFor === p.provider ? "🙈" : "👁"}
                </button>
              </div>
              {keyLooksWrong && <span className="format-hint">{t("api_key_format_hint")}</span>}
            </label>

            <label className="checkbox">
              <input
                type="checkbox"
                checked={p.use_custom_base_url}
                onChange={(e) =>
                  updateProvider(p.provider, {
                    use_custom_base_url: e.target.checked,
                    base_url: e.target.checked ? p.base_url : "",
                  })
                }
              />
              {t("use_custom_base_url")}
            </label>

            {p.use_custom_base_url && (
              <label>
                {t("base_url")}
                <input
                  type="text"
                  value={p.base_url}
                  onChange={(e) => updateProvider(p.provider, { base_url: e.target.value })}
                  placeholder={t("base_url_placeholder")}
                />
              </label>
            )}

            {!p.use_custom_base_url && (
              <div className="hint">
                {t("default_base_url")}: {providerDefaultBaseUrl(p.provider)}
              </div>
            )}

            <div className="field-row">
              <button
                type="button"
                onClick={() => runTest(p.provider)}
                disabled={testingFor === p.provider}
              >
                {testingFor === p.provider ? t("testing_connection") : t("test_connection")}
              </button>
              {result && (
                <span
                  className={`test-result ${result.ok ? "test-result-ok" : "test-result-error"}`}
                >
                  {result.message}
                </span>
              )}
            </div>
          </div>
        );
      })}

      {canAddProvider && (
        <button type="button" onClick={addProvider}>
          {t("add_provider")}
        </button>
      )}

      <div className="hint">
        {t("compute_settings_hint")}
      </div>
    </div>
  );
}
