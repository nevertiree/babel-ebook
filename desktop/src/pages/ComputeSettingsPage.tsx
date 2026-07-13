import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import type { ProviderConfig } from "../types";
import { providers as knownProviders } from "../types";
import ProviderCard from "../components/ProviderCard";
import EmptyStateIcon from "../components/EmptyStateIcon";

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
  const [testResults, setTestResults] = useState<
    Record<string, { ok: boolean; message: string }>
  >({});

  const usedNames = new Set(providers.map((p) => p.name));

  const updateProvider = (name: string, patch: Partial<ProviderConfig>) => {
    onChangeProviders(providers.map((p) => (p.name === name ? { ...p, ...patch } : p)));
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

  const removeProvider = async (name: string) => {
    const confirmed = await confirm(
      t("confirm_remove_provider", { name }),
      { title: t("confirm_remove_provider_title"), kind: "warning" }
    );
    if (!confirmed) return;
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
    <div className="page settings-page compute-settings-page">
      <h2>{t("settings_compute")}</h2>

      {providers.length === 0 && (
        <div className="empty-state">
          <EmptyStateIcon variant="provider" className="empty-state-icon" />
          <p>{t("no_provider_configured")}</p>
          <button type="button" onClick={addProvider}>
            {t("add_provider")}
          </button>
        </div>
      )}

      {providers.map((p, index) => (
        <ProviderCard
          key={index}
          provider={p}
          isActive={activeProvider === p.name}
          showKey={showKeyFor === p.name}
          testing={testingFor === p.name}
          testResult={testResults[p.name]}
          onChange={(patch) => updateProvider(p.name, patch)}
          onRemove={() => removeProvider(p.name)}
          onSetActive={() => onChangeActiveProvider(p.name)}
          onTest={() => runTest(p.name)}
          onToggleShowKey={() =>
            setShowKeyFor((prev) => (prev === p.name ? null : p.name))
          }
        />
      ))}

      {canAddProvider && (
        <button type="button" onClick={addProvider}>
          {t("add_provider")}
        </button>
      )}

      <div className="hint">{t("compute_settings_hint")}</div>
    </div>
  );
}
