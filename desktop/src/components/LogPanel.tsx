import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { LogEntry } from "../types";

interface LogPanelProps {
  entries: LogEntry[];
  onClear: () => void;
}

export default function LogPanel({ entries, onClear }: LogPanelProps) {
  const { t } = useTranslation();
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [entries]);

  const copyToClipboard = async () => {
    const text = entries
      .map(
        (e) =>
          `[${new Date(e.timestamp).toLocaleTimeString()}] ${e.message}` +
          (e.details ? `\n${e.details}` : "")
      )
      .join("\n");
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // Fallback is best-effort.
    }
  };

  return (
    <section className="log" data-testid="log-panel">
      <div className="log-header">
        <h3>{t("log")}</h3>
        <div className="log-actions">
          <button
            type="button"
            onClick={copyToClipboard}
            disabled={entries.length === 0}
          >
            {t("copy_log")}
          </button>
          <button
            type="button"
            onClick={onClear}
            disabled={entries.length === 0}
          >
            {t("clear_log")}
          </button>
        </div>
      </div>
      {entries.length === 0 ? (
        <p className="hint">{t("waiting")}</p>
      ) : (
        <ul className="log-list">
          {entries.map((entry) => (
            <li key={entry.id} className={`log-entry ${entry.kind}`}>
              <span className="log-timestamp">
                {new Date(entry.timestamp).toLocaleTimeString()}
              </span>
              <span className="log-content">
                {entry.details ? (
                  <details>
                    <summary>{entry.message}</summary>
                    <pre>{entry.details}</pre>
                  </details>
                ) : (
                  entry.message
                )}
              </span>
            </li>
          ))}
          <div ref={bottomRef} />
        </ul>
      )}
    </section>
  );
}
