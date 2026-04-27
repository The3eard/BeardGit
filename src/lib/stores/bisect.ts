/**
 * Bisect workflow store — manages bisect session state and actions.
 *
 * Wraps the Tauri bisect commands, keeping a reactive `BisectState`
 * and log string synchronized after each operation.
 */

import { writable } from "svelte/store";
import * as api from "$lib/api/tauri";
import type { BisectState } from "$lib/types";

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

/** Run an automated bisect with a test command. */
export async function runAutoBisect(testCommand: string): Promise<string> {
  bisectLoading.set(true);
  try {
    const result = await api.bisectRunAuto(testCommand);
    await refreshBisectState();
    return result;
  } finally {
    bisectLoading.set(false);
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
}
