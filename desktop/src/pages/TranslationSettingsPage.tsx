import { memo } from "react";
import { useTranslation } from "react-i18next";
import type { TranslationSettingsState } from "../types";
import Tooltip from "../components/Tooltip";
import {
  excludeSelectorExamples,
  outputModes,
  sourceLanguages,
  styles,
  targetLanguages,
  translateAttributeExamples,
} from "../types";

interface TranslationSettingsPageProps {
  settings: TranslationSettingsState;
  setSettings: (update: Partial<TranslationSettingsState>) => void;
}

const SCOPE_KEYS = [
  "translate_body",
  "translate_metadata",
  "translate_toc",
  "translate_alt_text",
  "translate_image_captions",
  "translate_tables",
  "translate_footnotes",
  "translate_code",
] as const;

function TranslationSettingsPage({
  settings,
  setSettings,
}: TranslationSettingsPageProps) {
  const { t } = useTranslation();

  const appendExample = (key: "exclude_selectors" | "translate_attributes", value: string) => {
    const current = settings[key];
    const parts = current.split(",").map((s) => s.trim()).filter(Boolean);
    if (parts.includes(value)) return;
    setSettings({ [key]: parts.length > 0 ? `${current}, ${value}` : value });
  };

  return (
    <div className="page settings-page">
      <h2>{t("settings_translation")}</h2>

      <div className="row">
        <label>
          {t("source_lang")}
          <select
            value={settings.source_lang}
            onChange={(e) => setSettings({ source_lang: e.target.value })}
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
            value={settings.target_lang}
            onChange={(e) => setSettings({ target_lang: e.target.value })}
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
            value={settings.output_mode}
            onChange={(e) => setSettings({ output_mode: e.target.value })}
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
            value={settings.style}
            onChange={(e) => setSettings({ style: e.target.value })}
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
          checked={settings.preserve_classes}
          onChange={(e) => setSettings({ preserve_classes: e.target.checked })}
        />
        {t("preserve_classes")}
      </label>

      <fieldset className="scope-fieldset">
        <legend>{t("translation_scope")}</legend>
        {SCOPE_KEYS.map((key) => (
          <label className="checkbox" key={key}>
            <input
              type="checkbox"
              checked={settings[key]}
              onChange={(e) => setSettings({ [key]: e.target.checked })}
            />
            {t(key)}
          </label>
        ))}
      </fieldset>

      <label>
        <span className="field-row">
          {t("exclude_selectors")}
          <Tooltip content={t("exclude_selectors_help")}>
            <span className="field-info" aria-hidden="true">ⓘ</span>
          </Tooltip>
        </span>
        <input
          type="text"
          value={settings.exclude_selectors}
          onChange={(e) => setSettings({ exclude_selectors: e.target.value })}
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
          <Tooltip content={t("translate_attributes_help")}>
            <span className="field-info" aria-hidden="true">ⓘ</span>
          </Tooltip>
        </span>
        <input
          type="text"
          value={settings.translate_attributes}
          onChange={(e) => setSettings({ translate_attributes: e.target.value })}
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

export default memo(TranslationSettingsPage);
