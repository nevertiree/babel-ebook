import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import LoadingSpinner from "./LoadingSpinner";

interface ModelSelectProps {
  provider: string;
  apiKey: string;
  baseUrl: string;
  useCustomBaseUrl: boolean;
  model: string;
  onChange: (value: string) => void;
}

/**
 * Provider-aware model picker. Fetches the models available to the given
 * provider credentials (via `list_models`), falls back to a free-text input
 * when the list is empty or the user picks "custom", and shows a spinner while
 * loading. Shared by the Translate and OCR pages so model selection behaves
 * identically everywhere.
 */
export default function ModelSelect({
  provider,
  apiKey,
  baseUrl,
  useCustomBaseUrl,
  model,
  onChange,
}: ModelSelectProps) {
  const { t } = useTranslation();
  const [models, setModels] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestIdRef = useRef(0);

  useEffect(() => {
    const requestId = (requestIdRef.current += 1);
    setLoading(true);
    setError(null);

    void (async () => {
      try {
        const list = await invoke<string[]>("list_models", {
          args: {
            provider,
            api_key: apiKey,
            base_url: useCustomBaseUrl ? baseUrl || null : null,
          },
        });
        if (requestId !== requestIdRef.current) return;
        setModels(list);
        if (list.length > 0 && !list.includes(model) && model !== "__custom__") {
          onChange(list[0]);
        }
      } catch (err) {
        if (requestId !== requestIdRef.current) return;
        setModels([]);
        setError(String(err));
      } finally {
        if (requestId === requestIdRef.current) {
          setLoading(false);
        }
      }
    })();

    return () => {
      // Increment the id so that any in-flight response for this effect is
      // discarded when the dependencies change.
      requestIdRef.current += 1;
    };
  }, [provider, apiKey, baseUrl, useCustomBaseUrl]);

  const isCustom = model === "__custom__" || (models.length > 0 && !models.includes(model));

  const showSpinner = loading && models.length === 0;

  return (
    <label title={error ?? undefined}>
      <span className="field-row">
        {t("model")}
        {showSpinner && <LoadingSpinner size={14} />}
      </span>
      {models.length === 0 ? (
        <input
          type="text"
          value={model === "__custom__" ? "" : model}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t("model_custom_placeholder")}
          disabled={loading}
        />
      ) : isCustom ? (
        <input
          type="text"
          value={model === "__custom__" ? "" : model}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t("model_custom_placeholder")}
          disabled={loading}
        />
      ) : (
        <select
          value={model}
          onChange={(e) => onChange(e.target.value)}
          disabled={loading}
        >
          {models.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
          <option value="__custom__">{t("model_custom")}</option>
        </select>
      )}
    </label>
  );
}
