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
  await apiAbortOperation();
  await refreshConflictStatus();
}

export async function continueOperation() {
  await apiContinueOperation();
  await refreshConflictStatus();
}
