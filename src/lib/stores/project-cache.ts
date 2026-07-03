/**
 * Project snapshot cache — loads/saves per-project state for instant UI.
 *
 * On project switch: load snapshot → apply to titlebar/badges.
 * After real data loads: build snapshot from current state → save to cache.
 *
 * ── Migrated to the RepoState container (spec 08 step 5) ──────────────
 * The in-memory mirror used to be a central `Map<projectPath, ProjectSnapshot>`
 * (`projectSnapshots`) owned here — the exact "wrote under the wrong key"
 * hazard this module documents. It now lives per-repo in `RepoState.snapshot`,
 * reached by path through the container (`getRepoState` / `repoField`), so a
 * RepoState only ever knows its own path. The disk snapshot persistence
 * (`get_project_snapshot` / `save_project_snapshot`) stays here unchanged.
 */

import type { ProjectSnapshot, GraphViewport } from "$lib/types";
import * as api from "$lib/api/tauri";
import { fileStatuses } from "./changes";
import { get, type Readable } from "svelte/store";
import { repoInfo } from "./repo";
import { viewport, graphOffset } from "./graph";
import { getRepoState, repoField } from "./repo-state";

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
 * Reactive view of a specific repo's in-memory snapshot, resolved by path
 * through the container (`RepoState.snapshot`). Emits `null` when the repo has
 * no live state (closed tab) or hasn't been hydrated yet. The tab strip
 * subscribes to this per tab so an inactive tab's badges update reactively when
 * a watcher-driven `saveCurrentSnapshot` rewrites that repo's snapshot.
 */
export function snapshotStore(path: string): Readable<ProjectSnapshot | null> {
  return repoField(path, (rs) => rs.snapshot);
}

/**
 * Mirror a snapshot into the owning repo's slice. No-ops when the repo has no
 * live state (a closed/never-opened tab) — the disk read still returns the
 * value to the caller.
 */
export function hydrateSnapshotCache(path: string, snap: ProjectSnapshot): void {
  getRepoState(path)?.snapshot.set(snap);
}

/**
 * Look up a cached snapshot synchronously from the owning repo's slice.
 * Returns `null` when the project has no live state or has never been loaded in
 * this session; callers that need disk I/O should use `loadProjectSnapshot`.
 */
export function getCachedSnapshot(path: string): ProjectSnapshot | null {
  const rs = getRepoState(path);
  return rs ? get(rs.snapshot) : null;
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
 * touches disk. The tab-switch path in `projects.ts` first checks the
 * incoming repo's in-memory `GraphSlice` viewport; only when that is
 * empty (cold start) does it fall through here to the disk-backed slice.
 *
 * Writes go to the target repo's own `GraphSlice` (resolved by path), not the
 * active facade — so this can only ever install a viewport under its own key.
 *
 * The restored viewport has empty `lane_segments` / `merge_curves`
 * because the cache doesn't persist layout geometry — the skeleton +
 * node list still renders usefully until the fresh refresh arrives
 * (< 100 ms) and the reconciler swaps in the full geometry.
 */
export function restorePersistedViewport(projectPath: string): boolean {
  const rs = getRepoState(projectPath);
  const snap = rs ? get(rs.snapshot) : null;
  if (!rs || !snap?.graph_viewport_cache) return false;
  const cache = snap.graph_viewport_cache;
  if (!isCacheFresh(cache.cached_at)) return false;
  rs.graph.viewport.set({
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
  rs.graph.graphOffset.set(cache.offset);
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
 * the repo's in-memory `RepoState.snapshot` slice (client-side, here) so the
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
