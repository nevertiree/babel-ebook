import { useTranslation } from "react-i18next";
import type { QueueState, Task } from "../types";

interface TasksPageProps {
  queue: QueueState;
  onRemove: (id: string) => Promise<void>;
  onRetry: (id: string) => Promise<void>;
  onCancel: (id: string) => Promise<void>;
  onPauseTask: (id: string) => Promise<void>;
  onStart: () => Promise<void>;
  onPause: () => Promise<void>;
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
}: TasksPageProps) {
  const { t } = useTranslation();

  const statusClass = (status: Task["status"]) => `task-status task-status-${status}`;

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
        <p className="empty-state" data-testid="queue-empty">{t("queue_empty")}</p>
      ) : (
        <ul className="task-list" data-testid="task-list">
          {queue.tasks.map((task) => (
            <li key={task.id} className="task-item" data-testid="task-item">
              <div className="task-info">
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
                {task.status === "running" && (
                  <button type="button" onClick={() => void onPauseTask(task.id)} data-testid="pause-task">
                    {t("pause")}
                  </button>
                )}
                {(task.status === "pending" || task.status === "running") && (
                  <button type="button" onClick={() => void onCancel(task.id)} data-testid="cancel-task">
                    {t("cancel")}
                  </button>
                )}
                {(task.status === "failed" || task.status === "cancelled" || task.status === "paused") && (
                  <button type="button" onClick={() => void onRetry(task.id)} data-testid="retry-task">
                    {t("retry")}
                  </button>
                )}
                {task.status !== "running" && (
                  <button type="button" className="danger" onClick={() => void onRemove(task.id)} data-testid="remove-task">
                    {t("remove")}
                  </button>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
