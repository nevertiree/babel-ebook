import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  defaults,
  outputModes,
  targetLanguages,
  type Page,
  type ProviderConfig,
  type TranslateInputs,
} from "../types";
import EmptyStateIcon from "../components/EmptyStateIcon";
import ProviderIcon from "../components/ProviderIcon";
import ModelSelect from "../components/ModelSelect";
import "./OcrPage.css";

interface OcrProgress {
  stage: "render" | "ocr" | "refine" | "done";
  page: number;
  page_total: number;
  refine_round: number | null;
  percent: number;
  message: string;
}

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
  const outputPathRef = useRef(outputPath);
  outputPathRef.current = outputPath;
  const [title, setTitle] = useState("");

  // OCR engine. verify/refine reuse the OCR provider's credentials and only
  // expose their own model, mirroring the translate page's single-provider model.
  const [ocrProviderName, setOcrProviderName] = useState(inputs.active_provider);
  const [ocrModel, setOcrModel] = useState("qwen-vl-ocr");
  const [ocrConcurrency, setOcrConcurrency] = useState(3);
  const [dpi, setDpi] = useState(200);

  const [verifyEnabled, setVerifyEnabled] = useState(false);
  const [verifyModel, setVerifyModel] = useState("deepseek-chat");
  const [verifyThreshold, setVerifyThreshold] = useState(0.7);
  const [verifyMaxAttempts, setVerifyMaxAttempts] = useState(3);

  const [refineEnabled, setRefineEnabled] = useState(false);
  const [refineModel, setRefineModel] = useState("qwen-max");
  const [refineRounds, setRefineRounds] = useState(1);
  const [refineWithImage, setRefineWithImage] = useState(false);

  // Pipeline mode reuses the shared translation inputs (provider/model/
  // target_lang/output_mode) configured on the translate page, so translation
  // settings live in one place. Only the final output path is OCR-specific.
  const [pipelineMode, setPipelineMode] = useState(false);
  const [translateOutputPath, setTranslateOutputPath] = useState("");

  const [converting, setConverting] = useState(false);
  const [resultPath, setResultPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<OcrProgress | null>(null);

  useEffect(() => {
    const unlisten = listen<OcrProgress>("ocr_progress", (e) => {
      setProgress(e.payload);
      if (e.payload.stage === "done") {
        setConverting(false);
        setResultPath(outputPathRef.current);
      }
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const providerByName = (name: string): ProviderConfig | undefined =>
    providers.find((p) => p.name === name);
  const ocrProvider = providerByName(ocrProviderName);
  const translateProvider = providerByName(inputs.active_provider);

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

  const selectTranslateOutput = async () => {
    const path = await save({
      filters: [{ name: "EPUB", extensions: ["epub"] }],
    });
    if (path && !Array.isArray(path)) {
      setTranslateOutputPath(path);
    }
  };

  const buildOcrArgs = () => ({
    pdf_path: pdfPath,
    output_path: outputPath,
    title: title.trim() || null,
    ocr_api_key: ocrProvider?.api_key ?? "",
    ocr_base_url: ocrProvider?.use_custom_base_url ? ocrProvider.base_url || null : null,
    ocr_model: ocrModel.trim() || null,
    ocr_concurrency: ocrConcurrency,
    no_verify: !verifyEnabled,
    verify_api_key: verifyEnabled ? ocrProvider?.api_key ?? null : null,
    verify_base_url:
      verifyEnabled && ocrProvider?.use_custom_base_url ? ocrProvider.base_url || null : null,
    verify_model: verifyModel.trim() || null,
    dpi,
    verify_threshold: verifyThreshold,
    verify_max_attempts: verifyMaxAttempts,
    verify_scale_factors: [1, 2, 3],
    ocr_refine_rounds: refineEnabled ? refineRounds : 0,
    ocr_refine_api_key: refineEnabled ? ocrProvider?.api_key ?? null : null,
    ocr_refine_base_url:
      refineEnabled && ocrProvider?.use_custom_base_url ? ocrProvider.base_url || null : null,
    ocr_refine_model: refineModel.trim() || null,
    ocr_refine_with_image: refineEnabled && refineWithImage,
  });

  const buildTranslateArgs = () => ({
    source: outputPath,
    output: translateOutputPath,
    provider: translateProvider?.provider ?? "deepseek",
    api_key: translateProvider?.api_key ?? "",
    base_url: translateProvider?.use_custom_base_url ? translateProvider.base_url || null : null,
    model: inputs.model,
    concurrency: defaults.concurrency,
    max_input_tokens: defaults.max_input_tokens,
    max_output_tokens: defaults.max_output_tokens,
    temperature: defaults.temperature,
    source_lang: inputs.source_lang,
    target_lang: inputs.target_lang,
    dry_run: false,
    output_mode: inputs.output_mode,
    style: defaults.style,
    preserve_classes: defaults.preserve_classes,
    exclude_selectors: [],
    translate_attributes: [],
    translate_body: defaults.translate_body,
    translate_metadata: defaults.translate_metadata,
    translate_toc: defaults.translate_toc,
    translate_alt_text: defaults.translate_alt_text,
    translate_image_captions: defaults.translate_image_captions,
    translate_tables: defaults.translate_tables,
    translate_footnotes: defaults.translate_footnotes,
    translate_code: defaults.translate_code,
    output_font: defaults.output_font || null,
    system_prompt: null,
    prompts: defaults.prompts,
    refine: false,
    checkpoint_dir: "",
    resume: null,
  });

  const convert = async () => {
    if (!ocrProvider?.api_key) {
      setError(t("error_no_provider"));
      return;
    }
    if (!pdfPath || !outputPath) {
      setError(t("ocr_error_no_files"));
      return;
    }
    if (pipelineMode) {
      if (!translateProvider?.api_key) {
        setError(t("error_no_provider"));
        return;
      }
      if (!translateOutputPath) {
        setError(t("ocr_error_no_files"));
        return;
      }
    }
    setConverting(true);
    setError(null);
    setResultPath(null);
    setProgress(null);
    try {
      const ocrArgs = buildOcrArgs();
      if (pipelineMode) {
        const translateArgs = buildTranslateArgs();
        await invoke("enqueue_pipeline_task", { ocrArgs, translateArgs });
        await invoke("start_queue");
        setConverting(false);
        onPageChange("tasks");
      } else {
        await invoke("enqueue_ocr_task", { args: ocrArgs });
        await invoke("start_queue");
      }
    } catch (err) {
      setError(String(err));
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

  const canStart =
    !!pdfPath && !!outputPath && (!pipelineMode || !!translateOutputPath) && !converting;

  return (
    <div className="page ocr-page">
      <h2>{t("ocr_title")}</h2>

      <label className="checkbox-label ocr-mode-toggle">
        <input
          type="checkbox"
          checked={pipelineMode}
          onChange={(e) => setPipelineMode(e.target.checked)}
        />
        {t("ocr_pipeline_enable")}
      </label>

      <section className="quick-settings">
        <div className="row">
          <label>
            {t("ocr_provider")}
            <div className="provider-select">
              {ocrProvider && (
                <ProviderIcon provider={ocrProvider.provider} className="provider-select-icon" />
              )}
              <select
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
          </label>

          <ModelSelect
            provider={ocrProvider?.provider ?? ""}
            apiKey={ocrProvider?.api_key ?? ""}
            baseUrl={ocrProvider?.base_url ?? ""}
            useCustomBaseUrl={ocrProvider?.use_custom_base_url ?? false}
            model={ocrModel}
            onChange={setOcrModel}
          />
        </div>
      </section>

      {pipelineMode && (
        <section className="quick-settings">
          <div className="row">
            <label>
              {t("provider")}
              <div className="provider-select">
                {translateProvider && (
                  <ProviderIcon
                    provider={translateProvider.provider}
                    className="provider-select-icon"
                  />
                )}
                <select
                  value={inputs.active_provider}
                  onChange={(e) => setInputs({ active_provider: e.target.value })}
                >
                  {providers.map((p) => (
                    <option key={p.name} value={p.name}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </div>
            </label>

            <ModelSelect
              provider={translateProvider?.provider ?? ""}
              apiKey={translateProvider?.api_key ?? ""}
              baseUrl={translateProvider?.base_url ?? ""}
              useCustomBaseUrl={translateProvider?.use_custom_base_url ?? false}
              model={inputs.model}
              onChange={(value) => setInputs({ model: value })}
            />

            <label>
              {t("target_lang")}
              <select
                value={inputs.target_lang}
                onChange={(e) => setInputs({ target_lang: e.target.value })}
              >
                {targetLanguages.map((lang) => (
                  <option key={lang.code} value={lang.code}>
                    {t(`target_lang_${lang.code}`)}
                  </option>
                ))}
              </select>
            </label>

            <label>
              {t("output_mode")}
              <select
                value={inputs.output_mode}
                onChange={(e) => setInputs({ output_mode: e.target.value })}
              >
                {outputModes.map((mode) => (
                  <option key={mode} value={mode}>
                    {t(`output_mode_${mode}`)}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </section>
      )}

      <section className="file-section">
        <div
          className="file-row file-row-source"
          role="button"
          tabIndex={0}
          onClick={selectPdf}
          aria-label={t("ocr_source_pdf")}
        >
          <div className="file-info">
            <span className="file-label">{t("ocr_source_pdf")}</span>
            <span className="file-path" title={pdfPath || undefined}>
              {pdfPath || t("ocr_no_pdf")}
            </span>
          </div>
          <div className="file-row-actions">
            {pdfPath && (
              <button
                type="button"
                className="icon-button"
                onClick={(e) => {
                  e.stopPropagation();
                  setPdfPath("");
                }}
                title={t("clear")}
                aria-label={t("clear")}
              >
                ×
              </button>
            )}
            <button type="button" onClick={(e) => { e.stopPropagation(); selectPdf(); }}>
              {t("select_file")}
            </button>
          </div>
        </div>

        <div className="file-row">
          <div className="file-info">
            <span className="file-label">
              {t("ocr_output_epub")}
              {pipelineMode && ` (${t("ocr_intermediate")})`}
            </span>
            <span className="file-path" title={outputPath || undefined}>
              {outputPath || t("ocr_no_output")}
            </span>
          </div>
          <div className="file-row-actions">
            {outputPath && (
              <button
                type="button"
                className="icon-button"
                onClick={() => {
                  setOutputPath("");
                  setResultPath(null);
                }}
                title={t("clear")}
                aria-label={t("clear")}
              >
                ×
              </button>
            )}
            <button type="button" onClick={selectOutput}>
              {t("save_as")}
            </button>
          </div>
        </div>

        {pipelineMode && (
          <div className="file-row">
            <div className="file-info">
              <span className="file-label">{t("ocr_final_output")}</span>
              <span className="file-path" title={translateOutputPath || undefined}>
                {translateOutputPath || t("ocr_no_output")}
              </span>
            </div>
            <div className="file-row-actions">
              {translateOutputPath && (
                <button
                  type="button"
                  className="icon-button"
                  onClick={() => setTranslateOutputPath("")}
                  title={t("clear")}
                  aria-label={t("clear")}
                >
                  ×
                </button>
              )}
              <button type="button" onClick={selectTranslateOutput}>
                {t("save_as")}
              </button>
            </div>
          </div>
        )}
      </section>

      <section className="advanced-section">
        <label>
          {t("ocr_title_label")}
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t("ocr_title_label")}
          />
        </label>

        <div className="refine-option">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={verifyEnabled}
              onChange={(e) => setVerifyEnabled(e.target.checked)}
            />
            {t("ocr_verify_enable")}
          </label>
          {verifyEnabled && (
            <div className="row">
              <label>
                {t("ocr_model")}
                <input
                  type="text"
                  value={verifyModel}
                  onChange={(e) => setVerifyModel(e.target.value)}
                />
              </label>
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
          )}
        </div>

        <div className="refine-option">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={refineEnabled}
              onChange={(e) => setRefineEnabled(e.target.checked)}
            />
            {t("ocr_refine_enable")}
          </label>
          {refineEnabled && (
            <div className="row">
              <label>
                {t("ocr_refine_rounds")}
                <input
                  type="number"
                  min={1}
                  value={refineRounds}
                  onChange={(e) => setRefineRounds(Number(e.target.value) || 1)}
                />
              </label>
              <label>
                {t("ocr_model")}
                <input
                  type="text"
                  value={refineModel}
                  onChange={(e) => setRefineModel(e.target.value)}
                />
              </label>
              <label className="checkbox-label">
                <input
                  type="checkbox"
                  checked={refineWithImage}
                  onChange={(e) => setRefineWithImage(e.target.checked)}
                />
                {t("ocr_refine_with_image")}
              </label>
            </div>
          )}
        </div>

        <div className="row">
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
      </section>

      <div className="start-row">
        <button
          className="start-button"
          type="button"
          onClick={convert}
          disabled={!canStart}
        >
          {converting
            ? t("ocr_converting")
            : pipelineMode
            ? t("ocr_convert_pipeline")
            : t("ocr_convert")}
        </button>
      </div>

      {progress && (
        <div className="ocr-progress-block">
          <div className="progress-header">
            <span>{progress.message}</span>
            <span>{progress.percent}%</span>
          </div>
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${progress.percent}%` }} />
          </div>
        </div>
      )}

      {error && <div className="ocr-error">{error}</div>}

      {resultPath && (
        <div className="inline-success">
          {t("ocr_convert_success")}
          <span className="inline-success-path">{resultPath}</span>
          <button
            type="button"
            className="button-secondary"
            onClick={sendToTranslate}
          >
            {t("ocr_send_to_translate")}
          </button>
        </div>
      )}
    </div>
  );
}

export default OcrPage;
