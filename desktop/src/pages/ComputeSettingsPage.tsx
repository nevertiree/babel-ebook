import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { FormState } from "../types";
import { providerApiKeyHints, providers, recommendedModels } from "../types";

interface ComputeSettingsPageProps {
  form: FormState;
  setForm: <K extends keyof FormState>(key: K, value: FormState[K]) => void;
  onTestConnection: (status: "idle" | "ok" | "error") => void;
}

export default function ComputeSettingsPage({
  form,
  setForm,
  onTestConnection,
}: ComputeSettingsPageProps) {
  const { t } = useTranslation();
  const [showKey, setShowKey] = useState(false);
  const [modelCustom, setModelCustom] = useState(
    () => !recommendedModels[form.provider]?.some((m) => m.value === form.model)
  );
  const [testing, setTesting] = useState(false);
  const [testMessage, setTestMessage] = useState<string>("");

  const models = recommendedModels[form.provider] ?? [];
  const expectedPrefix = providerApiKeyHints[form.provider];
  const keyLooksWrong =
    expectedPrefix && form.api_key && !form.api_key.startsWith(expectedPrefix);

  const handleProviderChange = (provider: string) => {
    setForm("provider", provider);
    const providerModels = recommendedModels[provider] ?? [];
    if (providerModels.length > 0) {
      setForm("model", providerModels[0].value);
      setModelCustom(false);
    } else {
      setModelCustom(true);
    }
  };

  const handleModelChange = (value: string) => {
    if (value === "__custom__") {
      setModelCustom(true);
      setForm("model", "");
    } else {
      setModelCustom(false);
      setForm("model", value);
    }
  };

  const runTest = async () => {
    setTesting(true);
    setTestMessage(t("testing_connection"));
    onTestConnection("idle");
    try {
      await invoke("test_connection", {
        provider: form.provider,
        apiKey: form.api_key,
        baseUrl: form.base_url || null,
        model: form.model,
        temperature: form.temperature,
      });
      onTestConnection("ok");
      setTestMessage(t("connection_ok"));
    } catch (err) {
      onTestConnection("error");
      setTestMessage(`${t("connection_failed")}: ${err}`);
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="page settings-page">
      <h2>{t("settings_compute")}</h2>

      <div className="row">
        <label>
          {t("provider")}
          <select
            value={form.provider}
            onChange={(e) => handleProviderChange(e.target.value)}
          >
            {providers.map((p) => (
              <option key={p} value={p}>
                {t(`provider_${p}`)}
              </option>
            ))}
          </select>
        </label>

        <label>
          {t("model")}
          {!modelCustom ? (
            <select value={form.model} onChange={(e) => handleModelChange(e.target.value)}>
              {models.map((m) => (
                <option key={m.value} value={m.value}>
                  {m.label}
                </option>
              ))}
              <option value="__custom__">{t("model_custom")}</option>
            </select>
          ) : (
            <input
              type="text"
              value={form.model}
              onChange={(e) => setForm("model", e.target.value)}
              placeholder={t("model_custom_placeholder")}
            />
          )}
        </label>
      </div>

      {modelCustom && (
        <button
          type="button"
          className="text-button"
          onClick={() => handleModelChange(models[0]?.value ?? "")}
        >
          {t("use_recommended_model")}
        </button>
      )}

      <label>
        {t("base_url")}
        <input
          type="text"
          value={form.base_url}
          onChange={(e) => setForm("base_url", e.target.value)}
          placeholder={t("base_url_placeholder")}
        />
      </label>

      <label>
        {t("api_key")}
        <div className="input-with-toggle">
          <input
            type={showKey ? "text" : "password"}
            value={form.api_key}
            onChange={(e) => setForm("api_key", e.target.value)}
            placeholder="sk-..."
          />
          <button
            type="button"
            className="input-toggle"
            onClick={() => setShowKey((s) => !s)}
            title={showKey ? t("hide") : t("show")}
          >
            {showKey ? "🙈" : "👁"}
          </button>
        </div>
        {keyLooksWrong && <span className="format-hint">{t("api_key_format_hint")}</span>}
      </label>

      <label className="checkbox">
        <input
          type="checkbox"
          checked={form.remember_api_key}
          onChange={(e) => setForm("remember_api_key", e.target.checked)}
        />
        {t("remember_api_key")}
      </label>

      <h3>{t("model_parameters")}</h3>
      <div className="row">
        <label>
          <span className="field-row">
            {t("concurrency")}
            <span className="field-info" data-tooltip={t("concurrency_help")}>
              ⓘ
            </span>
          </span>
          <input
            type="number"
            min={1}
            max={10}
            value={form.concurrency}
            onChange={(e) => setForm("concurrency", Number(e.target.value))}
          />
        </label>

        <label>
          <span className="field-row">
            {t("max_input_tokens")}
            <span className="field-info" data-tooltip={t("max_input_tokens_help")}>
              ⓘ
            </span>
          </span>
          <input
            type="number"
            min={1}
            value={form.max_input_tokens}
            onChange={(e) => setForm("max_input_tokens", Number(e.target.value))}
          />
        </label>

        <label>
          <span className="field-row">
            {t("max_output_tokens")}
            <span className="field-info" data-tooltip={t("max_output_tokens_help")}>
              ⓘ
            </span>
          </span>
          <input
            type="number"
            min={1}
            value={form.max_output_tokens}
            onChange={(e) => setForm("max_output_tokens", Number(e.target.value))}
          />
        </label>
      </div>

      <div className="row">
        <label>
          <span className="field-row">
            {t("temperature")}
            <span className="field-info" data-tooltip={t("temperature_help")}>
              ⓘ
            </span>
          </span>
          <input
            type="number"
            step={0.1}
            min={0}
            max={2}
            value={form.temperature}
            onChange={(e) => setForm("temperature", Number(e.target.value))}
          />
        </label>
      </div>

      <div className="field-row">
        <button type="button" onClick={runTest} disabled={testing}>
          {testing ? t("testing_connection") : t("test_connection")}
        </button>
        {testMessage && <span className="hint">{testMessage}</span>}
      </div>
    </div>
  );
}
