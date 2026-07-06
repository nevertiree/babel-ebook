import { useTranslation } from "react-i18next";
import type { FormState } from "../types";
import {
  excludeSelectorExamples,
  outputModes,
  sourceLanguages,
  styles,
  targetLanguages,
  translateAttributeExamples,
} from "../types";

interface TranslationSettingsPageProps {
  form: FormState;
  setForm: <K extends keyof FormState>(key: K, value: FormState[K]) => void;
}

export default function TranslationSettingsPage({
  form,
  setForm,
}: TranslationSettingsPageProps) {
  const { t } = useTranslation();

  const appendExample = (key: "exclude_selectors" | "translate_attributes", value: string) => {
    const current = form[key];
    const parts = current.split(",").map((s) => s.trim()).filter(Boolean);
    if (parts.includes(value)) return;
    setForm(key, parts.length > 0 ? `${current}, ${value}` : value);
  };

  return (
    <div className="page settings-page">
      <h2>{t("settings_translation")}</h2>

      <div className="row">
        <label>
          {t("source_lang")}
          <select
            value={form.source_lang}
            onChange={(e) => setForm("source_lang", e.target.value)}
          >
            {sourceLanguages.map((lang) => (
              <option key={lang.code} value={lang.code}>
                {lang.label}
              </option>
            ))}
          </select>
        </label>

        <label>
          {t("target_lang")}
          <select
            value={form.target_lang}
            onChange={(e) => setForm("target_lang", e.target.value)}
          >
            {targetLanguages.map((lang) => (
              <option key={lang.code} value={lang.code}>
                {lang.label}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="row">
        <label>
          {t("output_mode")}
          <select
            value={form.output_mode}
            onChange={(e) => setForm("output_mode", e.target.value)}
          >
            {outputModes.map((mode) => (
              <option key={mode} value={mode}>
                {t(`output_mode_${mode}`)}
              </option>
            ))}
          </select>
        </label>

        <label>
          {t("style")}
          <select
            value={form.style}
            onChange={(e) => setForm("style", e.target.value)}
          >
            {styles.map((style) => (
              <option key={style} value={style}>
                {t(`style_${style}`)}
              </option>
            ))}
          </select>
        </label>
      </div>

      <label className="checkbox">
        <input
          type="checkbox"
          checked={form.preserve_classes}
          onChange={(e) => setForm("preserve_classes", e.target.checked)}
        />
        {t("preserve_classes")}
      </label>

      <fieldset className="scope-fieldset">
        <legend>{t("translation_scope")}</legend>
        {(
          [
            "translate_body",
            "translate_metadata",
            "translate_toc",
            "translate_alt_text",
            "translate_image_captions",
            "translate_tables",
            "translate_footnotes",
            "translate_code",
          ] as const
        ).map((key) => (
          <label className="checkbox" key={key}>
            <input
              type="checkbox"
              checked={form[key]}
              onChange={(e) => setForm(key, e.target.checked)}
            />
            {t(key)}
          </label>
        ))}
      </fieldset>

      <label>
        <span className="field-row">
          {t("exclude_selectors")}
          <span className="field-info" data-tooltip={t("exclude_selectors_help")}>
            ⓘ
          </span>
        </span>
        <input
          type="text"
          value={form.exclude_selectors}
          onChange={(e) => setForm("exclude_selectors", e.target.value)}
          placeholder={t("exclude_selectors_placeholder")}
        />
        <div className="example-chips">
          {excludeSelectorExamples.map((ex) => (
            <button key={ex} type="button" onClick={() => appendExample("exclude_selectors", ex)}>
              + {ex}
            </button>
          ))}
        </div>
      </label>

      <label>
        <span className="field-row">
          {t("translate_attributes")}
          <span className="field-info" data-tooltip={t("translate_attributes_help")}>
            ⓘ
          </span>
        </span>
        <input
          type="text"
          value={form.translate_attributes}
          onChange={(e) => setForm("translate_attributes", e.target.value)}
          placeholder={t("translate_attributes_placeholder")}
        />
        <div className="example-chips">
          {translateAttributeExamples.map((ex) => (
            <button key={ex} type="button" onClick={() => appendExample("translate_attributes", ex)}>
              + {ex}
            </button>
          ))}
        </div>
      </label>
    </div>
  );
}
