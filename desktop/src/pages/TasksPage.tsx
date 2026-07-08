import type { QueueState } from "../types";

interface TasksPageProps {
  queue: QueueState;
  onRefresh: () => void;
  onRemove: (id: string) => void;
  onRetry: (id: string) => void;
  onCancel: (id: string) => void;
  onStart: () => void;
  onPause: () => void;
}

export default function TasksPage({
  queue,
  onRefresh,
  onRemove,
  onRetry,
  onCancel,
  onStart,
  onPause,
}: TasksPageProps) {
  // Stub for Task 5. Avoid unused-variable warnings by referencing props.
  void queue;
  void onRefresh;
  void onRemove;
  void onRetry;
  void onCancel;
  void onStart;
  void onPause;

  return <div className="page tasks-page">Tasks</div>;
}
