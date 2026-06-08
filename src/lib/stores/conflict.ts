/**
 * Conflict store — detects and manages in-progress merge, rebase,
 * cherry-pick, and revert operations.
 *
 * Refreshed on every `repo-changed` event. Drives the ConflictToolbar
 * banner which shows abort/continue buttons.
 */

import { writable, derived } from "svelte/store";
import type { ConflictStatus, ConflictStateValue } from "../types";
import {
  getConflictStatus as apiGetConflictStatus,
  abortOperation as apiAbortOperation,
  continueOperation as apiContinueOperation,
} from "../api/tauri";
import { runMutation } from "../api/runMutation";

const defaultStatus: ConflictStatus = {
  state: "none",
  conflicted_files: [],
  can_continue: false,
};

export const conflictStatus = writable<ConflictStatus>(defaultStatus);

export const isInConflict = derived(
  conflictStatus,
  ($s) => $s.state !== "none",
);

export const conflictStateLabel = derived(conflictStatus, ($s) => {
  const labels: Record<ConflictStateValue, string> = {
    none: "",
    merging: "MERGING",
    rebasing: "REBASING",
    cherry_picking: "CHERRY-PICKING",
    reverting: "REVERTING",
  };
  return labels[$s.state];
});

export async function refreshConflictStatus() {
  try {
    const status = await apiGetConflictStatus();
    conflictStatus.set(status);
  } catch {
    conflictStatus.set(defaultStatus);
  }
}

export async function abortOperation() {
  // Route through runMutation so a failure surfaces a toast + a recoverable
  // task-drawer entry (these are wired directly to buttons; a bare reject
  // would otherwise be silent). Refresh the banner either way.
  try {
    await runMutation({
      kind: "abort_operation",
      invoke: () => apiAbortOperation(),
      failureToastPrefix: "Abort failed",
    });
  } catch {
    // runMutation already surfaced the toast.
  }
  await refreshConflictStatus();
}

export async function continueOperation() {
  try {
    await runMutation({
      kind: "continue_operation",
      invoke: () => apiContinueOperation(),
      failureToastPrefix: "Continue failed",
    });
  } catch {
    // runMutation already surfaced the toast.
  }
  await refreshConflictStatus();
}
