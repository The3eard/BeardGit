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
import { runMutation } from "$lib/api/runMutation";
import type { WorktreeInfo, AiWorktree, EnrichedWorktree } from "$lib/types";
import { fetchIntoStore } from "$lib/utils/store-helpers";

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
  await fetchIntoStore(
    worktrees,
    worktreeLoading,
    async () => {
      const [gitList, aiList] = await Promise.all([
        listWorktrees(),
        aiListWorktrees().catch(() => [] as AiWorktree[]),
      ]);
      return enrichWorktrees(gitList, aiList);
    },
    [],
  );
}

/** Create a new linked worktree. */
export async function addWorktree(path: string, branch: string, createBranch: boolean) {
  await runMutation({
    kind: "worktree_create",
    invoke: () => createWorktree(path, branch, createBranch),
    successToast: () => `Created worktree at ${path}`,
    failureToastPrefix: "Worktree create failed",
  });
  // Worktree list refresh is driven by the project-mutated event.
}

/** Remove a linked worktree. */
export async function deleteWorktree(path: string, force: boolean) {
  await runMutation({
    kind: "worktree_remove",
    invoke: () => removeWorktree(path, force),
    successToast: () => `Removed worktree ${path}`,
    failureToastPrefix: "Worktree remove failed",
  });
}

/** Cleanup an AI worktree (removes directory + branch). */
export async function cleanupAiWorktree(provider: string, worktreePath: string) {
  await runMutation({
    kind: "worktree_cleanup_ai",
    invoke: () => aiCleanupWorktree(provider, worktreePath),
    successToast: () => `Cleaned up AI worktree ${worktreePath}`,
    failureToastPrefix: "AI worktree cleanup failed",
  });
}

/** Reset worktree state. Called on repo switch. */
export function clearWorktreeState() {
  worktrees.set([]);
  worktreeLoading.set(false);
}
