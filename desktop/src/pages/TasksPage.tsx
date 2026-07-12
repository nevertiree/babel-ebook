import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { confirm } from "@tauri-apps/plugin-dialog";
import type { Page, QueueState, Task } from "../types";
import EmptyStateIcon from "../components/EmptyStateIcon";
import TrashIcon from "../components/TrashIcon";

interface TasksPageProps {
  queue: QueueState;
  onRemove: (ids: string[]) => Promise<void>;
  onRetry: (ids: string[]) => Promise<void>;
  onCancel: (ids: string[]) => Promise<void>;
  onPauseTask: (id: string) => Promise<void>;
  onStart: () => Promise<void>;
  onPause: () => Promise<void>;
  onReorder: (ids: string[]) => Promise<void>;
  onNavigate: (page: Page) => void;
}

function formatPath(path: string) {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

function formatDateTime(timestamp: number, t: (key: string) => string) {
  const date = new Date(timestamp * 1000);
  const today = new Date();
  const isToday =
    date.getFullYear() === today.getFullYear() &&
    date.getMonth() === today.getMonth() &&
    date.getDate() === today.getDate();
  const timeStr = date.toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });
  if (isToday) {
    return `${t("task_time_today")} ${timeStr}`;
  }
  const dateStr = date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
  return `${dateStr} ${timeStr}`;
}

function formatDuration(seconds: number, t: (key: string) => string) {
  if (seconds < 60) {
    return `${seconds}${t("task_time_seconds")}`;
  }
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  if (mins < 60) {
    return `${mins}${t("task_time_minutes")}${secs > 0 ? ` ${secs}${t("task_time_seconds")}` : ""}`;
  }
  const hours = Math.floor(mins / 60);
  const remainingMins = mins % 60;
  return `${hours}${t("task_time_hours")}${remainingMins > 0 ? ` ${remainingMins}${t("task_time_minutes")}` : ""}`;
}

function getElapsedSeconds(startedAt?: number, completedAt?: number): number {
  const end = completedAt ? completedAt * 1000 : Date.now();
  const start = startedAt ? startedAt * 1000 : end;
  return Math.max(0, Math.floor((end - start) / 1000));
}

function getTimeInfo(task: Task, t: (key: string, options?: Record<string, unknown>) => string) {
  const statusKey = `task_status_${task.status}`;
  const statusLabel = t(statusKey);

  switch (task.status) {
    case "running": {
      const elapsed = getElapsedSeconds(task.started_at);
      return `${statusLabel} · ${t("task_time_running")} ${formatDuration(elapsed, t as (key: string) => string)}`;
    }
    case "completed": {
      const elapsed = getElapsedSeconds(task.started_at, task.completed_at);
      const completedTime = task.completed_at
        ? formatDateTime(task.completed_at, t as (key: string) => string)
        : "";
      return `${statusLabel} · ${t("task_time_elapsed")} ${formatDuration(elapsed, t as (key: string) => string)}${completedTime ? ` · ${completedTime}` : ""}`;
    }
    case "failed":
    case "cancelled": {
      const elapsed = getElapsedSeconds(task.started_at, task.completed_at);
      return `${statusLabel} · ${t("task_time_elapsed")} ${formatDuration(elapsed, t as (key: string) => string)}`;
    }
    case "paused": {
      const elapsed = getElapsedSeconds(task.started_at, task.completed_at);
      return `${statusLabel} · ${t("task_time_elapsed")} ${formatDuration(elapsed, t as (key: string) => string)}`;
    }
    case "pending":
    default: {
      const createdTime = formatDateTime(task.created_at, t as (key: string) => string);
      return `${statusLabel} · ${t("task_time_created")} ${createdTime}`;
    }
  }
}

function getTaskMessage(task: Task) {
  if (task.status === "failed" && task.error) {
    return task.error;
  }
  return "";
}

export default function TasksPage({
  queue,
  onRemove,
  onRetry,
  onCancel,
  onPauseTask,
  onStart,
  onPause,
  onReorder,
  onNavigate,
}: TasksPageProps) {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [detailTask, setDetailTask] = useState<Task | null>(null);
  const [nowTick, setNowTick] = useState(Date.now());

  useEffect(() => {
    const timer = setInterval(() => setNowTick(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!detailTask) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setDetailTask(null);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [detailTask]);

  const statusClass = (status: Task["status"]) => `task-status task-status-${status}`;

  const selectableIds = useMemo(
    () => queue.tasks.filter((t) => t.status !== "running").map((t) => t.id),
    [queue.tasks]
  );
  const selectedList = useMemo(
    () => queue.tasks.filter((t) => selected.has(t.id)),
    [queue.tasks, selected]
  );

  const toggleSelection = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleSelectAll = () => {
    if (selected.size === selectableIds.length && selectableIds.length > 0) {
      setSelected(new Set());
    } else {
      setSelected(new Set(selectableIds));
    }
  };

  const moveTask = (index: number, direction: -1 | 1) => {
    const newIndex = index + direction;
    if (newIndex < 0 || newIndex >= queue.tasks.length) return;
    const next = [...queue.tasks];
    const [moved] = next.splice(index, 1);
    next.splice(newIndex, 0, moved);
    void onReorder(next.map((t) => t.id));
  };

  const handleBatchRemove = async () => {
    if (selectedList.length === 0) return;
    const confirmed = await confirm(
      t("confirm_remove_tasks", { count: selectedList.length }),
      { title: t("confirm_remove_tasks_title"), kind: "warning" }
    );
    if (!confirmed) return;
    await onRemove(selectedList.map((t) => t.id));
    setSelected(new Set());
  };

  const handleBatchRetry = async () => {
    if (selectedList.length === 0) return;
    await onRetry(selectedList.map((t) => t.id));
    setSelected(new Set());
  };

  const handleBatchCancel = async () => {
    if (selectedList.length === 0) return;
    await onCancel(selectedList.map((t) => t.id));
    setSelected(new Set());
  };

  const handleRemoveOne = async (task: Task) => {
    const confirmed = await confirm(
      t("confirm_remove_task", { file: formatPath(task.source_path) }),
      { title: t("confirm_remove_task_title"), kind: "warning" }
    );
    if (confirmed) await onRemove([task.id]);
  };

  const allSelected = selectableIds.length > 0 && selected.size === selectableIds.length;

  // Force re-render of running time every second without changing references.
  void nowTick;

  return (
    <div className="page tasks-page">
      <h2>{t("nav_tasks")}</h2>

      <div className="queue-controls">
        {queue.running ? (
          <button type="button" onClick={() => void onPause()} data-testid="pause-queue">
            {t("pause_queue")}
          </button>
        ) : (
          <button type="button" onClick={() => void onStart()} data-testid="start-queue">
            {t("start_queue")}
          </button>
        )}
      </div>

      {queue.tasks.length === 0 ? (
        <div className="empty-state">
          <EmptyStateIcon variant="task" className="empty-state-icon" />
          <p data-testid="queue-empty">{t("queue_empty")}</p>
          <button type="button" onClick={() => onNavigate("translate")}>
            {t("nav_translate")}
          </button>
        </div>
      ) : (
        <>
          <div className="batch-toolbar">
            <label className="checkbox batch-select-all">
              <input type="checkbox" checked={allSelected} onChange={toggleSelectAll} />
              {t("select_all")}
            </label>
            <div className="batch-actions">
              <button type="button" onClick={handleBatchCancel} disabled={selectedList.length === 0}>
                {t("cancel")}
              </button>
              <button type="button" onClick={handleBatchRetry} disabled={selectedList.length === 0}>
                {t("retry")}
              </button>
              <button
                type="button"
                className="danger"
                onClick={handleBatchRemove}
                disabled={selectedList.length === 0}
              >
                {t("remove")}
              </button>
            </div>
          </div>

          <ul className="task-list" data-testid="task-list">
            {queue.tasks.map((task, index) => {
              const message = getTaskMessage(task);
              return (
                <li key={task.id} className="task-item" data-testid="task-item">
                  <div className="task-row task-row-main">
                    <div className="task-primary">
                      {task.status !== "running" && (
                        <input
                          type="checkbox"
                          checked={selected.has(task.id)}
                          onChange={() => toggleSelection(task.id)}
                          aria-label={t("select_task", { file: formatPath(task.source_path) })}
                        />
                      )}
                      <span className={statusClass(task.status)}>
                        {t(`task_status_${task.status}`)}
                      </span>
                      <div className="task-title-block">
                        <span
                          className="task-title"
                          title={task.source_path}
                          data-testid="task-source"
                        >
                          {formatPath(task.source_path)}
                        </span>
                        <span
                          className="task-subtitle"
                          title={task.output_path}
                          data-testid="task-output"
                        >
                          → {formatPath(task.output_path)}
                        </span>
                      </div>
                    </div>
                    <div className="task-meta">
                      <span className="task-time" data-testid="task-time">
                        {getTimeInfo(task, t)}
                      </span>
                      {task.status !== "running" && (
                        <button
                          type="button"
                          className="task-delete"
                          onClick={() => void handleRemoveOne(task)}
                          data-testid="remove-task"
                          title={t("remove")}
                          aria-label={t("remove")}
                        >
                          <TrashIcon />
                        </button>
                      )}
                    </div>
                  </div>

                  <div className="task-row task-row-progress">
                    <div className="progress-bar" data-testid="task-progress-bar">
                      <div
                        className="progress-fill"
                        style={{ width: `${task.progress_percent}%` }}
                      />
                    </div>
                    <span className="task-percent">{task.progress_percent}%</span>
                  </div>

                  {message && (
                    <div className="task-row task-row-message">
                      <span className="inline-error">{message}</span>
                    </div>
                  )}

                  <div className="task-row task-row-actions">
                    {task.status === "pending" && (
                      <>
                        <button
                          type="button"
                          className="task-action"
                          onClick={() => moveTask(index, -1)}
                          disabled={index === 0}
                          title={t("move_up")}
                        >
                          ↑
                        </button>
                        <button
                          type="button"
                          className="task-action"
                          onClick={() => moveTask(index, 1)}
                          disabled={index === queue.tasks.length - 1}
                          title={t("move_down")}
                        >
                          ↓
                        </button>
                      </>
                    )}
                    {task.status === "running" && (
                      <button
                        type="button"
                        className="task-action"
                        onClick={() => void onPauseTask(task.id)}
                        data-testid="pause-task"
                      >
                        {t("pause")}
                      </button>
                    )}
                    {task.status === "paused" && (
                      <button
                        type="button"
                        className="task-action"
                        onClick={() => void onRetry([task.id])}
                        data-testid="resume-task"
                      >
                        {t("resume")}
                      </button>
                    )}
                    {(task.status === "pending" || task.status === "running") && (
                      <button
                        type="button"
                        className="task-action"
                        onClick={() => void onCancel([task.id])}
                        data-testid="cancel-task"
                      >
                        {t("cancel")}
                      </button>
                    )}
                    {(task.status === "failed" || task.status === "cancelled") && (
                      <button
                        type="button"
                        className="task-action"
                        onClick={() => void onRetry([task.id])}
                        data-testid="retry-task"
                      >
                        {t("retry")}
                      </button>
                    )}
                    {task.status === "failed" && task.error && (
                      <button
                        type="button"
                        className="task-action"
                        onClick={() => setDetailTask(task)}
                        data-testid="view-error-details"
                      >
                        {t("details")}
                      </button>
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        </>
      )}

      {detailTask && (
        <div
          className="modal-overlay"
          onClick={(e) => {
            if (e.target === e.currentTarget) setDetailTask(null);
          }}
          role="dialog"
          aria-modal="true"
          aria-labelledby="error-detail-title"
        >
          <div className="modal-dialog">
            <div className="modal-header">
              <h3 id="error-detail-title">{t("error_details")}</h3>
              <button
                type="button"
                className="modal-close"
                onClick={() => setDetailTask(null)}
                aria-label={t("close")}
              >
                ×
              </button>
            </div>
            <div className="modal-body">
              <p className="task-file" title={detailTask.source_path}>
                {formatPath(detailTask.source_path)}
              </p>
              <pre className="error-detail">{detailTask.error}</pre>
            </div>
            <div className="modal-footer">
              <button
                type="button"
                onClick={() => {
                  if (detailTask.error) {
                    void navigator.clipboard.writeText(detailTask.error);
                  }
                }}
                disabled={!detailTask.error}
              >
                {t("copy_error")}
              </button>
              <button type="button" onClick={() => setDetailTask(null)}>
                {t("close")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
