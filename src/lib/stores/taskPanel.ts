/**
 * Task panel store (infrastructure) — Rust `TaskManager` lifecycle bridge
 * for raw `task-started` / `task-output` / `task-completed` /
 * `task-failed` / `task-cancelled` events.
 *
 * Maintains three reactive surfaces used across the app:
 *
 *   1. **`tasks`** — every lifecycle snapshot reported by the
 *      `TaskManager`, upserted in place. Consumed by
 *      `AssetUploadProgress` and tests.
 *   2. **`taskOutput`** — per-`TaskId` stdout/stderr buffer, updated at
 *      rAF cadence to avoid GC thrash when a subprocess blasts lines.
 *      Consumed by `TaskDetailPanel.svelte` (the popover drill-down)
 *      and by the AI commit-message flow in `StagingArea.svelte` which
 *      harvests the final output to seed the commit box.
 *   3. **`selectedTaskId` / `selectedTask` / `selectedOutput`** — the
 *      "which task's output do I want to stream right now" cursor,
 *      shared with the AI commit flow so the user jumps straight to
 *      the live output as soon as `selectTask(id)` fires.
 *
 * Refresh of graph / branches / statuses after a Fetch/Pull/Push used
 * to be triggered from `task-completed` here — that side-effect is
 * now owned by the `project-mutated` event listener (see
 * `mutations.ts`), which emits one coalesced refresh per mutation.
 *
 * The legacy popover/panel UI that this module powered was retired in
 * favour of the unified `TasksPopover.svelte` (wired through
 * `src/lib/stores/tasks.ts`). What remains is the infrastructure the
 * new popover piggybacks on plus the AI commit-message flow that still
 * needs raw task output.
 */

import { writable, derived, get } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TaskInfo, TaskId, TaskOutputLine, TaskOutputEvent } from "../types";
import * as api from "../api/tauri";

/** Every task reported by the Rust `TaskManager`, upserted in place. */
export const tasks = writable<TaskInfo[]>([]);
/** Output lines keyed by task ID. Mutated in-place, cloned on rAF tick. */
export const taskOutput = writable<Map<TaskId, TaskOutputLine[]>>(new Map());
/**
 * Currently-selected `TaskId` for the output viewer.
 *
 * Set by `selectTask(id)` ahead of opening the popover/drilling down so
 * the detail panel can render the buffer without an extra round-trip.
 */
export const selectedTaskId = writable<TaskId | null>(null);

/** Tasks keyed by id for O(1) lookup in hot paths (e.g. upload progress). */
export const taskById = derived(tasks, ($tasks) => {
  const map = new Map<TaskId, TaskInfo>();
  for (const t of $tasks) map.set(t.id, t);
  return map;
});
export const selectedTask = derived(
  [tasks, selectedTaskId],
  ([$tasks, $id]) => ($id !== null ? $tasks.find((t) => t.id === $id) ?? null : null)
);
export const selectedOutput = derived(
  [taskOutput, selectedTaskId],
  ([$output, $id]) => ($id !== null ? $output.get($id) ?? [] : [])
);

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

  // Graph / branches / statuses refresh after a successful Fetch /
  // Pull / Push is handled by the `project-mutated` event dispatcher
  // in `mutations.ts`. We just mirror the lifecycle into the local
  // `tasks` store for surfaces like AssetUploadProgress.
  unlisteners.push(await listen<TaskInfo>("task-completed", updateTaskStatus));
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

// Actions
export async function cancelTask(taskId: TaskId): Promise<void> {
  await api.cancelTask(taskId);
}

/**
 * Select a task for output viewing and back-fill its buffer from the
 * backend when the local `taskOutput` map has no lines yet. Safe to
 * call multiple times — the fetch is a no-op when the buffer is
 * already populated.
 */
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
