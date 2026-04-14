/**
 * Changes store — staging area state for the commit workflow.
 *
 * Manages file statuses (staged/unstaged), diffs for the workdir and index,
 * and the commit message. After every mutation (stage, unstage, commit),
 * statuses and diffs are refreshed in parallel.
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
  getDiffWorkdir as apiDiffWorkdir,
  getDiffIndex as apiDiffIndex,
} from "../api/tauri";

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

export async function stageFiles(paths: string[]) {
  await apiStageFiles(paths);
  await Promise.all([refreshStatuses(), refreshDiffs()]);
}

export async function unstageFiles(paths: string[]) {
  await apiUnstageFiles(paths);
  await Promise.all([refreshStatuses(), refreshDiffs()]);
}

export async function stageAll() {
  await apiStageAll();
  await Promise.all([refreshStatuses(), refreshDiffs()]);
}

export async function unstageAll() {
  await apiUnstageAll();
  await Promise.all([refreshStatuses(), refreshDiffs()]);
}

/** Create a commit, clear the message, and refresh statuses + diffs. */
export async function commit(message: string) {
  await apiCreateCommit(message);
  commitMessage.set("");
  await Promise.all([refreshStatuses(), refreshDiffs()]);
}
