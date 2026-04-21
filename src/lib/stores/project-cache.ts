/**
 * Project snapshot cache — loads/saves per-project state for instant UI.
 *
 * On project switch: load snapshot → apply to titlebar/badges.
 * After real data loads: build snapshot from current state → save to cache.
 */

import type { ProjectSnapshot, GraphViewport } from "$lib/types";
import * as api from "$lib/api/tauri";
import { fileStatuses } from "./changes";
import { get } from "svelte/store";
import { repoInfo } from "./repo";

/**
 * Persistent graph viewport slice.
 *
 * Stored inside `ProjectSnapshot.graph_viewport_cache` so a cold start
 * can paint the commit graph synchronously from disk. Size is roughly
 * 60 KB per project (300 rows × ~200 B JSON each).
 */
export interface GraphViewportCache {
  /** Last-seen 300-row viewport window (the `nodes` array as served by
   *  `get_graph_viewport`). No lane segments / merge curves — the fresh
   *  refresh repopulates those within one tick of paint. */
  nodes: GraphViewport["nodes"];
  /** Total commit count for the repo — used to render the scroll footer. */
  total_count: number;
  /** HEAD OID at the time the cache was written. Used as a coarse
   *  staleness check alongside `top_oid`. */
  head_oid: string;
  /** First visible commit in the cached window. Primary staleness
   *  check during reconciliation: if fresh `top_oid` matches, the
   *  cache is still accurate and no repaint is needed. */
  top_oid: string;
  /** Scroll offset captured at cache time — preserves vertical scroll
   *  position across cold starts. */
  offset: number;
  /** Epoch milliseconds when the cache was written. Entries older than
   *  `GRAPH_CACHE_TTL_MS` at load time are ignored and overwritten. */
  cached_at: number;
}

/** Max age before a cached graph slice is ignored at load time (7 days). */
export const GRAPH_CACHE_TTL_MS = 7 * 24 * 60 * 60 * 1000;

/**
 * Return `true` when the cache timestamp is within the TTL window.
 * Boundary (`cached_at === now - TTL`) is accepted so borderline entries
 * hydrate instead of being discarded on a refresh.
 */
export function isCacheFresh(cachedAt: number): boolean {
  return Date.now() - cachedAt <= GRAPH_CACHE_TTL_MS;
}

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
