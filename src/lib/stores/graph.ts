/**
 * Graph store — manages the virtual-scroll commit graph viewport, commit
 * selection, ref-badge file diffs, and per-file diff panels.
 *
 * The graph canvas fetches a window of 300 rows at a time via `loadViewport`.
 * Selection uses a "last-wins" async guard to handle rapid clicks.
 */

import { writable, get } from "svelte/store";
import type { GraphViewport, GraphViewOptions, CommitInfo, CommitFileChange } from "../types";
import { getGraphViewport as apiGetGraphViewport, getCommitDetail as apiGetCommitDetail, getCommitFiles as apiGetCommitFiles, getDiffBetweenCommits, getCommitFileDiff, getUserIdentities as apiGetUserIdentities, getCommitRow as apiGetCommitRow, getFileAtCommit, refreshGraphLayout as apiRefreshGraphLayout } from "../api/tauri";

/** Holds raw file content for the DiffEditor panel. */
export interface RawDiffContent {
  oldContent: string;
  newContent: string;
  filename: string;
  /**
   * When set, the DiffEditor renders this string as a placeholder
   * instead of trying to compute a diff. Used for binary blobs and
   * for blobs that exceeded the server-side size cap (5 MB).
   */
  placeholder?: string;
}

/** Format a byte count in MB with one decimal — used in too-large messages. */
function formatMB(bytes: number): string {
  return (bytes / (1024 * 1024)).toFixed(1) + " MB";
}

/**
 * Fetch both sides of a file diff and classify the result.
 *
 * Handles:
 * - Missing parent (added file or root commit) — `parentOid === null`
 *   ⇒ old side is treated as empty text.
 * - Binary blob on either side ⇒ returns a placeholder marker.
 * - Blob exceeding the server-side cap (`MAX_FILE_AT_COMMIT_BYTES`)
 *   ⇒ returns a placeholder marker with the byte size.
 * - Fetch error (404, permission, …) ⇒ that side is treated as empty.
 *
 * Shared between the graph's own `openFileDiff` and detail panels
 * (Tag, Stash, …) that show the same diff layout outside the main
 * graph viewport.
 */
type FetchSide =
  | { kind: "text"; data: string }
  | { kind: "binary" }
  | { kind: "too_large"; size: number };

const EMPTY_TEXT: FetchSide = { kind: "text", data: "" };

export async function fetchDiffSides(
  oid: string,
  parentOid: string | null,
  path: string,
): Promise<RawDiffContent> {
  const [oldRes, newRes]: [FetchSide, FetchSide] = await Promise.all([
    parentOid
      ? getFileAtCommit(parentOid, path).catch(() => EMPTY_TEXT)
      : Promise.resolve(EMPTY_TEXT),
    getFileAtCommit(oid, path).catch(() => EMPTY_TEXT),
  ]);

  const tooLarge: FetchSide | undefined = [oldRes, newRes].find(
    (r) => r.kind === "too_large",
  );
  if (tooLarge && tooLarge.kind === "too_large") {
    return {
      oldContent: "",
      newContent: "",
      filename: path,
      placeholder: `File too large to diff (${formatMB(tooLarge.size)}).`,
    };
  }
  if (oldRes.kind === "binary" || newRes.kind === "binary") {
    return {
      oldContent: "",
      newContent: "",
      filename: path,
      placeholder: "Binary file — no text diff available.",
    };
  }

  // After the early returns, both sides must be `kind: "text"`.
  const oldData = oldRes.kind === "text" ? oldRes.data : "";
  const newData = newRes.kind === "text" ? newRes.data : "";
  return { oldContent: oldData, newContent: newData, filename: path };
}

export const viewport = writable<GraphViewport | null>(null);
/** OID of the selected commit in the graph (drives CommitDetail). */
export const selectedOid = writable<string | null>(null);
/** Group ID of the selected lane segment (dims other branches). */
export const selectedGroup = writable<number | null>(null);
/** Lowercased identity strings for highlighting the current user's commits. */
export const userEmails = writable<string[]>([]);
export const selectedCommit = writable<CommitInfo | null>(null);
export const selectedCommitFiles = writable<CommitFileChange[]>([]);
export const graphOffset = writable(0);
/** Ref name of the currently expanded merge-badge diff. */
export const selectedRef = writable<string | null>(null);
/** Files changed between the merge commit's parents (for ref badge click). */
export const refFiles = writable<CommitFileChange[] | null>(null);
export const loadingRefFiles = writable(false);
export const fileDiffPanel = writable<RawDiffContent | null>(null);
export const loadingFileDiff = writable(false);

const VIEWPORT_SIZE = 300;

/**
 * Active graph view mode — first-parent simplification and/or a
 * branch-scoped walk. Session-only (not persisted); every viewport
 * and refresh call sends it, and the backend keys its layout cache
 * by it. `maxLanes` is wired separately by the canvas component
 * based on the available width.
 */
export const graphViewOptions = writable<GraphViewOptions>({});

/**
 * Update the view mode and reload. A structural change (first-parent
 * toggle or branch scope) invalidates the per-project viewport cache,
 * drops the selection — the selected commit may not exist in the new
 * view (e.g. a merged-branch commit under first-parent) — and reloads
 * from the top. A lane-budget change (`maxLanes`) is cosmetic: the
 * same commits remain, so the scroll offset and selection are kept.
 */
export async function setGraphViewOptions(
  patch: Partial<GraphViewOptions>,
): Promise<void> {
  const current = get(graphViewOptions);
  const next = { ...current, ...patch };
  const structuralChange =
    (next.firstParent ?? false) !== (current.firstParent ?? false) ||
    (next.branch ?? null) !== (current.branch ?? null);
  const laneChange = (next.maxLanes ?? null) !== (current.maxLanes ?? null);
  if (!structuralChange && !laneChange) return;

  graphViewOptions.set(next);
  viewportCache.clear();
  if (structuralChange) {
    clearGraphState();
    await loadViewport(0);
  } else {
    await loadViewport(get(graphOffset));
  }
}

/**
 * Per-project viewport cache for instant graph rendering on tab switch.
 * Keyed by project path. Stores the last viewport + scroll offset.
 */
const viewportCache = new Map<string, { vp: GraphViewport; offset: number }>();

/** Save current viewport to cache before switching away from a project. */
export function cacheViewport(projectPath: string): void {
  const vp = get(viewport);
  if (vp && vp.nodes.length > 0) {
    viewportCache.set(projectPath, { vp, offset: get(graphOffset) });
  }
}

/** Restore cached viewport for instant rendering. Returns true if cache hit. */
export function restoreCachedViewport(projectPath: string): boolean {
  const cached = viewportCache.get(projectPath);
  if (cached) {
    viewport.set(cached.vp);
    graphOffset.set(cached.offset);
    return true;
  }
  return false;
}

export async function loadViewport(offset: number) {
  graphOffset.set(offset);
  const vp = await apiGetGraphViewport(offset, VIEWPORT_SIZE, get(graphViewOptions));
  viewport.set(vp);
}

/** Re-fetch the current viewport window. Used after mutations that change refs. */
export async function reloadGraph(): Promise<void> {
  await loadViewport(get(graphOffset));
}

/**
 * Reconcile a fresh viewport against the currently-displayed one,
 * preserving the user's scroll anchor by OID.
 *
 * Two paths:
 * 1. `top_oid` matches (nothing above the old top changed) → atomic
 *    `viewport.set(fresh)`. Svelte reactivity no-ops if the
 *    serialised content is identical; when it differs (e.g. refs
 *    moved but commits didn't) the canvas repaints in place.
 * 2. `top_oid` differs → new commits likely landed above the old
 *    top. Locate the old anchor in the fresh window and bump
 *    `graphOffset` so the same commit stays at the top of the
 *    visible viewport. When the anchor is outside the fresh
 *    window we keep the current offset — the fresh refresh will
 *    be re-attempted at the new offset on the next mutation.
 *
 * Exported so unit tests can exercise both branches without mocking
 * the entire refresh chain.
 */
export function reconcileViewport(fresh: GraphViewport): void {
  const cached = get(viewport);
  const cachedTopOid = cached?.nodes[0]?.oid;
  const freshTopOid = fresh.nodes[0]?.oid;
  if (cached && cachedTopOid && freshTopOid && cachedTopOid !== freshTopOid) {
    const idx = fresh.nodes.findIndex((n) => n.oid === cachedTopOid);
    if (idx > 0) {
      graphOffset.update((o) => o + idx);
    } else if (idx < 0) {
      // The old anchor fell outside the fresh window entirely (e.g. a
      // pull rewrote history above it, or it was squashed away). Keeping
      // the stale offset would scroll to an unrelated row — snap to the
      // top so the user lands on the newest commits instead.
      graphOffset.set(0);
    }
  }
  viewport.set(fresh);
}

/**
 * Rebuild the server-side cached layout, then reload the current
 * viewport window so new commits / moved refs become visible.
 *
 * `get_graph_viewport` slices a layout that lives in `ProjectSlot.layout`
 * on the Rust side; that field is only populated by `open_repo` and
 * `switch_project`. After a commit, amend, push, pull, rebase, reset,
 * etc. the layout goes stale. This helper invokes
 * `refresh_graph_layout` to rebuild the slot layout (honouring the
 * persistent on-disk cache — a HEAD/refs change misses and falls through
 * to a live walk) and then re-fetches the viewport so the UI shows the
 * fresh state.
 *
 * Reconciliation (Phase 8.4): the fresh viewport is compared against the
 * currently-displayed one by `top_oid`. When they match we do a silent
 * replace (no flicker); when they differ we preserve the scroll anchor
 * by locating the old `top_oid` in the fresh window and adjusting
 * `graphOffset` accordingly. The final paint is always a single
 * `viewport.set` — no partial states.
 *
 * Failures are swallowed: if the Rust command fails (e.g. no active
 * project) we still fall through to a viewport fetch against whatever
 * layout is currently cached, which matches the pre-regression
 * behaviour for edge cases like detached repos.
 */
export async function refreshAndReloadGraph(): Promise<void> {
  try {
    await apiRefreshGraphLayout();
  } catch {
    // Best-effort — fall through to the viewport fetch below.
  }
  const offset = get(graphOffset);
  const fresh = await apiGetGraphViewport(offset, VIEWPORT_SIZE, get(graphViewOptions));
  reconcileViewport(fresh);
}

/** Select a commit by OID, fetching detail + files in parallel. Uses last-wins guard. */
export async function selectCommit(oid: string) {
  selectedOid.set(oid);
  selectedCommit.set(null);
  selectedCommitFiles.set([]);
  try {
    const [commit, files] = await Promise.all([
      apiGetCommitDetail(oid),
      apiGetCommitFiles(oid).catch(() => [] as CommitFileChange[]),
    ]);
    if (get(selectedOid) !== oid) return;
    selectedCommit.set(commit);
    selectedCommitFiles.set(files);
  } catch {
    if (get(selectedOid) !== oid) return;
    selectedCommitFiles.set([]);
  }
}

export async function loadRefFiles(parentOid: string, commitOid: string, ref: string) {
  selectedRef.set(ref);
  loadingRefFiles.set(true);
  refFiles.set(null);
  try {
    const files = await getDiffBetweenCommits(parentOid, commitOid);
    if (get(selectedRef) === ref) {
      refFiles.set(files);
    }
  } finally {
    if (get(selectedRef) === ref) {
      loadingRefFiles.set(false);
    }
  }
}

export function clearRefFiles() {
  selectedRef.set(null);
  refFiles.set(null);
  loadingRefFiles.set(false);
}

/**
 * Fetch raw old/new content for a file at a given commit and display
 * it in the DiffEditor panel.  The "old" side is the file at the
 * commit's first parent; the "new" side is the file at the commit itself.
 */
export async function openFileDiff(oid: string, path: string) {
  loadingFileDiff.set(true);
  fileDiffPanel.set(null);
  try {
    const commit = get(selectedCommit);
    const parentOid = commit?.parents?.[0] ?? null;
    fileDiffPanel.set(await fetchDiffSides(oid, parentOid, path));
  } finally {
    loadingFileDiff.set(false);
  }
}

export function closeFileDiff() {
  fileDiffPanel.set(null);
  loadingFileDiff.set(false);
}

export async function navigateToCommit(oid: string) {
  const row = await apiGetCommitRow(oid);
  if (row === null) return;
  const offset = Math.max(0, row - 15);
  await loadViewport(offset);
  await selectCommit(oid);
}

export async function refreshUserEmails() {
  try {
    const identities = await apiGetUserIdentities();
    userEmails.set(identities);
  } catch {
    userEmails.set([]);
  }
}

/** Move selection to the next commit (down) in the graph. */
export function graphNavigateDown(): void {
  const oid = get(selectedOid);
  const vp = get(viewport);
  if (!vp) return;
  const nodes = vp.nodes;
  if (!oid) {
    if (nodes.length > 0) {
      selectedOid.set(nodes[0].oid);
    }
    return;
  }
  const idx = nodes.findIndex((n) => n.oid === oid);
  if (idx >= 0 && idx < nodes.length - 1) {
    selectedOid.set(nodes[idx + 1].oid);
  }
}

/** Move selection to the previous commit (up) in the graph. */
export function graphNavigateUp(): void {
  const oid = get(selectedOid);
  const vp = get(viewport);
  if (!vp) return;
  const nodes = vp.nodes;
  if (!oid) {
    if (nodes.length > 0) {
      selectedOid.set(nodes[nodes.length - 1].oid);
    }
    return;
  }
  const idx = nodes.findIndex((n) => n.oid === oid);
  if (idx > 0) {
    selectedOid.set(nodes[idx - 1].oid);
  }
}

/** Jump selection to the first commit in the graph. */
export function graphNavigateFirst(): void {
  loadViewport(0).then(() => {
    const vp = get(viewport);
    if (vp && vp.nodes.length > 0) {
      selectedOid.set(vp.nodes[0].oid);
    }
  });
}

/** Jump selection to the last commit in the graph. */
export function graphNavigateLast(): void {
  const vp = get(viewport);
  if (!vp) return;
  const lastOffset = Math.max(0, vp.total_count - 50);
  loadViewport(lastOffset).then(() => {
    const newVp = get(viewport);
    if (newVp && newVp.nodes.length > 0) {
      selectedOid.set(newVp.nodes[newVp.nodes.length - 1].oid);
    }
  });
}

/** Reset all graph selection/detail state. Called on repo switch.
 *  The viewport is NOT cleared — it's either restored from cache
 *  or replaced when loadViewport() returns fresh data. */
export function clearGraphState() {
  selectedOid.set(null);
  selectedCommit.set(null);
  selectedCommitFiles.set([]);
  selectedRef.set(null);
  refFiles.set(null);
  loadingRefFiles.set(false);
  fileDiffPanel.set(null);
  loadingFileDiff.set(false);
  selectedGroup.set(null);
}

/**
 * Drop the project-specific part of the view mode. Called on project
 * switch: branch names don't carry across repos, while the
 * first-parent toggle is generic and survives.
 */
export function resetGraphViewScope() {
  graphViewOptions.update((o) => (o.branch ? { ...o, branch: undefined } : o));
}

/** Full reset including viewport (used when no cache is available). */
export function resetGraphState() {
  clearGraphState();
  viewport.set(null);
  graphOffset.set(0);
}
