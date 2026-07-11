import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { confirm } from "@tauri-apps/plugin-dialog";
import type { Page, QueueState, Task } from "../types";

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

  const allSelected = selectableIds.length > 0 && selected.size === selectableIds.length;

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
          <div className="empty-state-icon" aria-hidden="true">📋</div>
          <p data-testid="queue-empty">{t("queue_empty")}</p>
          <button type="button" onClick={() => onNavigate("translate")}>
            {t("nav_translate")}
          </button>
        </div>
      ) : (
        <>
          <div className="batch-toolbar">
            <label className="checkbox batch-select-all">
              <input
                type="checkbox"
                checked={allSelected}
                onChange={toggleSelectAll}
              />
              {t("select_all")}
            </label>
            <div className="batch-actions">
              <button type="button" onClick={handleBatchCancel} disabled={selectedList.length === 0}>
                {t("cancel")}
              </button>
              <button type="button" onClick={handleBatchRetry} disabled={selectedList.length === 0}>
                {t("retry")}
              </button>
              <button type="button" className="danger" onClick={handleBatchRemove} disabled={selectedList.length === 0}>
                {t("remove")}
              </button>
            </div>
          </div>

          <ul className="task-list" data-testid="task-list">
            {queue.tasks.map((task, index) => (
              <li key={task.id} className="task-item" data-testid="task-item">
                <div className="task-info">
                  {task.status !== "running" && (
                    <input
                      type="checkbox"
                      checked={selected.has(task.id)}
                      onChange={() => toggleSelection(task.id)}
                      aria-label={t("select_task", { file: formatPath(task.source_path) })}
                    />
                  )}
                  <span className={statusClass(task.status)}>{t(`task_status_${task.status}`)}</span>
                  <span className="task-file" title={task.source_path}>
                    {formatPath(task.source_path)}
                  </span>
                  <span className="task-file" title={task.output_path}>
                    → {formatPath(task.output_path)}
                  </span>
                </div>

                <div className="task-progress">
                  <div className="progress-bar">
                    <div
                      className="progress-fill"
                      style={{ width: `${task.progress_percent}%` }}
                    />
                  </div>
                  <span className="progress-message">{task.message}</span>
                  {task.error && <span className="inline-error">{task.error}</span>}
                </div>

                <div className="task-actions">
                  {task.status === "pending" && (
                    <>
                      <button
                        type="button"
                        onClick={() => moveTask(index, -1)}
                        disabled={index === 0}
                        title={t("move_up")}
                      >
                        ↑
                      </button>
                      <button
                        type="button"
                        onClick={() => moveTask(index, 1)}
                        disabled={index === queue.tasks.length - 1}
                        title={t("move_down")}
                      >
                        ↓
                      </button>
                    </>
                  )}
                  {task.status === "running" && (
                    <button type="button" onClick={() => void onPauseTask(task.id)} data-testid="pause-task">
                      {t("pause")}
                    </button>
                  )}
                  {(task.status === "pending" || task.status === "running") && (
                    <button type="button" onClick={() => void onCancel([task.id])} data-testid="cancel-task">
                      {t("cancel")}
                    </button>
                  )}
                  {(task.status === "failed" || task.status === "cancelled" || task.status === "paused") && (
                    <button type="button" onClick={() => void onRetry([task.id])} data-testid="retry-task">
                      {t("retry")}
                    </button>
                  )}
                  {task.status === "failed" && task.error && (
                    <button
                      type="button"
                      onClick={() => setDetailTask(task)}
                      data-testid="view-error-details"
                    >
                      {t("details")}
                    </button>
                  )}
                  {task.status !== "running" && (
                    <button
                      type="button"
                      className="danger"
                      onClick={async () => {
                        const confirmed = await confirm(
                          t("confirm_remove_task", { file: formatPath(task.source_path) }),
                          { title: t("confirm_remove_task_title"), kind: "warning" }
                        );
                        if (confirmed) await onRemove([task.id]);
                      }}
                      data-testid="remove-task"
                    >
                      {t("remove")}
                    </button>
                  )}
                </div>
              </li>
            ))}
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
