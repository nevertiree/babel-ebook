import { useTranslation } from "react-i18next";
import type { FormState } from "../types";
import { defaultFonts } from "../types";

const filenameTemplateExamples = ["{stem}_{target_lang}", "{stem}_{output_mode}", "{stem}"];

interface OutputSettingsPageProps {
  form: FormState;
  setForm: <K extends keyof FormState>(key: K, value: FormState[K]) => void;
}

export default function OutputSettingsPage({
  form,
  setForm,
}: OutputSettingsPageProps) {
  const { t } = useTranslation();

  return (
    <div className="page settings-page">
      <h2>{t("settings_output")}</h2>

      <label className="checkbox">
        <input
          type="checkbox"
          checked={form.dry_run}
          onChange={(e) => setForm("dry_run", e.target.checked)}
        />
        {t("dry_run")}
      </label>

      <label>
        <span className="field-row">
          {t("output_font")}
          <span className="field-info" data-tooltip={t("output_font_help")}>
            ⓘ
          </span>
        </span>
        <input
          type="text"
          value={form.output_font}
          onChange={(e) => setForm("output_font", e.target.value)}
          placeholder={t("output_font_placeholder")}
        />
        <button
          type="button"
          className="text-button"
          onClick={() =>
            setForm("output_font", defaultFonts[form.target_lang] ?? defaultFonts.en)
          }
        >
          {t("reset_default_font")}
        </button>
      </label>

      <label>
        <span className="field-row">
          {t("output_filename_template")}
          <span className="field-info" data-tooltip={t("output_filename_template_help")}>
            ⓘ
          </span>
        </span>
        <input
          type="text"
          value={form.output_filename_template}
          onChange={(e) => setForm("output_filename_template", e.target.value)}
          placeholder="{stem}_{target_lang}"
        />
        <div className="example-chips">
          {filenameTemplateExamples.map((ex) => (
            <button key={ex} type="button" onClick={() => setForm("output_filename_template", ex)}>
              + {ex}
            </button>
          ))}
        </div>
      </label>
    </div>
  );
}
