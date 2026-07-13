import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Page, ProviderConfig, TranslateInputs } from "../types";
import EmptyStateIcon from "../components/EmptyStateIcon";
import "./OcrPage.css";

interface OcrPageProps {
  inputs: TranslateInputs;
  setInputs: (update: Partial<TranslateInputs>) => void;
  onPageChange: (page: Page) => void;
}

function OcrPage({ inputs, setInputs, onPageChange }: OcrPageProps) {
  const { t } = useTranslation();
  const providers = inputs.providers;
  const hasProviders = providers.length > 0;

  const [pdfPath, setPdfPath] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [title, setTitle] = useState("");

  const [ocrProviderName, setOcrProviderName] = useState(inputs.active_provider);
  const [ocrModel, setOcrModel] = useState("qwen-vl-ocr");
  const [ocrConcurrency, setOcrConcurrency] = useState(3);
  const [dpi, setDpi] = useState(200);

  const [verifyEnabled, setVerifyEnabled] = useState(false);
  const [verifyProviderName, setVerifyProviderName] = useState(inputs.active_provider);
  const [verifyModel, setVerifyModel] = useState("deepseek-chat");
  const [verifyThreshold, setVerifyThreshold] = useState(0.7);
  const [verifyMaxAttempts, setVerifyMaxAttempts] = useState(3);

  const [refineEnabled, setRefineEnabled] = useState(false);
  const [refineProviderName, setRefineProviderName] = useState(inputs.active_provider);
  const [refineModel, setRefineModel] = useState("qwen-max");
  const [refineRounds, setRefineRounds] = useState(1);
  const [refineWithImage, setRefineWithImage] = useState(false);

  const [converting, setConverting] = useState(false);
  const [resultPath, setResultPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const providerByName = (name: string): ProviderConfig | undefined =>
    providers.find((p) => p.name === name);

  const selectPdf = async () => {
    const path = await open({
      filters: [{ name: t("pdf_files"), extensions: ["pdf"] }],
    });
    if (path && !Array.isArray(path)) {
      setPdfPath(path);
      setResultPath(null);
      if (!title) {
        const stem = path.replace(/\.pdf$/i, "");
        setTitle(stem.split(/[\\/]/).pop() || stem);
      }
      if (!outputPath) {
        const stem = path.replace(/\.pdf$/i, "");
        setOutputPath(`${stem}.epub`);
      }
    }
  };

  const selectOutput = async () => {
    const path = await save({
      filters: [{ name: "EPUB", extensions: ["epub"] }],
    });
    if (path && !Array.isArray(path)) {
      setOutputPath(path);
      setResultPath(null);
    }
  };

  const convert = async () => {
    const ocrProv = providerByName(ocrProviderName);
    if (!ocrProv?.api_key) {
      setError(t("error_no_provider"));
      return;
    }
    if (!pdfPath || !outputPath) {
      setError(t("ocr_error_no_files"));
      return;
    }
    setConverting(true);
    setError(null);
    setResultPath(null);
    try {
      const verifyProv = verifyEnabled ? providerByName(verifyProviderName) : undefined;
      const refineProv = refineEnabled ? providerByName(refineProviderName) : undefined;
      const args = {
        pdf_path: pdfPath,
        output_path: outputPath,
        title: title.trim() || null,
        ocr_api_key: ocrProv.api_key,
        ocr_base_url: ocrProv.use_custom_base_url ? ocrProv.base_url || null : null,
        ocr_model: ocrModel.trim() || null,
        ocr_concurrency: ocrConcurrency,
        no_verify: !verifyEnabled,
        verify_api_key: verifyProv?.api_key ?? null,
        verify_base_url: verifyProv?.use_custom_base_url ? verifyProv.base_url || null : null,
        verify_model: verifyModel.trim() || null,
        dpi,
        verify_threshold: verifyThreshold,
        verify_max_attempts: verifyMaxAttempts,
        verify_scale_factors: [1, 2, 3],
        ocr_refine_rounds: refineEnabled ? refineRounds : 0,
        ocr_refine_api_key: refineProv?.api_key ?? null,
        ocr_refine_base_url: refineProv?.use_custom_base_url ? refineProv.base_url || null : null,
        ocr_refine_model: refineModel.trim() || null,
        ocr_refine_with_image: refineEnabled && refineWithImage,
      };
      const out = await invoke<string>("convert_pdf_to_epub", { args });
      setResultPath(out);
    } catch (err) {
      setError(String(err));
    } finally {
      setConverting(false);
    }
  };

  const sendToTranslate = () => {
    if (!resultPath) return;
    setInputs({ source: resultPath });
    if (inputs.resume) {
      setInputs({ resume: "" });
    }
    onPageChange("translate");
  };

  if (!hasProviders) {
    return (
      <div className="page ocr-page">
        <h2>{t("ocr_title")}</h2>
        <div className="empty-state">
          <EmptyStateIcon variant="provider" className="empty-state-icon" />
          <p>{t("no_provider_configured")}</p>
          <button type="button" onClick={() => onPageChange("settings-compute")}>
            {t("configure_provider")}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="page ocr-page">
      <h2>{t("ocr_title")}</h2>

      <section className="ocr-section">
        <h3>{t("ocr_files")}</h3>
        <div className="ocr-field">
          <label>{t("ocr_source_pdf")}</label>
          <div className="ocr-file-row">
            <span className={`ocr-file-path ${pdfPath ? "" : "empty"}`} title={pdfPath || undefined}>
              {pdfPath || t("ocr_no_pdf")}
            </span>
            <button type="button" onClick={selectPdf}>
              {t("select_file")}
            </button>
          </div>
        </div>
        <div className="ocr-field">
          <label>{t("ocr_output_epub")}</label>
          <div className="ocr-file-row">
            <span
              className={`ocr-file-path ${outputPath ? "" : "empty"}`}
              title={outputPath || undefined}
            >
              {outputPath || t("ocr_no_output")}
            </span>
            <button type="button" onClick={selectOutput}>
              {t("select_file")}
            </button>
          </div>
        </div>
        <div className="ocr-field">
          <label>{t("ocr_title_label")}</label>
          <input
            type="text"
            className="ocr-input"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t("ocr_title_label")}
          />
        </div>
      </section>

      <section className="ocr-section">
        <h3>{t("ocr_engine")}</h3>
        <div className="ocr-field">
          <label>{t("ocr_provider")}</label>
          <select
            className="ocr-select"
            value={ocrProviderName}
            onChange={(e) => setOcrProviderName(e.target.value)}
          >
            {providers.map((p) => (
              <option key={p.name} value={p.name}>
                {p.name}
              </option>
            ))}
          </select>
        </div>
        <div className="ocr-field">
          <label>{t("ocr_model")}</label>
          <input
            type="text"
            className="ocr-input"
            value={ocrModel}
            onChange={(e) => setOcrModel(e.target.value)}
          />
        </div>
        <details className="ocr-collapsible">
          <summary>{t("ocr_advanced")}</summary>
          <div className="ocr-inline-row">
            <label>
              {t("ocr_concurrency")}
              <input
                type="number"
                min={1}
                value={ocrConcurrency}
                onChange={(e) => setOcrConcurrency(Number(e.target.value) || 1)}
              />
            </label>
            <label>
              {t("ocr_dpi")}
              <input
                type="number"
                min={50}
                value={dpi}
                onChange={(e) => setDpi(Number(e.target.value) || 200)}
              />
            </label>
          </div>
        </details>
      </section>

      <section className="ocr-section">
        <h3>{t("ocr_verify")}</h3>
        <label className="ocr-checkbox">
          <input
            type="checkbox"
            checked={verifyEnabled}
            onChange={(e) => setVerifyEnabled(e.target.checked)}
          />
          {t("ocr_verify_enable")}
        </label>
        {verifyEnabled && (
          <>
            <div className="ocr-field">
              <label>{t("ocr_provider")}</label>
              <select
                className="ocr-select"
                value={verifyProviderName}
                onChange={(e) => setVerifyProviderName(e.target.value)}
              >
                {providers.map((p) => (
                  <option key={p.name} value={p.name}>
                    {p.name}
                  </option>
                ))}
              </select>
            </div>
            <div className="ocr-field">
              <label>{t("ocr_model")}</label>
              <input
                type="text"
                className="ocr-input"
                value={verifyModel}
                onChange={(e) => setVerifyModel(e.target.value)}
              />
            </div>
            <div className="ocr-inline-row">
              <label>
                {t("ocr_verify_threshold")}
                <input
                  type="number"
                  step={0.05}
                  min={0}
                  max={1}
                  value={verifyThreshold}
                  onChange={(e) => setVerifyThreshold(Number(e.target.value) || 0)}
                />
              </label>
              <label>
                {t("ocr_verify_max_attempts")}
                <input
                  type="number"
                  min={0}
                  value={verifyMaxAttempts}
                  onChange={(e) => setVerifyMaxAttempts(Number(e.target.value) || 0)}
                />
              </label>
            </div>
          </>
        )}
      </section>

      <section className="ocr-section">
        <h3>{t("ocr_refine")}</h3>
        <label className="ocr-checkbox">
          <input
            type="checkbox"
            checked={refineEnabled}
            onChange={(e) => setRefineEnabled(e.target.checked)}
          />
          {t("ocr_refine_enable")}
        </label>
        {refineEnabled && (
          <>
            <div className="ocr-inline-row" style={{ marginBottom: "0.85rem" }}>
              <label>
                {t("ocr_refine_rounds")}
                <input
                  type="number"
                  min={1}
                  value={refineRounds}
                  onChange={(e) => setRefineRounds(Number(e.target.value) || 1)}
                />
              </label>
            </div>
            <div className="ocr-field">
              <label>{t("ocr_provider")}</label>
              <select
                className="ocr-select"
                value={refineProviderName}
                onChange={(e) => setRefineProviderName(e.target.value)}
              >
                {providers.map((p) => (
                  <option key={p.name} value={p.name}>
                    {p.name}
                  </option>
                ))}
              </select>
            </div>
            <div className="ocr-field">
              <label>{t("ocr_model")}</label>
              <input
                type="text"
                className="ocr-input"
                value={refineModel}
                onChange={(e) => setRefineModel(e.target.value)}
              />
            </div>
            <label className="ocr-checkbox">
              <input
                type="checkbox"
                checked={refineWithImage}
                onChange={(e) => setRefineWithImage(e.target.checked)}
              />
              {t("ocr_refine_with_image")}
            </label>
          </>
        )}
      </section>

      <div className="ocr-actions">
        <button
          type="button"
          className="button-primary"
          onClick={convert}
          disabled={converting || !pdfPath || !outputPath}
        >
          {converting ? t("ocr_converting") : t("ocr_convert")}
        </button>
        {resultPath && (
          <button type="button" className="button-secondary" onClick={sendToTranslate}>
            {t("ocr_send_to_translate")}
          </button>
        )}
      </div>

      {error && <div className="ocr-error">{error}</div>}
      {resultPath && (
        <div className="ocr-success">
          {t("ocr_convert_success")}
          <div className="ocr-success-path">{resultPath}</div>
        </div>
      )}
    </div>
  );
}

export default OcrPage;
