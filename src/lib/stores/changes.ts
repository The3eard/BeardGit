/**
 * Changes store — staging area state for the commit workflow.
 *
 * Manages file statuses (staged/unstaged), diffs for the workdir and
 * index, and the commit message.
 *
 * Mutations route through {@link runMutation} so toast + task policy
 * lives in one place; refresh side-effects come from the `project-
 * mutated` event dispatcher in `mutations.ts` — the per-call manual
 * `refreshStatuses()` / `refreshAndReloadGraph()` chains that used to
 * follow each mutating invoke are now handled by that listener.
 */

import { writable } from "svelte/store";
import type { FileStatus, FileDiff } from "../types";
import {
  getFileStatuses as apiGetStatuses,
  stageFiles as apiStageFiles,
  unstageFiles as apiUnstageFiles,
  stageAll as apiStageAll,
  unstageAll as apiUnstageAll,
  createCommit as apiCreateCommit,
  amendCommit as apiAmendCommit,
  getDiffWorkdir as apiDiffWorkdir,
  getDiffIndex as apiDiffIndex,
} from "../api/tauri";
import { runMutation } from "../api/runMutation";
import { clearChangesSelection } from "./changesSelection";

/** Per-file status list (staged and unstaged combined). */
export const fileStatuses = writable<FileStatus[]>([]);
/** Workdir-vs-index diffs for unstaged changes. */
export const unstagedDiffs = writable<FileDiff[]>([]);
/** Index-vs-HEAD diffs for staged changes. */
export const stagedDiffs = writable<FileDiff[]>([]);
/** Current commit message draft. Cleared after successful commit. */
export const commitMessage = writable("");

/** Clear all changes state (e.g., on project switch). */
export function clearChangesState(): void {
  fileStatuses.set([]);
  unstagedDiffs.set([]);
  stagedDiffs.set([]);
  // Checkbox selection is per-project — reset it when the repo changes.
  clearChangesSelection();
}

export async function refreshStatuses() {
  const statuses = await apiGetStatuses();
  fileStatuses.set(statuses);
}

export async function refreshDiffs() {
  const [workdir, index] = await Promise.all([apiDiffWorkdir(), apiDiffIndex()]);
  unstagedDiffs.set(workdir);
  stagedDiffs.set(index);
}

/** Truncate a commit message for the success-toast body. */
function truncate(msg: string, max: number): string {
  const firstLine = msg.split(/\r?\n/, 1)[0] ?? "";
  return firstLine.length > max ? `${firstLine.slice(0, max - 1)}…` : firstLine;
}

export async function stageFiles(paths: string[]) {
  await runMutation({
    kind: "stage",
    invoke: () => apiStageFiles(paths),
    failureToastPrefix: "Stage failed",
  });
}

export async function unstageFiles(paths: string[]) {
  await runMutation({
    kind: "unstage",
    invoke: () => apiUnstageFiles(paths),
    failureToastPrefix: "Unstage failed",
  });
}

export async function stageAll() {
  await runMutation({
    kind: "stage",
    invoke: () => apiStageAll(),
    failureToastPrefix: "Stage failed",
  });
}

export async function unstageAll() {
  await runMutation({
    kind: "unstage",
    invoke: () => apiUnstageAll(),
    failureToastPrefix: "Unstage failed",
  });
}

/**
 * Create a commit and clear the message draft.
 *
 * Refresh of statuses / diffs / graph is driven by the `project-
 * mutated` event emitted by the Rust-side commit command — see
 * `mutations.ts`.
 */
export async function commit(message: string) {
  await runMutation({
    kind: "commit",
    invoke: () => apiCreateCommit(message),
    successToast: () => `Committed — ${truncate(message, 60)}`,
    failureToastPrefix: "Commit failed",
  });
  commitMessage.set("");
}

/**
 * Amend the current HEAD commit.
 *
 * Same refresh story as {@link commit}: the mutation listener reloads
 * statuses + graph automatically.
 */
export async function amendCommit(message: string): Promise<void> {
  await runMutation({
    kind: "amend",
    invoke: () => apiAmendCommit(message),
    successToast: () => `Amended — ${truncate(message, 60)}`,
    failureToastPrefix: "Amend failed",
  });
}
