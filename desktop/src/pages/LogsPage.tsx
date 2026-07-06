import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { LogEntry } from "../types";

interface LogsPageProps {
  entries: LogEntry[];
  onClear: () => void;
}

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleString();
}

export default function LogsPage({ entries, onClear }: LogsPageProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => {
      const text = `${formatTime(e.timestamp)} ${e.message} ${e.details ?? ""}`.toLowerCase();
      return text.includes(q);
    });
  }, [entries, query]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [filtered]);

  const copyToClipboard = async () => {
    const text = entries
      .map(
        (e) =>
          `[${formatTime(e.timestamp)}] ${e.message}` +
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
    <div className="page logs-page">
      <div className="logs-header">
        <h2>{t("logs_title")}</h2>
        <span className="logs-count">{t("log_count", { count: filtered.length })}</span>
      </div>

      <div className="logs-toolbar">
        <input
          type="search"
          className="logs-search"
          placeholder={t("search_logs")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="log-actions">
          <button type="button" onClick={copyToClipboard} disabled={entries.length === 0}>
            {t("copy_log")}
          </button>
          <button type="button" onClick={onClear} disabled={entries.length === 0}>
            {t("clear_log")}
          </button>
        </div>
      </div>

      {filtered.length === 0 ? (
        <p className="hint">{entries.length === 0 ? t("waiting") : t("no_search_results")}</p>
      ) : (
        <ul className="log-list">
          {filtered.map((entry) => (
            <li key={entry.id} className={`log-entry ${entry.kind}`}>
              <span className="log-timestamp">{formatTime(entry.timestamp)}</span>
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
    </div>
  );
}
