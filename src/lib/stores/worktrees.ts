/**
 * Worktrees store — worktree listing, creation, and removal.
 *
 * Wraps the IPC commands for worktree management and exposes
 * reactive stores for the worktree list and loading state.
 */

import { writable } from "svelte/store";
import { listWorktrees, createWorktree, removeWorktree } from "$lib/api/tauri";
import type { WorktreeInfo } from "$lib/types";

/** All worktrees for the active repository (including main). */
export const worktrees = writable<WorktreeInfo[]>([]);

/** True while a worktree list refresh is in progress. */
export const worktreeLoading = writable(false);

/** Fetch worktrees from the backend and update the store. */
export async function refreshWorktrees() {
  worktreeLoading.set(true);
  try {
    const list = await listWorktrees();
    worktrees.set(list);
  } catch {
    worktrees.set([]);
  } finally {
    worktreeLoading.set(false);
  }
}

/** Create a new linked worktree and refresh the list. */
export async function addWorktree(path: string, branch: string, createBranch: boolean) {
  await createWorktree(path, branch, createBranch);
  await refreshWorktrees();
}

/** Remove a linked worktree and refresh the list. */
export async function deleteWorktree(path: string, force: boolean) {
  await removeWorktree(path, force);
  await refreshWorktrees();
}

/** Reset worktree state. Called on repo switch. */
export function clearWorktreeState() {
  worktrees.set([]);
  worktreeLoading.set(false);
}
