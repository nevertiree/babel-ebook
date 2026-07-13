import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { LogEntry, ProgressState, Task } from "../types";

interface RunningPanelProps {
  currentTask?: Task;
  progress?: ProgressState;
  logs: LogEntry[];
  onClearLogs: () => void;
}

export default function RunningPanel({
  currentTask,
  progress,
  logs,
  onClearLogs,
}: RunningPanelProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(true);

  const status = currentTask?.status ?? (currentTask === undefined ? "waiting" : "running");
  const percent = currentTask?.progress_percent ?? progress?.percent ?? 0;
  const message =
    currentTask?.status === "completed"
      ? t("completed")
      : currentTask?.status === "failed"
      ? currentTask.error || currentTask.message || t("task_status_failed")
      : currentTask?.message || progress?.message || t("waiting");
  const isRunning = currentTask?.status === "running";
  const hasContent = currentTask !== undefined || logs.length > 0;

  const statusLabel =
    currentTask?.status === "completed"
      ? t("task_status_completed")
      : currentTask?.status === "failed"
      ? t("task_status_failed")
      : currentTask?.status === "running"
      ? t("task_status_running")
      : t("waiting");

  const copyToClipboard = async () => {
    const text = logs
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
    <section
      className={`running-panel status-${status}`}
      data-testid="running-panel"
      data-status={status}
    >
      <button
        type="button"
        className="running-panel-header"
        onClick={() => setExpanded((prev) => !prev)}
        aria-expanded={expanded}
        data-testid="running-panel-toggle"
      >
        <span className="running-panel-title">{t("progress")}</span>
        <span className="running-panel-status">
          {isRunning ? `${percent}% — ${message}` : message}
        </span>
        <span className="running-panel-chevron" aria-hidden="true">
          {expanded ? "▾" : "▸"}
        </span>
      </button>

      {expanded ? (
        <div className="running-panel-body">
          <div className="running-panel-progress">
            <div className="progress-header">
              <span className="progress-status">{statusLabel}</span>
              <span>{percent}%</span>
            </div>
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${percent}%` }}
                data-testid="running-panel-progress-fill"
              />
            </div>
            <p className="progress-message" data-testid="running-panel-message">
              {message}
            </p>
            {currentTask?.error && (
              <p className="inline-error" data-testid="running-panel-error">
                {currentTask.error}
              </p>
            )}
          </div>

          <div className="running-panel-log">
            <div className="log-header">
              <h3>{t("log")}</h3>
              <div className="log-actions">
                <button
                  type="button"
                  onClick={copyToClipboard}
                  disabled={logs.length === 0}
                >
                  {t("copy_log")}
                </button>
                <button
                  type="button"
                  onClick={onClearLogs}
                  disabled={logs.length === 0}
                >
                  {t("clear_log")}
                </button>
              </div>
            </div>
            {logs.length === 0 ? (
              <p className="hint">{t("waiting")}</p>
            ) : (
              <ul className="log-list">
                {logs.map((entry) => (
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
              </ul>
            )}
          </div>
        </div>
      ) : (
        isRunning && (
          <div className="running-panel-compact">
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${percent}%` }}
              />
            </div>
            <span className="progress-message">{message}</span>
          </div>
        )
      )}

      {!hasContent && !expanded && (
        <p className="hint">{t("waiting")}</p>
      )}
    </section>
  );
}
