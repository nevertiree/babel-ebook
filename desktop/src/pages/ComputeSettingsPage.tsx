import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { ProviderConfig } from "../types";
import { providerApiKeyHints, providers as knownProviders } from "../types";

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

  const usedNames = new Set(providers.map((p) => p.name));

  const updateProvider = (name: string, patch: Partial<ProviderConfig>) => {
    onChangeProviders(
      providers.map((p) => (p.name === name ? { ...p, ...patch } : p))
    );
  };

  const makeUniqueName = (base: string): string => {
    let candidate = base;
    let index = 1;
    while (usedNames.has(candidate)) {
      index += 1;
      candidate = `${base} ${index}`;
    }
    return candidate;
  };

  const addProvider = () => {
    const configured = new Set(providers.map((p) => p.provider));
    const remaining = knownProviders.find((p) => !configured.has(p));
    if (!remaining) return;

    const name = makeUniqueName(remaining);
    const newProvider: ProviderConfig = {
      name,
      provider: remaining,
      api_key: "",
      base_url: "",
      use_custom_base_url: false,
    };
    onChangeProviders([...providers, newProvider]);
    onChangeActiveProvider(name);
  };

  const removeProvider = (name: string) => {
    const next = providers.filter((p) => p.name !== name);
    onChangeProviders(next);
    if (activeProvider === name && next.length > 0) {
      onChangeActiveProvider(next[0].name);
    }
    setTestResults((prev) => {
      const copy = { ...prev };
      delete copy[name];
      return copy;
    });
  };

  const handleProviderTypeChange = (name: string, newProviderType: string) => {
    const p = providers.find((x) => x.name === name);
    if (!p || p.provider === newProviderType) return;

    onChangeProviders(
      providers.map((x) =>
        x.name === name
          ? {
              ...x,
              provider: newProviderType,
              base_url: "",
              use_custom_base_url: false,
            }
          : x
      )
    );
  };

  const runTest = async (name: string) => {
    const p = providers.find((x) => x.name === name);
    if (!p) return;

    setTestingFor(name);
    setTestResults((prev) => ({
      ...prev,
      [name]: { ok: false, message: t("testing_connection") },
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
        [name]: { ok: true, message: t("connection_ok") },
      }));
    } catch (err) {
      setTestResults((prev) => ({
        ...prev,
        [name]: { ok: false, message: `${t("connection_failed")}: ${err}` },
      }));
    } finally {
      setTestingFor(null);
    }
  };

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
        const isActive = activeProvider === p.name;
        const expectedPrefix = providerApiKeyHints[p.provider];
        const keyLooksWrong =
          expectedPrefix && p.api_key && !p.api_key.startsWith(expectedPrefix);
        const result = testResults[p.name];

        return (
          <div
            key={p.name}
            className={`provider-config ${isActive ? "active" : ""}`}
          >
            <div className="provider-row">
              <label className="provider-type-label">
                <span>{t("provider_type")}</span>
                <select
                  value={p.provider}
                  onChange={(e) => handleProviderTypeChange(p.name, e.target.value)}
                >
                  {knownProviders.map((kp) => (
                    <option key={kp} value={kp}>
                      {t(`provider_${kp}`)}
                    </option>
                  ))}
                </select>
              </label>

              <div className="provider-api-key">
                <label>
                  <span>{t("api_key")}</span>
                  <div className="input-with-toggle">
                    <input
                      type={showKeyFor === p.name ? "text" : "password"}
                      value={p.api_key}
                      onChange={(e) => updateProvider(p.name, { api_key: e.target.value })}
                      placeholder="sk-..."
                    />
                    <button
                      type="button"
                      className="input-toggle"
                      onClick={() =>
                        setShowKeyFor((prev) => (prev === p.name ? null : p.name))
                      }
                      title={showKeyFor === p.name ? t("hide") : t("show")}
                    >
                      {showKeyFor === p.name ? "🙈" : "👁"}
                    </button>
                  </div>
                </label>
                {keyLooksWrong && <span className="format-hint">{t("api_key_format_hint")}</span>}
              </div>

              <button
                type="button"
                onClick={() => runTest(p.name)}
                disabled={testingFor === p.name}
              >
                {testingFor === p.name ? t("testing_connection") : t("test_connection")}
              </button>
            </div>

            {result && (
              <span
                className={`test-result ${result.ok ? "test-result-ok" : "test-result-error"}`}
              >
                {result.message}
              </span>
            )}

            <div className="provider-row provider-meta-row">
              <label>
                <span>{t("provider_name")}</span>
                <input
                  type="text"
                  value={p.name}
                  onChange={(e) => updateProvider(p.name, { name: e.target.value })}
                />
              </label>

              <label className="checkbox">
                <input
                  type="checkbox"
                  checked={p.use_custom_base_url}
                  onChange={(e) =>
                    updateProvider(p.name, {
                      use_custom_base_url: e.target.checked,
                      base_url: e.target.checked ? p.base_url : "",
                    })
                  }
                />
                {t("use_custom_base_url")}
              </label>

              {p.use_custom_base_url && (
                <label>
                  <span>{t("base_url")}</span>
                  <input
                    type="text"
                    value={p.base_url}
                    onChange={(e) => updateProvider(p.name, { base_url: e.target.value })}
                    placeholder={t("base_url_placeholder")}
                  />
                </label>
              )}

              {!p.use_custom_base_url && (
                <span className="hint">{t("default_base_url")}</span>
              )}

              <button
                type="button"
                className="text-button danger"
                onClick={() => removeProvider(p.name)}
              >
                {t("remove_provider")}
              </button>

              <button
                type="button"
                className={`text-button ${isActive ? "active-provider-label" : ""}`}
                onClick={() => onChangeActiveProvider(p.name)}
                disabled={isActive}
              >
                {isActive ? t("active_provider") : t("set_active_provider")}
              </button>
            </div>
          </div>
        );
      })}

      {canAddProvider && (
        <button type="button" onClick={addProvider}>
          {t("add_provider")}
        </button>
      )}

      <div className="hint">{t("compute_settings_hint")}</div>
    </div>
  );
}
