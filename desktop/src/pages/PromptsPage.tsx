import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { FormState, PromptTemplates } from "../types";

interface PromptsPageProps {
  form: FormState;
  setForm: <K extends keyof FormState>(key: K, value: FormState[K]) => void;
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

export default function PromptsPage({ form, setForm }: PromptsPageProps) {
  const { t } = useTranslation();
  const [focusedField, setFocusedField] = useState<string | null>(null);

  useEffect(() => {
    if (!promptsAreEmpty(form.prompts)) {
      return;
    }
    invoke<PromptTemplates>("get_default_prompts")
      .then((defaults) => {
        if (promptsAreEmpty(form.prompts)) {
          setForm("prompts", defaults);
        }
      })
      .catch(() => undefined);
  }, []);

  const baseStyle: React.CSSProperties = {
    padding: "0.55rem 0.75rem",
    border: "1px solid var(--border)",
    borderRadius: "var(--radius)",
    background: "var(--input-bg)",
    color: "var(--text)",
    fontSize: "0.95rem",
    outline: "none",
    minHeight: "120px",
    resize: "vertical",
    fontFamily: "inherit",
    lineHeight: 1.5,
  };

  const focusStyle: React.CSSProperties = {
    borderColor: "var(--accent)",
  };

  const updatePrompt = <K extends keyof PromptTemplates>(
    key: K,
    value: PromptTemplates[K]
  ) => {
    setForm("prompts", { ...form.prompts, [key]: value });
  };

  const handleReset = async () => {
    setForm("system_prompt", "");
    try {
      const defaults = await invoke<PromptTemplates>("get_default_prompts");
      setForm("prompts", defaults);
    } catch {
      setForm("prompts", { ...emptyPrompts });
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
          value={form.system_prompt}
          onChange={(e) => setForm("system_prompt", e.target.value)}
          onFocus={() => setFocusedField("system_prompt")}
          onBlur={() => setFocusedField(null)}
          style={
            focusedField === "system_prompt"
              ? { ...baseStyle, ...focusStyle }
              : baseStyle
          }
          placeholder={t("system_prompt_placeholder")}
        />
      </label>

      {fields.map(({ key, labelKey }) => (
        <label key={key}>
          {t(labelKey)}
          <textarea
            value={form.prompts[key]}
            onChange={(e) => updatePrompt(key, e.target.value)}
            onFocus={() => setFocusedField(key)}
            onBlur={() => setFocusedField(null)}
            style={
              focusedField === key
                ? { ...baseStyle, ...focusStyle }
                : baseStyle
            }
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
