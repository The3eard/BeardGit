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
import { viewport, graphOffset } from "./graph";

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

/**
 * In-memory mirror of the on-disk snapshot keyed by project path.
 *
 * `restoreCachedViewport` is synchronous by design — it has to paint
 * the graph before the canvas mounts — so it can only hydrate from
 * what's already in memory. `loadProjectSnapshot` primes this map on
 * every successful disk read, so subsequent tab switches to the same
 * project are instant.
 */
const _memorySnapshotCache = new Map<string, ProjectSnapshot>();

/** Expose in-memory hydration to tests and direct warm-up paths. */
export function hydrateSnapshotCache(path: string, snap: ProjectSnapshot): void {
  _memorySnapshotCache.set(path, snap);
}

/** Test seam — reset the in-memory cache so tests don't leak across files. */
export function _clearSnapshotCacheForTests(): void {
  _memorySnapshotCache.clear();
}

/**
 * Look up a cached snapshot synchronously from the in-memory mirror.
 * Returns `null` when the project has never been loaded in this
 * session; callers that need disk I/O should use `loadProjectSnapshot`.
 */
export function getCachedSnapshot(path: string): ProjectSnapshot | null {
  return _memorySnapshotCache.get(path) ?? null;
}

/** Load a snapshot from cache. Returns null if not cached. */
export async function loadProjectSnapshot(path: string): Promise<ProjectSnapshot | null> {
  try {
    const snap = await api.getProjectSnapshot(path);
    if (snap) hydrateSnapshotCache(path, snap);
    return snap;
  } catch {
    return null;
  }
}

/**
 * Hydrate `viewport` + `graphOffset` synchronously from the in-memory
 * snapshot cache when a fresh-enough slice exists. Returns `true` when
 * a paint-worthy viewport was installed — i.e. the caller must NOT
 * clobber it with a spinner/skeleton.
 *
 * Callers must have primed the cache (via `loadProjectSnapshot` on a
 * prior activation or an explicit warm-up) — this function never
 * touches disk. The tab-switch path in `projects.ts` first asks the
 * in-memory tab cache (via `graph.ts#restoreCachedViewport`); only on
 * a miss does it fall through here to the disk-backed slice.
 *
 * The restored viewport has empty `lane_segments` / `merge_curves`
 * because the cache doesn't persist layout geometry — the skeleton +
 * node list still renders usefully until the fresh refresh arrives
 * (< 100 ms) and the reconciler swaps in the full geometry.
 */
export function restorePersistedViewport(projectPath: string): boolean {
  const snap = _memorySnapshotCache.get(projectPath);
  if (!snap?.graph_viewport_cache) return false;
  const cache = snap.graph_viewport_cache;
  if (!isCacheFresh(cache.cached_at)) return false;
  viewport.set({
    nodes: cache.nodes,
    lane_segments: [],
    merge_curves: [],
    total_count: cache.total_count,
    offset: cache.offset,
    visible_lane_count: 0,
    total_lane_count: 0,
    head_lane: null,
    has_more: false,
  });
  graphOffset.set(cache.offset);
  return true;
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

/**
 * Assemble a `graph_viewport_cache` slice from the current viewport
 * store, or return `null` when there's nothing worth persisting
 * (empty or absent viewport). Exported for unit tests so reconciliation
 * logic can assert shape parity without reaching into the store.
 */
export function buildGraphViewportCacheFromStores(
  headOid: string | null | undefined,
): NonNullable<ProjectSnapshot["graph_viewport_cache"]> | null {
  const vp = get(viewport);
  if (!vp || vp.nodes.length === 0) return null;
  const topOid = vp.nodes[0]?.oid ?? "";
  return {
    nodes: vp.nodes,
    total_count: vp.total_count,
    head_oid: headOid ?? "",
    top_oid: topOid,
    offset: get(graphOffset),
    cached_at: Date.now(),
  };
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

  const graph_viewport_cache = buildGraphViewportCacheFromStores(info.head_oid);

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
    graph_viewport_cache,
  };

  // Mirror into memory first so subsequent tab switches hydrate instantly
  // without racing the save RTT.
  hydrateSnapshotCache(projectPath, snapshot);

  try {
    await api.saveProjectSnapshot(snapshot);
  } catch { /* non-critical */ }
}
