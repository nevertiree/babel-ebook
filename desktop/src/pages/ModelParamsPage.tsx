import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { FormState } from "../types";
import Tooltip from "../components/Tooltip";

interface ModelParamsPageProps {
  form: FormState;
  setForm: <K extends keyof FormState>(key: K, value: FormState[K]) => void;
}

interface FieldMeta {
  key: "concurrency" | "max_input_tokens" | "max_output_tokens" | "temperature";
  labelKey: string;
  helpKey: string;
  min: number;
  max?: number;
  step?: string;
}

const FIELDS: FieldMeta[] = [
  { key: "concurrency", labelKey: "concurrency", helpKey: "concurrency_help", min: 1, max: 10 },
  { key: "max_input_tokens", labelKey: "max_input_tokens", helpKey: "max_input_tokens_help", min: 1 },
  { key: "max_output_tokens", labelKey: "max_output_tokens", helpKey: "max_output_tokens_help", min: 1 },
  { key: "temperature", labelKey: "temperature", helpKey: "temperature_help", min: 0, max: 2, step: "0.1" },
];

function clamp(value: number, min: number, max?: number): number {
  let v = value;
  if (Number.isNaN(v)) return min;
  v = Math.max(min, v);
  if (max !== undefined) v = Math.min(max, v);
  return v;
}

export default function ModelParamsPage({ form, setForm }: ModelParamsPageProps) {
  const { t } = useTranslation();
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validate = (_key: string, raw: number, meta: FieldMeta): string | undefined => {
    if (Number.isNaN(raw)) return t("error_number_required");
    if (raw < meta.min) return t("error_number_min", { min: meta.min });
    if (meta.max !== undefined && raw > meta.max) return t("error_number_max", { max: meta.max });
    return undefined;
  };

  const rows = useMemo(() => {
    const first = FIELDS.slice(0, 3);
    const second = FIELDS.slice(3);
    return [first, second];
  }, []);

  const handleChange = (meta: FieldMeta, value: string) => {
    const raw = value === "" ? Number.NaN : Number(value);
    const error = validate(meta.key, raw, meta);
    setErrors((prev) => ({ ...prev, [meta.key]: error ?? "" }));
    setForm(meta.key, clamp(raw, meta.min, meta.max));
  };

  const handleBlur = (meta: FieldMeta) => {
    setForm(meta.key, clamp(form[meta.key], meta.min, meta.max));
    setErrors((prev) => ({ ...prev, [meta.key]: "" }));
  };

  return (
    <div className="page settings-page">
      <h2>{t("settings_model")}</h2>

      {rows.map((row, rowIndex) => (
        <div className="row" key={rowIndex}>
          {row.map((meta) => (
            <label key={meta.key}>
              <span className="field-row">
                {t(meta.labelKey)}
                <Tooltip content={t(meta.helpKey)}>
                  <span className="field-info" aria-hidden="true">ⓘ</span>
                </Tooltip>
              </span>
              <input
                type="number"
                min={meta.min}
                max={meta.max}
                step={meta.step}
                value={form[meta.key]}
                onChange={(e) => handleChange(meta, e.target.value)}
                onBlur={() => handleBlur(meta)}
                aria-invalid={!!errors[meta.key]}
                aria-errormessage={errors[meta.key] ? `error-${meta.key}` : undefined}
              />
              {errors[meta.key] && (
                <span className="inline-error" id={`error-${meta.key}`} role="alert">
                  {errors[meta.key]}
                </span>
              )}
            </label>
          ))}
        </div>
      ))}
    </div>
  );
}
