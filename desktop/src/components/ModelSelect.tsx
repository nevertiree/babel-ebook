import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import LoadingSpinner from "./LoadingSpinner";
import { isVisionModel } from "../utils";

interface ModelSelectProps {
  provider: string;
  apiKey: string;
  baseUrl: string;
  useCustomBaseUrl: boolean;
  model: string;
  onChange: (value: string) => void;
  /**
   * When true, the dropdown only offers vision/multimodal-capable models for
   * this provider (e.g. for OCR, which must send page images). Non-vision models
   * are hidden; "custom" remains available as an escape hatch so any model can
   * still be typed.
   */
  visionOnly?: boolean;
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
  visionOnly = false,
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
        const candidates = visionOnly ? list.filter((m) => isVisionModel(provider, m)) : list;
        if (candidates.length > 0 && !candidates.includes(model) && model !== "__custom__") {
          onChange(candidates[0]);
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
  }, [provider, apiKey, baseUrl, useCustomBaseUrl, visionOnly, onChange, model]);

  const filteredModels = visionOnly ? models.filter((m) => isVisionModel(provider, m)) : models;
  const hasModels = filteredModels.length > 0;
  const isCustom = model === "__custom__" || (hasModels && !filteredModels.includes(model));
  const showSpinner = loading && models.length === 0;

  return (
    <label title={error ?? undefined}>
      <span className="field-row">
        {t("model")}
        {showSpinner && <LoadingSpinner size={14} />}
      </span>
      {hasModels && !isCustom ? (
        <select value={model} onChange={(e) => onChange(e.target.value)} disabled={loading}>
          {filteredModels.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
          <option value="__custom__">{t("model_custom")}</option>
        </select>
      ) : (
        <input
          type="text"
          value={model === "__custom__" ? "" : model}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t("model_custom_placeholder")}
          disabled={loading}
        />
      )}
    </label>
  );
}
