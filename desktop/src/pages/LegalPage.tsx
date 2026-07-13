import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface LegalPageProps {
  onBack: () => void;
}

export default function LegalPage({ onBack }: LegalPageProps) {
  const { t } = useTranslation();
  const [paragraphs, setParagraphs] = useState<string[]>([]);
  const [error, setError] = useState<string>("");

  useEffect(() => {
    let cancelled = false;
    fetch("/legal/LICENSE")
      .then((res) => {
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }
        return res.text();
      })
      .then((data) => {
        if (!cancelled) {
          // Split by blank lines, trim, and drop empty chunks.
          const chunks = data
            .split(/\n\s*\n/)
            .map((p) => p.trim())
            .filter((p) => p.length > 0);
          setParagraphs(chunks);
        }
      })
      .catch((err) => {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="page settings-page legal-page">
      <div className="legal-header">
        <button type="button" className="button-secondary" onClick={onBack}>
          {t("legal_back")}
        </button>
        <h2>{t("legal_title")}</h2>
      </div>

      {error ? (
        <p className="legal-error">{t("legal_load_error", { error })}</p>
      ) : paragraphs.length === 0 ? (
        <p className="hint">{t("legal_loading")}</p>
      ) : (
        <article className="legal-document">
          {paragraphs.map((p, index) => (
            <p key={index}>{p}</p>
          ))}
        </article>
      )}
    </div>
  );
}
