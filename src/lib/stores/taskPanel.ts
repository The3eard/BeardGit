/**
 * Task panel store (legacy) — background task lifecycle, output streaming,
 * and UI state for the sidebar task popover + panel.
 *
 * Listens for Tauri events (`task-started`, `task-output`, `task-completed`,
 * `task-failed`, `task-cancelled`) and maintains a reactive list of tasks
 * and their output buffers. Output events are batched via `requestAnimationFrame`
 * to reduce GC pressure from rapid updates.
 *
 * After remote operations (Fetch/Pull) complete successfully, the graph,
 * branches, and file statuses are auto-refreshed.
 *
 * This module powers the existing `TaskList` / `TaskPopover` / `TaskPanel`
 * stack. The unified "Tasks drawer" ships a separate aggregator store at
 * `src/lib/stores/tasks.ts` that bridges this panel's state alongside AI
 * background runs and auto-update downloads into a single feed.
 */

import { writable, derived, get } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TaskInfo, TaskId, TaskOutputLine, TaskOutputEvent } from "../types";
import * as api from "../api/tauri";
import { loadViewport, graphOffset } from "./graph";
import { refreshStatuses } from "./changes";
import { getBranches as apiGetBranches } from "../api/tauri";
import { branches } from "./repo";

export const tasks = writable<TaskInfo[]>([]);
/** Output lines keyed by task ID. Mutated in-place, cloned on rAF tick. */
export const taskOutput = writable<Map<TaskId, TaskOutputLine[]>>(new Map());
export const selectedTaskId = writable<TaskId | null>(null);

/** Display mode for the task UI: hidden, floating popover, or full panel. */
export type TaskPanelMode = "closed" | "popover" | "panel";
export const panelMode = writable<TaskPanelMode>("closed");

// Derived
export const runningTasks = derived(tasks, ($tasks) =>
  $tasks.filter((t) => t.status.state === "running")
);
/** Tasks keyed by id for O(1) lookup in hot paths (e.g. upload progress). */
export const taskById = derived(tasks, ($tasks) => {
  const map = new Map<TaskId, TaskInfo>();
  for (const t of $tasks) map.set(t.id, t);
  return map;
});
export const hasRunningTasks = derived(runningTasks, ($running) => $running.length > 0);
export const selectedTask = derived(
  [tasks, selectedTaskId],
  ([$tasks, $id]) => ($id !== null ? $tasks.find((t) => t.id === $id) ?? null : null)
);
export const selectedOutput = derived(
  [taskOutput, selectedTaskId],
  ([$output, $id]) => ($id !== null ? $output.get($id) ?? [] : [])
);

/** Tasks sorted: running first, then by most recent start time (newest first). */
export const sortedTasks = derived(tasks, ($tasks) => {
  return [...$tasks].sort((a, b) => {
    // Running tasks always come first
    const aRunning = a.status.state === "running" ? 0 : 1;
    const bRunning = b.status.state === "running" ? 0 : 1;
    if (aRunning !== bRunning) return aRunning - bRunning;
    // Then sort by start time descending (most recent first)
    const aTime = a.started_at_ms ?? 0;
    const bTime = b.started_at_ms ?? 0;
    return bTime - aTime;
  });
});

/** True when there are any tasks in history (running, completed, or failed). */
export const hasHistory = derived(tasks, ($tasks) => $tasks.length > 0);

// Lifecycle
let unlisteners: UnlistenFn[] = [];
let outputRafPending = false;

export async function initTaskStore(): Promise<void> {
  const existing = await api.getTasks();
  tasks.set(existing);

  unlisteners.push(
    await listen<TaskInfo>("task-started", (event) => {
      tasks.update((list) => {
        const idx = list.findIndex((t) => t.id === event.payload.id);
        if (idx >= 0) {
          list[idx] = event.payload;
          return [...list];
        }
        return [...list, event.payload];
      });
    })
  );

  unlisteners.push(
    await listen<TaskOutputEvent>("task-output", (event) => {
      taskOutput.update((map) => {
        const lines = map.get(event.payload.task_id) ?? [];
        lines.push(event.payload.line);
        map.set(event.payload.task_id, lines);
        return map; // mutate in place, defer clone
      });
      if (!outputRafPending) {
        outputRafPending = true;
        requestAnimationFrame(() => {
          outputRafPending = false;
          taskOutput.update((map) => new Map(map));
        });
      }
    })
  );

  const updateTaskStatus = (event: { payload: TaskInfo }) => {
    tasks.update((list) => {
      const idx = list.findIndex((t) => t.id === event.payload.id);
      if (idx >= 0) {
        list[idx] = event.payload;
        return [...list];
      }
      return list;
    });
  };

  unlisteners.push(
    await listen<TaskInfo>("task-completed", (event) => {
      updateTaskStatus(event);
      // Auto-refresh graph after fetch or pull completes
      const label = event.payload.label;
      if (label.startsWith("Fetch") || label.startsWith("Pull")) {
        refreshAfterRemoteOp();
      }
    })
  );
  unlisteners.push(await listen<TaskInfo>("task-failed", updateTaskStatus));
  unlisteners.push(await listen<TaskInfo>("task-cancelled", updateTaskStatus));
}

/** Unregister all task event listeners (called on app teardown). */
export function cleanupTaskStore(): void {
  for (const unlisten of unlisteners) {
    unlisten();
  }
  unlisteners = [];
}

/** Refresh the graph, branches, and statuses after a remote operation completes. */
async function refreshAfterRemoteOp() {
  try {
    const offset = get(graphOffset);
    await loadViewport(offset);
    const branchList = await apiGetBranches();
    branches.set(branchList);
    await refreshStatuses();
  } catch {
    // Silently ignore refresh errors — the user can manually refresh
  }
}

// Actions
export async function cancelTask(taskId: TaskId): Promise<void> {
  await api.cancelTask(taskId);
}

export async function selectTask(taskId: TaskId): Promise<void> {
  selectedTaskId.set(taskId);

  // If we don't have output locally (e.g. task ran before listener started,
  // or events were missed), fetch it from the backend.
  const currentOutput = get(taskOutput);
  if (!currentOutput.has(taskId) || currentOutput.get(taskId)!.length === 0) {
    try {
      const lines = await api.getTaskOutput(taskId);
      if (lines.length > 0) {
        taskOutput.update((map) => {
          map.set(taskId, lines);
          return new Map(map);
        });
      }
    } catch {
      // Task might have been cleaned up — ignore
    }
  }
}

export function togglePopover(): void {
  panelMode.update((mode) => (mode === "closed" ? "popover" : "closed"));
}

export function expandPanel(): void {
  panelMode.set("panel");
}

export function collapsePanel(): void {
  panelMode.set("popover");
}

export function closePanel(): void {
  panelMode.set("closed");
}
