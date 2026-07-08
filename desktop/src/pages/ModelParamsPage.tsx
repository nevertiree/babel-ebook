import { useTranslation } from "react-i18next";
import type { FormState } from "../types";

interface ModelParamsPageProps {
  form: FormState;
  setForm: <K extends keyof FormState>(key: K, value: FormState[K]) => void;
}

export default function ModelParamsPage({ form, setForm }: ModelParamsPageProps) {
  const { t } = useTranslation();

  return (
    <div className="page settings-page">
      <h2>{t("settings_model")}</h2>

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
