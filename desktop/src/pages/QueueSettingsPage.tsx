import { memo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { QueueSettingsState } from "../types";
import Tooltip from "../components/Tooltip";

interface QueueSettingsPageProps {
  queueSettings: QueueSettingsState;
  setQueueSettings: (update: Partial<QueueSettingsState>) => void;
}

function clamp(value: number, min: number, max?: number): number {
  let v = value;
  if (Number.isNaN(v)) return min;
  v = Math.max(min, v);
  if (max !== undefined) v = Math.min(max, v);
  return v;
}

function QueueSettingsPage({ queueSettings, setQueueSettings }: QueueSettingsPageProps) {
  const { t } = useTranslation();
  const [error, setError] = useState<string>("");

  const validate = (raw: number): string | undefined => {
    if (Number.isNaN(raw)) return t("error_number_required");
    if (raw < 1) return t("error_number_min", { min: 1 });
    if (raw > 10) return t("error_number_max", { max: 10 });
    return undefined;
  };

  const handleChange = (value: string) => {
    const raw = value === "" ? Number.NaN : Number(value);
    const err = validate(raw);
    setError(err ?? "");
    setQueueSettings({ concurrency: clamp(raw, 1, 10) });
  };

  const handleBlur = () => {
    setQueueSettings({ concurrency: clamp(queueSettings.concurrency, 1, 10) });
    setError("");
  };

  return (
    <div className="page settings-page">
      <h2>{t("settings_queue")}</h2>

      <div className="row">
        <label>
          <span className="field-row">
            {t("concurrency")}
            <Tooltip content={t("concurrency_help")}>
              <span className="field-info" aria-hidden="true">
                ⓘ
              </span>
            </Tooltip>
          </span>
          <input
            type="number"
            min={1}
            max={10}
            step={1}
            value={queueSettings.concurrency}
            onChange={(e) => handleChange(e.target.value)}
            onBlur={handleBlur}
            aria-invalid={!!error}
            aria-errormessage={error ? "error-concurrency" : undefined}
          />
          {error && (
            <span className="inline-error" id="error-concurrency" role="alert">
              {error}
            </span>
          )}
        </label>
      </div>

      <p className="hint">{t("queue_settings_hint")}</p>
    </div>
  );
}

export default memo(QueueSettingsPage);
