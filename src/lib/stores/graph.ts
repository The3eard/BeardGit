/**
 * Graph store — manages the virtual-scroll commit graph viewport, commit
 * selection, ref-badge file diffs, and per-file diff panels.
 *
 * The graph canvas fetches a window of 300 rows at a time via `loadViewport`.
 * Selection uses a "last-wins" async guard to handle rapid clicks.
 */

import { writable, get } from "svelte/store";
import type { GraphViewport, CommitInfo, CommitFileChange, FileDiff } from "../types";
import { getGraphViewport as apiGetGraphViewport, getCommitDetail as apiGetCommitDetail, getCommitFiles as apiGetCommitFiles, getDiffBetweenCommits, getCommitFileDiff, getUserIdentities as apiGetUserIdentities, getCommitRow as apiGetCommitRow } from "../api/tauri";

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
export const fileDiffPanel = writable<FileDiff | null>(null);
export const loadingFileDiff = writable(false);

const VIEWPORT_SIZE = 300;

export async function loadViewport(offset: number) {
  graphOffset.set(offset);
  const vp = await apiGetGraphViewport(offset, VIEWPORT_SIZE);
  viewport.set(vp);
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

export async function openFileDiff(oid: string, path: string) {
  loadingFileDiff.set(true);
  fileDiffPanel.set(null);
  try {
    const diffs = await getCommitFileDiff(oid, path);
    if (diffs.length > 0) {
      fileDiffPanel.set(diffs[0]);
    }
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
