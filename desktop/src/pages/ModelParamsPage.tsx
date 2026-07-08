import { useTranslation } from "react-i18next";
import type { FormState } from "../types";
import { recommendedModels } from "../types";

interface ModelParamsPageProps {
  form: FormState;
  setForm: <K extends keyof FormState>(key: K, value: FormState[K]) => void;
}

export default function ModelParamsPage({ form, setForm }: ModelParamsPageProps) {
  const { t } = useTranslation();

  const activeProvider = form.providers.find((p) => p.provider === form.active_provider);
  const models = activeProvider ? recommendedModels[activeProvider.provider] ?? [] : [];
  const modelIsCustom = !models.some((m) => m.value === form.model);

  const handleModelChange = (value: string) => {
    if (value === "__custom__") {
      setForm("model", "");
    } else {
      setForm("model", value);
    }
  };

  return (
    <div className="page settings-page">
      <h2>{t("settings_model")}</h2>

      <label>
        {t("model")}
        {models.length > 0 && !modelIsCustom ? (
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

      {modelIsCustom && models.length > 0 && (
        <button
          type="button"
          className="text-button"
          onClick={() => handleModelChange(models[0]?.value ?? "")}
        >
          {t("use_recommended_model")}
        </button>
      )}

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
    </div>
  );
}
