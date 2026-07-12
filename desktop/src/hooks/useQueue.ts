import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { QueueState, Task } from "../types";

export interface UseQueueReturn {
  queue: QueueState;
  currentTask?: Task;
  runningTaskCount: number;
  refreshQueue: () => Promise<void>;
  removeTask: (ids: string[]) => Promise<void>;
  retryTask: (ids: string[]) => Promise<void>;
  cancelTask: (ids: string[]) => Promise<void>;
  reorderTasks: (ids: string[]) => Promise<void>;
  pauseTask: (id: string) => Promise<void>;
  resumeTask: (id: string) => Promise<void>;
  startQueue: () => Promise<void>;
  pauseQueue: () => Promise<void>;
}

/**
 * Manage the translation task queue as a single source of truth.
 *
 * Queue state is always fetched from the Tauri backend. Both per-task progress
 * events and coarse-grained queue changes only trigger a refresh, eliminating
 * races between local patches and backend snapshots.
 */
export function useQueue(): UseQueueReturn {
  const [queue, setQueue] = useState<QueueState>({
    tasks: [],
    running: false,
  });
  const [lastTaskId, setLastTaskId] = useState<string | undefined>();

  const refreshQueue = async () => {
    const state = await invoke<QueueState>("get_queue_state").catch(() => ({
      tasks: [],
      running: false,
    }));
    setQueue(state);
  };

  useEffect(() => {
    void (async () => {
      const initial = await invoke<QueueState>("get_queue_state").catch(() => ({
        tasks: [],
        running: false,
      }));
      setQueue(initial);
    })();
  }, []);

  useEffect(() => {
    const unlistenProgress = listen<unknown>("task_progress", () => {
      void refreshQueue();
    });

    const unlistenChanged = listen<unknown>("queue_state_changed", () => {
      void refreshQueue();
    });

    return () => {
      void unlistenProgress.then((f) => f());
      void unlistenChanged.then((f) => f());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (queue.current_task_id) {
      setLastTaskId(queue.current_task_id);
    }
  }, [queue.current_task_id]);

  const currentTask = useMemo(() => {
    if (queue.current_task_id) {
      return queue.tasks.find((t) => t.id === queue.current_task_id);
    }
    if (lastTaskId) {
      return queue.tasks.find((t) => t.id === lastTaskId);
    }
    return undefined;
  }, [queue, lastTaskId]);

  const runningTaskCount = useMemo(
    () => queue.tasks.filter((t) => t.status === "running").length,
    [queue]
  );

  const removeTask = async (ids: string[]) => {
    await Promise.all(ids.map((id) => invoke("remove_task", { id })));
    await refreshQueue();
  };

  const retryTask = async (ids: string[]) => {
    await Promise.all(ids.map((id) => invoke("retry_task", { id })));
    await refreshQueue();
  };

  const cancelTask = async (ids: string[]) => {
    await Promise.all(ids.map((id) => invoke("cancel_task", { id })));
    await refreshQueue();
  };

  const reorderTasks = async (ids: string[]) => {
    await invoke("reorder_tasks", { ids });
    await refreshQueue();
  };

  const pauseTask = async (id: string) => {
    await invoke("pause_task", { id });
    await refreshQueue();
  };

  const resumeTask = async (id: string) => {
    await invoke("resume_task", { id });
    await refreshQueue();
  };

  const startQueue = async () => {
    await invoke("start_queue");
    await refreshQueue();
  };

  const pauseQueue = async () => {
    await invoke("pause_queue");
    await refreshQueue();
  };

  return {
    queue,
    currentTask,
    runningTaskCount,
    refreshQueue,
    removeTask,
    retryTask,
    cancelTask,
    reorderTasks,
    pauseTask,
    resumeTask,
    startQueue,
    pauseQueue,
  };
}
