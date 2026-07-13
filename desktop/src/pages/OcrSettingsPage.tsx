import { useTranslation } from "react-i18next";
import type { OcrSettings } from "../config";

interface OcrSettingsPageProps {
  ocrSettings: OcrSettings;
  setOcrSettings: (next: OcrSettings) => void;
}

/**
 * "Set once" OCR engine tuning: concurrency, DPI, and the verify/refine quality
 * knobs. The OCR/verify/refine *models* are picked on the OCR page itself,
 * because they depend on the chosen OCR provider (the model list is fetched
 * per provider) - mirroring how the translate page keeps provider/model on the
 * page and pushes only provider-independent tuning into settings.
 */
export default function OcrSettingsPage({
  ocrSettings,
  setOcrSettings,
}: OcrSettingsPageProps) {
  const { t } = useTranslation();

  const setVerify = (patch: Partial<OcrSettings["verify"]>) =>
    setOcrSettings({ ...ocrSettings, verify: { ...ocrSettings.verify, ...patch } });
  const setRefine = (patch: Partial<OcrSettings["refine"]>) =>
    setOcrSettings({ ...ocrSettings, refine: { ...ocrSettings.refine, ...patch } });

  return (
    <div className="page settings-page ocr-settings-page">
      <h2>{t("settings_ocr")}</h2>
      <p className="hint">{t("ocr_settings_models_hint")}</p>

      <h3>{t("ocr_engine")}</h3>
      <div className="row">
        <label>
          {t("ocr_concurrency")}
          <input
            type="number"
            min={1}
            value={ocrSettings.concurrency}
            onChange={(e) =>
              setOcrSettings({ ...ocrSettings, concurrency: Number(e.target.value) || 1 })
            }
          />
        </label>
        <label>
          {t("ocr_dpi")}
          <input
            type="number"
            min={50}
            value={ocrSettings.dpi}
            onChange={(e) => setOcrSettings({ ...ocrSettings, dpi: Number(e.target.value) || 200 })}
          />
        </label>
      </div>

      <h3>{t("ocr_verify")}</h3>
      <p className="hint">{t("ocr_verify_settings_hint")}</p>
      <div className="row">
        <label>
          {t("ocr_verify_threshold")}
          <input
            type="number"
            step={0.05}
            min={0}
            max={1}
            value={ocrSettings.verify.threshold}
            onChange={(e) => setVerify({ threshold: Number(e.target.value) || 0 })}
          />
        </label>
        <label>
          {t("ocr_verify_max_attempts")}
          <input
            type="number"
            min={0}
            value={ocrSettings.verify.maxAttempts}
            onChange={(e) => setVerify({ maxAttempts: Number(e.target.value) || 0 })}
          />
        </label>
      </div>

      <h3>{t("ocr_refine")}</h3>
      <p className="hint">{t("ocr_refine_settings_hint")}</p>
      <div className="row">
        <label>
          {t("ocr_refine_rounds")}
          <input
            type="number"
            min={1}
            value={ocrSettings.refine.rounds}
            onChange={(e) => setRefine({ rounds: Number(e.target.value) || 1 })}
          />
        </label>
        <label className="checkbox-label">
          <input
            type="checkbox"
            checked={ocrSettings.refine.withImage}
            onChange={(e) => setRefine({ withImage: e.target.checked })}
          />
          {t("ocr_refine_with_image")}
        </label>
      </div>
    </div>
  );
}
