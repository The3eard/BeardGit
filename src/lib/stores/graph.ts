/**
 * Graph store — manages the virtual-scroll commit graph viewport, commit
 * selection, ref-badge file diffs, and per-file diff panels.
 *
 * The graph canvas fetches a window of 300 rows at a time via `loadViewport`.
 * Selection uses a "last-wins" async guard to handle rapid clicks.
 */

import { writable, get } from "svelte/store";
import type { GraphViewport, CommitInfo, CommitFileChange } from "../types";
import { getGraphViewport as apiGetGraphViewport, getCommitDetail as apiGetCommitDetail, getCommitFiles as apiGetCommitFiles, getDiffBetweenCommits, getCommitFileDiff, getUserIdentities as apiGetUserIdentities, getCommitRow as apiGetCommitRow, getFileAtCommit } from "../api/tauri";

/** Holds raw file content for the DiffEditor panel. */
export interface RawDiffContent {
  oldContent: string;
  newContent: string;
  filename: string;
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
  const vp = await apiGetGraphViewport(offset, VIEWPORT_SIZE);
  viewport.set(vp);
}

/** Re-fetch the current viewport window. Used after mutations that change refs. */
export async function reloadGraph(): Promise<void> {
  await loadViewport(get(graphOffset));
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
    // Resolve the parent OID for the "old" side.
    const commit = get(selectedCommit);
    const parentOid = commit?.parents?.[0] ?? null;

    const [oldContent, newContent] = await Promise.all([
      parentOid ? getFileAtCommit(parentOid, path).catch(() => "") : Promise.resolve(""),
      getFileAtCommit(oid, path).catch(() => ""),
    ]);

    fileDiffPanel.set({ oldContent, newContent, filename: path });
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

/** Full reset including viewport (used when no cache is available). */
export function resetGraphState() {
  clearGraphState();
  viewport.set(null);
  graphOffset.set(0);
}
