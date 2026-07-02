/**
 * Project snapshot cache — loads/saves per-project state for instant UI.
 *
 * On project switch: load snapshot → apply to titlebar/badges.
 * After real data loads: build snapshot from current state → save to cache.
 *
 * TODO(spec 08): fold the in-memory role here into the RepoState container
 * (`stores/repo-state/`) — the disk snapshot persistence stays, but reads/
 * writes go through the container so a RepoState only ever knows its own
 * path (the "wrote under the wrong key" class of bug becomes impossible).
 * See `stores/branches.ts` for the migrated facade pattern.
 */

import type { ProjectSnapshot, GraphViewport } from "$lib/types";
import * as api from "$lib/api/tauri";
import { fileStatuses } from "./changes";
import { get, writable } from "svelte/store";
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
 * In-memory mirror of the on-disk snapshot keyed by project path,
 * exposed as a Svelte store so subscribers re-render whenever a
 * watcher-driven `saveCurrentSnapshot` rewrites an entry.
 *
 * `restoreCachedViewport` is synchronous by design — it has to paint
 * the graph before the canvas mounts — so it can only hydrate from
 * what's already in memory. `loadProjectSnapshot` primes this map on
 * every successful disk read, so subsequent tab switches to the same
 * project are instant.
 */
export const projectSnapshots = writable<Record<string, ProjectSnapshot>>({});

/** Expose in-memory hydration to tests and direct warm-up paths. */
export function hydrateSnapshotCache(path: string, snap: ProjectSnapshot): void {
  projectSnapshots.update((m) => ({ ...m, [path]: snap }));
}

/** Test seam — reset the in-memory cache so tests don't leak across files. */
export function _clearSnapshotCacheForTests(): void {
  projectSnapshots.set({});
}

/**
 * Look up a cached snapshot synchronously from the in-memory mirror.
 * Returns `null` when the project has never been loaded in this
 * session; callers that need disk I/O should use `loadProjectSnapshot`.
 */
export function getCachedSnapshot(path: string): ProjectSnapshot | null {
  return get(projectSnapshots)[path] ?? null;
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
  const snap = get(projectSnapshots)[projectPath];
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
 * Get a snapshot for any project. Reads the persisted on-disk cache
 * for `path` (fast path); never falls back to live status fetched
 * from the *active* project's stores — that's the bug that used to
 * pin BeardGit's status under beardgit_gh_tests's key.
 *
 * Returns `null` for projects that have never been activated and
 * have no cache file. Callers that want fresh data for a non-active
 * project should use [`refreshProjectSnapshot`].
 */
export async function getSnapshotForHover(path: string): Promise<ProjectSnapshot | null> {
  return await loadProjectSnapshot(path);
}

/**
 * Force-refresh a project's snapshot via the `compute_project_snapshot`
 * Tauri command, which opens a temp repo handle at `path` on the Rust
 * side and reads its status without touching `AppState`. Updates both
 * the on-disk cache (server-side, via the command's persist step) and
 * the in-memory `projectSnapshots` store (client-side, here) so the
 * tab strip and tooltip both see fresh data.
 *
 * Used on tab mount for non-active projects to recover from any stale
 * cache (the previous broken fallback wrote active-project data under
 * inactive project keys; calling this once per non-active tab on
 * mount overwrites that).
 */
export async function refreshProjectSnapshot(path: string): Promise<ProjectSnapshot | null> {
  try {
    const snap = await api.computeProjectSnapshot(path);
    hydrateSnapshotCache(path, snap);
    return snap;
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
