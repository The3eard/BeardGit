/**
 * Worktrees store — worktree listing, creation, and removal.
 *
 * Wraps the IPC commands for worktree management and exposes
 * reactive stores for the worktree list and loading state.
 * Enriches git worktrees with AI provider data when available.
 */

import { writable } from "svelte/store";
import {
  listWorktrees,
  createWorktree,
  removeWorktree,
  aiListWorktrees,
  aiCleanupWorktree,
} from "$lib/api/tauri";
import type { WorktreeInfo, AiWorktree, EnrichedWorktree } from "$lib/types";

/** All worktrees for the active repository, enriched with AI data. */
export const worktrees = writable<EnrichedWorktree[]>([]);

/** True while a worktree list refresh is in progress. */
export const worktreeLoading = writable(false);

/**
 * Join git worktrees with AI worktree data.
 *
 * Matches on absolute path. Non-matching worktrees get null AI fields.
 * Sorted: current first, then AI active, then alphabetical by branch.
 */
export function enrichWorktrees(
  gitWorktrees: WorktreeInfo[],
  aiWorktrees: AiWorktree[],
): EnrichedWorktree[] {
  const aiMap = new Map(aiWorktrees.map((w) => [w.path, w]));

  const enriched: EnrichedWorktree[] = gitWorktrees.map((wt) => {
    const ai = aiMap.get(wt.path);
    return {
      ...wt,
      ai_provider: ai?.provider ?? null,
      ai_status: ai?.status ?? null,
      ai_session_id: ai?.session_id ?? null,
    };
  });

  return enriched.sort((a, b) => {
    if (a.is_main !== b.is_main) return a.is_main ? -1 : 1;
    if ((a.ai_status === "active") !== (b.ai_status === "active")) {
      return a.ai_status === "active" ? -1 : 1;
    }
    return (a.branch ?? "").localeCompare(b.branch ?? "");
  });
}

/** Fetch worktrees from both git and AI backends, merge, and update store. */
export async function refreshWorktrees() {
  worktreeLoading.set(true);
  try {
    const [gitList, aiList] = await Promise.all([
      listWorktrees(),
      aiListWorktrees().catch(() => [] as AiWorktree[]),
    ]);
    worktrees.set(enrichWorktrees(gitList, aiList));
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

/** Cleanup an AI worktree (removes directory + branch) and refresh. */
export async function cleanupAiWorktree(provider: string, worktreePath: string) {
  await aiCleanupWorktree(provider, worktreePath);
  await refreshWorktrees();
}

/** Reset worktree state. Called on repo switch. */
export function clearWorktreeState() {
  worktrees.set([]);
  worktreeLoading.set(false);
}
