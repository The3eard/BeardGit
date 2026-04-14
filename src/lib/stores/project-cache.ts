/**
 * Project snapshot cache — loads/saves per-project state for instant UI.
 *
 * On project switch: load snapshot → apply to titlebar/badges.
 * After real data loads: build snapshot from current state → save to cache.
 */

import type { ProjectSnapshot } from "$lib/types";
import * as api from "$lib/api/tauri";
import { fileStatuses } from "./changes";
import { get } from "svelte/store";
import { repoInfo } from "./repo";

/** Load a snapshot from cache. Returns null if not cached. */
export async function loadProjectSnapshot(path: string): Promise<ProjectSnapshot | null> {
  try {
    return await api.getProjectSnapshot(path);
  } catch {
    return null;
  }
}

/**
 * Get a snapshot for any project — tries cache first, falls back to
 * computing a live snapshot by opening a temporary repository.
 * Used by tab hover tooltips so they work even without a cached file.
 */
export async function getSnapshotForHover(path: string): Promise<ProjectSnapshot | null> {
  // Try cache first (fast file read)
  const cached = await loadProjectSnapshot(path);
  if (cached) return cached;

  // No cache — compute live from a temporary repo handle
  try {
    const summary = await api.getStatusSummary();
    const info = get(repoInfo);
    const statuses = get(fileStatuses);
    if (!info) return null;

    const snapshot: ProjectSnapshot = {
      path,
      head_branch: info.head_branch ?? null,
      ahead: summary.ahead,
      behind: summary.behind,
      staged: summary.staged,
      unstaged: summary.unstaged,
      untracked: summary.untracked,
      conflicted: summary.conflicted,
      stash_count: summary.stash_count,
      change_count: statuses.length,
    };
    // Save so next hover is instant
    api.saveProjectSnapshot(snapshot).catch(() => {});
    return snapshot;
  } catch {
    return null;
  }
}

/** Build a snapshot from the current store state and save it. */
export async function saveCurrentSnapshot(projectPath: string): Promise<void> {
  const info = get(repoInfo);
  const statuses = get(fileStatuses);
  if (!info) return;

  // Use getStatusSummary for ahead/behind/stash data
  let ahead = 0, behind = 0, stash_count = 0, conflicted = 0;
  let staged = 0, unstaged = 0, untracked = 0;
  try {
    const s = await api.getStatusSummary();
    ahead = s.ahead;
    behind = s.behind;
    stash_count = s.stash_count;
    conflicted = s.conflicted;
    staged = s.staged;
    unstaged = s.unstaged;
    untracked = s.untracked;
  } catch { /* use defaults */ }

  const snapshot: ProjectSnapshot = {
    path: projectPath,
    head_branch: info.head_branch ?? null,
    ahead,
    behind,
    staged,
    unstaged,
    untracked,
    conflicted,
    stash_count,
    change_count: statuses.length,
  };

  try {
    await api.saveProjectSnapshot(snapshot);
  } catch { /* non-critical */ }
}
