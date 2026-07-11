import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import type { FormState } from "../types";
import { defaultFonts } from "../types";
import Tooltip from "../components/Tooltip";

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

  const selectCheckpointDir = async () => {
    const path = await open({
      directory: true,
      defaultPath: form.checkpoint_dir || undefined,
    });
    if (path) {
      setForm("checkpoint_dir", path);
    }
  };

  return (
    <div className="page settings-page">
      <h2>{t("settings_output")}</h2>

      <label>
        <span className="field-row">
          {t("checkpoint_dir")}
          <Tooltip content={t("checkpoint_dir_help")}>
            <span className="field-info" aria-hidden="true">ⓘ</span>
          </Tooltip>
        </span>
        <div className="file-row">
          <input
            type="text"
            value={form.checkpoint_dir}
            onChange={(e) => setForm("checkpoint_dir", e.target.value)}
            placeholder={t("checkpoint_dir_placeholder")}
            style={{ flex: 1 }}
          />
          <button type="button" onClick={selectCheckpointDir}>
            {t("select_directory")}
          </button>
        </div>
      </label>

      <label>
        <span className="field-row">
          {t("output_font")}
          <Tooltip content={t("output_font_help")}>
            <span className="field-info" aria-hidden="true">ⓘ</span>
          </Tooltip>
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
          <Tooltip content={t("output_filename_template_help")}>
            <span className="field-info" aria-hidden="true">ⓘ</span>
          </Tooltip>
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
