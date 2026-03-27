/**
 * Repository store — manages the currently open repo's metadata and the
 * filesystem watcher that auto-refreshes statuses on disk changes.
 *
 * Used by `projects.ts` for multi-tab lifecycle and by `+page.svelte` for
 * initial load.
 */

import { writable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { RepoInfo, BranchInfo } from "../types";
import { openRepo as apiOpenRepo, getBranches as apiGetBranches, detectProject } from "../api/tauri";
import { checkStatus as checkProviderStatus } from "./provider";
import { refreshStatuses, refreshDiffs } from "./changes";
import { refreshConflictStatus } from "./conflict";
import { refreshUserEmails } from "./graph";

export const repoInfo = writable<RepoInfo | null>(null);
export const branches = writable<BranchInfo[]>([]);
export const isLoading = writable(false);
export const error = writable<string | null>(null);

let unlistenWatcher: UnlistenFn | null = null;
let watcherDebounceTimer: ReturnType<typeof setTimeout> | null = null;

/**
 * Register (or re-register) the repo-changed watcher listener.
 *
 * Debounced at 300 ms to batch rapid file changes and reduce IPC overhead.
 * Calling this again replaces any existing listener, so it is safe to call
 * on every project/repo switch.
 */
export async function registerWatcher() {
  if (unlistenWatcher) {
    unlistenWatcher();
  }
  unlistenWatcher = await listen("repo-changed", () => {
    if (watcherDebounceTimer) clearTimeout(watcherDebounceTimer);
    watcherDebounceTimer = setTimeout(async () => {
      // Refresh statuses first (fast, updates sidebar badge),
      // then diffs in parallel (updates changes view if open).
      await refreshStatuses();
      refreshDiffs();
      refreshConflictStatus();
    }, 300);
  });
}

export async function openRepo(path: string) {
  isLoading.set(true);
  error.set(null);
  try {
    const info = await apiOpenRepo(path);
    repoInfo.set(info);
    const branchList = await apiGetBranches();
    branches.set(branchList);
    // Re-detect project from the new repo's remote
    await detectProject();
    await checkProviderStatus();
    await refreshStatuses();
    await refreshConflictStatus();
    await refreshUserEmails();

    // Listen for filesystem changes from the watcher crate.
    await registerWatcher();
  } catch (e) {
    error.set(String(e));
  } finally {
    isLoading.set(false);
  }
}
