/**
 * Bisect workflow store — manages bisect session state and actions.
 *
 * Wraps the Tauri bisect commands, keeping a reactive `BisectState`
 * and log string synchronized after each operation.
 */

import { get, writable } from "svelte/store";
import * as api from "$lib/api/tauri";
import { tasks, cancelTask } from "$lib/stores/taskPanel";
import type { BisectState, TaskId } from "$lib/types";

/** Reactive bisect session state. */
export const bisectState = writable<BisectState>({
  active: false,
  current_commit: null,
  steps_remaining: null,
  good_commits: [],
  bad_commits: [],
});

/** Raw bisect log output. */
export const bisectLog = writable<string>("");

/** True while an auto-bisect run is in progress. */
export const bisectLoading = writable(false);

/**
 * TaskId of the in-flight `git bisect run` background task, or `null`
 * when no auto-bisect is running. Lets the UI cancel a runaway test
 * command via {@link cancelAutoBisect}.
 */
export const bisectTaskId = writable<TaskId | null>(null);

/**
 * Resolve once the given background task reaches a terminal state
 * (completed / failed / cancelled), tracked via the raw task lifecycle
 * events already mirrored into `taskPanel`'s `tasks` store.
 */
function waitForTaskTerminal(taskId: TaskId): Promise<void> {
  return new Promise((resolve) => {
    let resolved = false;
    let unsubscribe: () => void = () => {};
    unsubscribe = tasks.subscribe(($tasks) => {
      const state = $tasks.find((t) => t.id === taskId)?.status.state;
      if (state === "completed" || state === "failed" || state === "cancelled") {
        resolved = true;
        resolve();
        unsubscribe();
      }
    });
    // If the terminal state was already present on the initial (synchronous)
    // subscribe tick, `unsubscribe` was still the no-op above — tear down now.
    if (resolved) unsubscribe();
  });
}

/** Fetch and update the current bisect state from the backend. */
export async function refreshBisectState(): Promise<void> {
  const state = await api.bisectGetState();
  bisectState.set(state);
  if (state.active) {
    const log = await api.bisectGetLog();
    bisectLog.set(log);
  }
}

/** Start a bisect session, optionally providing bad/good commits. */
export async function startBisect(
  bad?: string,
  good?: string,
): Promise<string> {
  const result = await api.bisectStart(bad, good);
  await refreshBisectState();
  return result;
}

/** Mark a commit (or current HEAD) as good. */
export async function markGood(commit?: string): Promise<string> {
  const result = await api.bisectGood(commit);
  await refreshBisectState();
  return result;
}

/** Mark a commit (or current HEAD) as bad. */
export async function markBad(commit?: string): Promise<string> {
  const result = await api.bisectBad(commit);
  await refreshBisectState();
  return result;
}

/** Skip the current commit. */
export async function skipCommit(): Promise<string> {
  const result = await api.bisectSkip();
  await refreshBisectState();
  return result;
}

/** Reset (end) the bisect session. */
export async function resetBisect(): Promise<string> {
  const result = await api.bisectReset();
  bisectState.set({
    active: false,
    current_commit: null,
    steps_remaining: null,
    good_commits: [],
    bad_commits: [],
  });
  bisectLog.set("");
  return result;
}

/**
 * Run an automated bisect with a test command.
 *
 * `git bisect run` now executes as a cancellable background task
 * (returns a `TaskId` immediately). We keep the loading flag set until
 * the task reaches a terminal state, then refresh the session so the
 * panel shows where the bisect landed.
 */
export async function runAutoBisect(testCommand: string): Promise<TaskId> {
  bisectLoading.set(true);
  const taskId = await api.bisectRunAuto(testCommand);
  bisectTaskId.set(taskId);
  try {
    await waitForTaskTerminal(taskId);
    await refreshBisectState();
    return taskId;
  } finally {
    bisectTaskId.set(null);
    bisectLoading.set(false);
  }
}

/** Cancel the in-flight auto-bisect run, if any. */
export async function cancelAutoBisect(): Promise<void> {
  const taskId = get(bisectTaskId);
  if (taskId !== null) {
    await cancelTask(taskId);
  }
}

/** Clear all bisect state (e.g. on project switch). */
export function clearBisectState(): void {
  bisectState.set({
    active: false,
    current_commit: null,
    steps_remaining: null,
    good_commits: [],
    bad_commits: [],
  });
  bisectLog.set("");
  bisectLoading.set(false);
  bisectTaskId.set(null);
}
