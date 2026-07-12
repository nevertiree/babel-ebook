import { memo } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import type { OutputSettingsState } from "../types";
import { defaultFonts } from "../types";
import Tooltip from "../components/Tooltip";

const filenameTemplateExamples = ["{stem}_{target_lang}", "{stem}_{output_mode}", "{stem}"];

interface OutputSettingsPageProps {
  outputSettings: OutputSettingsState;
  setOutputSettings: (update: Partial<OutputSettingsState>) => void;
  targetLang: string;
}

function OutputSettingsPage({
  outputSettings,
  setOutputSettings,
  targetLang,
}: OutputSettingsPageProps) {
  const { t } = useTranslation();

  const selectCheckpointDir = async () => {
    const path = await open({
      directory: true,
      defaultPath: outputSettings.checkpoint_dir || undefined,
    });
    if (path) {
      setOutputSettings({ checkpoint_dir: path });
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
            value={outputSettings.checkpoint_dir}
            onChange={(e) => setOutputSettings({ checkpoint_dir: e.target.value })}
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
          value={outputSettings.output_font}
          onChange={(e) => setOutputSettings({ output_font: e.target.value })}
          placeholder={t("output_font_placeholder")}
        />
        <button
          type="button"
          className="text-button"
          onClick={() =>
            setOutputSettings({ output_font: defaultFonts[targetLang] ?? defaultFonts.en })
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
          value={outputSettings.output_filename_template}
          onChange={(e) => setOutputSettings({ output_filename_template: e.target.value })}
          placeholder="{stem}_{target_lang}"
        />
        <div className="example-chips">
          {filenameTemplateExamples.map((ex) => (
            <button key={ex} type="button" onClick={() => setOutputSettings({ output_filename_template: ex })}>
              + {ex}
            </button>
          ))}
        </div>
      </label>
    </div>
  );
}

export default memo(OutputSettingsPage);
