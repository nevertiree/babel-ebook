import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import type { LogEntry } from "../types";
import {
  assertProgressPayloadExhaustive,
  parseProgressPayload,
} from "../progress";
import { generateId } from "../utils";

interface LogState {
  entries: LogEntry[];
  total: number;
  completed: number;
}

interface UseLogStateReturn {
  logs: LogEntry[];
  clearLogs: () => void;
  appendError: (message: string) => void;
}

/**
 * Manage the translation log panel.
 *
 * Progress counters that used to live in mutable refs are now kept in React
 * state so log messages are derived from deterministic state updates.
 */
export function useLogState(): UseLogStateReturn {
  const { t } = useTranslation();
  const [logState, setLogState] = useState<LogState>({
    entries: [],
    total: 0,
    completed: 0,
  });

  useEffect(() => {
    const unlisten = listen<unknown>("translation_progress", (event) => {
      const payload = parseProgressPayload(event.payload);
      if (!payload) return;

      switch (payload.type) {
        case "Completed": {
          setLogState((prev) => ({
            ...prev,
            total: 0,
            completed: 0,
            entries: [
              ...prev.entries,
              {
                id: generateId(),
                timestamp: Date.now(),
                kind: "success",
                message: t("completed"),
              },
            ],
          }));
          break;
        }
        case "Started": {
          setLogState((prev) => ({
            total: payload.total,
            completed: 0,
            entries: [
              ...prev.entries,
              {
                id: generateId(),
                timestamp: Date.now(),
                kind: "info",
                message: t("log_started", { total: payload.total }),
              },
            ],
          }));
          break;
        }
        case "ChapterStarted": {
          setLogState((prev) => ({
            ...prev,
            entries: [
              ...prev.entries,
              {
                id: generateId(),
                timestamp: Date.now(),
                kind: "chapter",
                message: t("log_chapter_started", { href: payload.href }),
              },
            ],
          }));
          break;
        }
        case "ChapterFinished": {
          setLogState((prev) => {
            const message = t("log_chapter_finished", {
              href: payload.href,
              current: prev.completed,
              total: prev.total,
            });
            return {
              ...prev,
              completed: prev.completed + 1,
              entries: [
                ...prev.entries,
                {
                  id: generateId(),
                  timestamp: Date.now(),
                  kind: "chapter",
                  message,
                },
              ],
            };
          });
          break;
        }
        case "ChunkStarted": {
          setLogState((prev) => ({
            ...prev,
            entries: [
              ...prev.entries,
              {
                id: generateId(),
                timestamp: Date.now(),
                kind: "chapter",
                message: t("log_chunk_started", {
                  chunk_index: payload.chunk_index + 1,
                  chunk_total: payload.chunk_total,
                  href: payload.href,
                }),
              },
            ],
          }));
          break;
        }
        case "ChunkFinished": {
          setLogState((prev) => ({
            ...prev,
            entries: [
              ...prev.entries,
              {
                id: generateId(),
                timestamp: Date.now(),
                kind: "chapter",
                message: t("log_chunk_finished", {
                  chunk_index: payload.chunk_index + 1,
                  chunk_total: payload.chunk_total,
                  href: payload.href,
                }),
              },
            ],
          }));
          break;
        }
        case "Failed": {
          setLogState((prev) => ({
            ...prev,
            entries: [
              ...prev.entries,
              {
                id: generateId(),
                timestamp: Date.now(),
                kind: "error",
                message: t("log_chapter_failed", {
                  href: payload.href,
                  error: payload.error,
                }),
                details: payload.error,
              },
            ],
          }));
          break;
        }
        default: {
          assertProgressPayloadExhaustive(payload);
          break;
        }
      }
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [t]);

  const clearLogs = () => setLogState((prev) => ({ ...prev, entries: [] }));

  const appendError = (message: string) => {
    setLogState((prev) => ({
      ...prev,
      entries: [
        ...prev.entries,
        { id: generateId(), timestamp: Date.now(), kind: "error", message },
      ],
    }));
  };

  return { logs: logState.entries, clearLogs, appendError };
}
