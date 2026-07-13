import { memo, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import type { PromptSettingsState, PromptTemplates } from "../types";

interface PromptsPageProps {
  promptSettings: PromptSettingsState;
  setPromptSettings: (update: Partial<PromptSettingsState>) => void;
}

const emptyPrompts: PromptTemplates = {
  default: "",
  literary: "",
  technical: "",
  academic: "",
  refine: "",
};

function promptsAreEmpty(prompts: PromptTemplates): boolean {
  return Object.values(prompts).every((v) => typeof v === "string" && v.trim() === "");
}

function PromptsPage({ promptSettings, setPromptSettings }: PromptsPageProps) {
  const { t } = useTranslation();

  useEffect(() => {
    if (!promptsAreEmpty(promptSettings.prompts)) {
      return;
    }
    invoke<PromptTemplates>("get_default_prompts")
      .then((defaults) => {
        if (promptsAreEmpty(promptSettings.prompts)) {
          setPromptSettings({ prompts: defaults });
        }
      })
      .catch(() => undefined);
  }, []);

  const updatePrompt = <K extends keyof PromptTemplates>(
    key: K,
    value: PromptTemplates[K]
  ) => {
    setPromptSettings({ prompts: { ...promptSettings.prompts, [key]: value } });
  };

  const handleReset = async () => {
    const confirmed = await confirm(t("confirm_reset_prompts"), {
      title: t("confirm_reset_prompts_title"),
      kind: "warning",
    });
    if (!confirmed) return;
    setPromptSettings({ system_prompt: "" });
    try {
      const defaults = await invoke<PromptTemplates>("get_default_prompts");
      setPromptSettings({ prompts: defaults });
    } catch {
      setPromptSettings({ prompts: { ...emptyPrompts } });
    }
  };

  const fields: { key: keyof PromptTemplates; labelKey: string }[] = [
    { key: "default", labelKey: "prompt_default" },
    { key: "literary", labelKey: "prompt_literary" },
    { key: "technical", labelKey: "prompt_technical" },
    { key: "academic", labelKey: "prompt_academic" },
    { key: "refine", labelKey: "prompt_refine" },
  ];

  return (
    <div className="page settings-page">
      <h2>{t("settings_prompts")}</h2>

      <label>
        {t("system_prompt")}
        <textarea
          value={promptSettings.system_prompt}
          onChange={(e) => setPromptSettings({ system_prompt: e.target.value })}
          className="prompt-textarea"
          placeholder={t("system_prompt_placeholder")}
        />
      </label>

      {fields.map(({ key, labelKey }) => (
        <label key={key}>
          {t(labelKey)}
          <textarea
            value={promptSettings.prompts[key]}
            onChange={(e) => updatePrompt(key, e.target.value)}
            className="prompt-textarea"
            placeholder={t(`prompt_${key}_placeholder`)}
          />
        </label>
      ))}

      <p className="hint">
        {t("prompts_help", {
          source_lang: "{source_lang}",
          target_lang: "{target_lang}",
        })}
      </p>

      <button
        type="button"
        className="text-button danger"
        onClick={handleReset}
      >
        {t("reset_prompts")}
      </button>
    </div>
  );
}

export default memo(PromptsPage);
