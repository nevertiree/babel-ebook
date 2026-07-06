import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

const HOMEPAGE = "https://github.com/nevertiree/babel-ebook";

interface AboutPageProps {
  onOpenLegal?: () => void;
}

export default function AboutPage({ onOpenLegal }: AboutPageProps) {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string>("");

  useEffect(() => {
    invoke<string>("get_app_version")
      .then(setVersion)
      .catch(() => setVersion(t("about_version_unknown")));
  }, [t]);

  const handleOpen = async (e: React.MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    try {
      await openUrl(HOMEPAGE);
    } catch {
      // Non-blocking fallback.
    }
  };

  return (
    <div className="page settings-page about-page">
      <h2>{t("about_title")}</h2>

      <section className="about-section">
        <h3>{t("app_title")}</h3>
        <p>{t("subtitle")}</p>
        <p>
          {t("about_version")}: {version || t("about_version_unknown")}
        </p>
      </section>

      <section className="about-section">
        <h3>{t("about_description")}</h3>
        <p>{t("about_description_text")}</p>
      </section>

      <section className="about-section">
        <h3>{t("about_authors")}</h3>
        <p>{t("about_authors_text")}</p>
      </section>

      <section className="about-section">
        <h3>{t("about_license")}</h3>
        <p>{t("about_license_text")}</p>
      </section>

      <section className="about-section">
        <h3>{t("about_homepage")}</h3>
        <p>
          <a href={HOMEPAGE} onClick={handleOpen} target="_blank" rel="noreferrer">
            {HOMEPAGE}
          </a>
        </p>
      </section>

      <section className="about-section">
        <h3>{t("about_legal")}</h3>
        <p>
          {t("about_legal_text")}{" "}
          {onOpenLegal ? (
            <a
              href="#legal"
              onClick={(e) => {
                e.preventDefault();
                onOpenLegal();
              }}
            >
              {t("about_eula_link")}
            </a>
          ) : (
            <a href="/legal/EULA.md">{t("about_eula_link")}</a>
          )}
        </p>
      </section>

      <section className="about-section">
        <h3>{t("about_acknowledgments")}</h3>
        <p>{t("about_acknowledgments_text")}</p>
      </section>
    </div>
  );
}
