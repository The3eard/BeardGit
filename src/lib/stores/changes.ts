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

import { writable, get } from "svelte/store";
import type { FileStatus, FileDiff, FileDiffStat } from "../types";
import {
  getFileStatuses as apiGetStatuses,
  stageFiles as apiStageFiles,
  unstageFiles as apiUnstageFiles,
  stageAll as apiStageAll,
  unstageAll as apiUnstageAll,
  createCommit as apiCreateCommit,
  amendCommit as apiAmendCommit,
  getDiffStatsWorkdir as apiDiffStatsWorkdir,
  getDiffStatsIndex as apiDiffStatsIndex,
  getDiffFile as apiDiffFile,
} from "../api/tauri";
import { runMutation } from "../api/runMutation";
import { clearChangesSelection } from "./changesSelection";

/** Per-file status list (staged and unstaged combined). */
export const fileStatuses = writable<FileStatus[]>([]);
/**
 * Per-file change stats (name/status + add/del counts, no hunks) for the
 * Changes list. Refreshed on every mutation — cheap because hunks are
 * never materialized here. The full hunks/lines diff of a single file is
 * fetched lazily into {@link openStagingDiff} when the user opens it.
 */
export const unstagedStats = writable<FileDiffStat[]>([]);
/** Staged (index-vs-HEAD) per-file stats. See {@link unstagedStats}. */
export const stagedStats = writable<FileDiffStat[]>([]);
/** The file whose full diff is open in the staging pane, or `null`. */
export const openStagingFile = writable<{ path: string; isStaged: boolean } | null>(null);
/** Full hunks/lines diff for {@link openStagingFile}, fetched on demand. */
export const openStagingDiff = writable<FileDiff | null>(null);
/** Current commit message draft. Cleared after successful commit. */
export const commitMessage = writable("");

/** Clear all changes state (e.g., on project switch). */
export function clearChangesState(): void {
  fileStatuses.set([]);
  unstagedStats.set([]);
  stagedStats.set([]);
  openStagingFile.set(null);
  openStagingDiff.set(null);
  // Checkbox selection is per-project — reset it when the repo changes.
  clearChangesSelection();
}

export async function refreshStatuses() {
  const statuses = await apiGetStatuses();
  fileStatuses.set(statuses);
}

/**
 * Refresh the Changes view after a mutation.
 *
 * Fetches only the lightweight per-file stats for both lists — never the
 * full hunk set — and re-fetches the full diff of the currently open file
 * (if any) so the diff pane stays live. Full hunks for any other file are
 * fetched lazily on selection via {@link loadStagingDiff}. This keeps the
 * mutation-refresh IPC payload tiny even when the working tree holds a
 * huge generated/minified file.
 */
export async function refreshDiffs() {
  const [workdir, index] = await Promise.all([
    apiDiffStatsWorkdir(),
    apiDiffStatsIndex(),
  ]);
  unstagedStats.set(workdir);
  stagedStats.set(index);
  const open = get(openStagingFile);
  if (open) await loadStagingDiff(open.path, open.isStaged);
}

/**
 * Open a file's full diff in the staging pane, fetching its hunks lazily.
 * Guards against a slower fetch clobbering a newer selection.
 */
export async function loadStagingDiff(path: string, isStaged: boolean): Promise<void> {
  openStagingFile.set({ path, isStaged });
  let diff: FileDiff | null = null;
  try {
    diff = await apiDiffFile(path, isStaged);
  } catch {
    diff = null;
  }
  const current = get(openStagingFile);
  if (current && current.path === path && current.isStaged === isStaged) {
    openStagingDiff.set(diff);
  }
}

/** Close the staging diff pane. */
export function closeStagingDiff(): void {
  openStagingFile.set(null);
  openStagingDiff.set(null);
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
