/**
 * Compare store — "compare any ref against any ref" (spec 10).
 *
 * Facades over the *active* repo's `CompareSlice` (per-repo state, like
 * `branches.ts`), plus the fetch/swap/mode-toggle logic that drives the
 * `CompareView`. All reads are backend calls that resolve arbitrary revspecs
 * (branch, tag, `HEAD`, SHA):
 *
 * - **three-dot** (default): file diff is `merge-base(A,B)..B` — "what B adds".
 * - **two-dot**: file diff is the direct `A..B` tree comparison.
 *
 * The commit list (`A..B`) and the ahead/behind counts are the same in both
 * modes; only the file diff's "from" endpoint changes. Nothing here mutates
 * the repo — the view is read-only, so there is no `runMutation`/MutationGuard.
 */

import { get } from "svelte/store";
import type { CommitInfo, CommitFileChange } from "../types";
import type { RawDiffContent } from "./graph";
import { fetchDiffSides } from "./graph";
import { getMergeBase, getCommitsBetween, getDiffBetweenCommits } from "../api/tauri";
import { activeField, getActiveRepoState } from "./repo-state";
import type { CompareMode } from "./repo-state/CompareSlice";
import { activeViewStore } from "./navigation";

export type { CompareMode } from "./repo-state/CompareSlice";

/** Page size for the ahead commit list (and the behind count probe). */
export const COMPARE_PAGE_LIMIT = 100;

// Facades over the active repo's CompareSlice.
export const compareRefA = activeField<string | null>((rs) => rs.compare.refA);
export const compareRefB = activeField<string | null>((rs) => rs.compare.refB);
export const compareMode = activeField<CompareMode>((rs) => rs.compare.mode);
export const compareMergeBase = activeField<string | null>((rs) => rs.compare.mergeBase);
export const compareCommits = activeField<CommitInfo[]>((rs) => rs.compare.commits);
export const compareBehindCount = activeField<number>((rs) => rs.compare.behindCount);
export const compareCommitsCapped = activeField<boolean>((rs) => rs.compare.commitsCapped);
export const compareLoadingMore = activeField<boolean>((rs) => rs.compare.loadingMore);
export const compareFiles = activeField<CommitFileChange[]>((rs) => rs.compare.files);
export const compareLoading = activeField<boolean>((rs) => rs.compare.loading);
export const compareError = activeField<string | null>((rs) => rs.compare.error);
export const compareSelectedFilePath = activeField<string | null>((rs) => rs.compare.selectedFilePath);
export const compareOpenDiff = activeField<RawDiffContent | null>((rs) => rs.compare.openDiff);
export const compareLoadingDiff = activeField<boolean>((rs) => rs.compare.loadingDiff);
export const compareDiffError = activeField<string | null>((rs) => rs.compare.diffError);

// Last-wins guards so a slow response for a stale ref pair / file can't
// clobber the newest one.
let compareRequestId = 0;
let diffRequestId = 0;

/** The "from" endpoint of the file diff for the current mode: the merge-base
 *  in three-dot mode (falling back to A for unrelated histories), else A. */
function diffFrom(a: string, mode: CompareMode, mergeBase: string | null): string {
  return mode === "three-dot" ? (mergeBase ?? a) : a;
}

/**
 * Run the full comparison for the current `refA`/`refB`/`mode`: resolve the
 * merge-base, then load the changed-file list, the ahead commit list, and the
 * behind count in parallel. No-op if either ref is unset.
 */
export async function runCompare(): Promise<void> {
  const a = get(compareRefA);
  const b = get(compareRefB);
  if (!a || !b) return;

  const requestId = ++compareRequestId;
  compareLoading.set(true);
  compareError.set(null);
  getActiveRepoState().compare.clearDiff();

  try {
    const mergeBase = await getMergeBase(a, b).catch(() => null);
    if (requestId !== compareRequestId) return;
    compareMergeBase.set(mergeBase);

    const from = diffFrom(a, get(compareMode), mergeBase);
    const [files, ahead, behind] = await Promise.all([
      getDiffBetweenCommits(from, b),
      getCommitsBetween(a, b, COMPARE_PAGE_LIMIT),
      getCommitsBetween(b, a, COMPARE_PAGE_LIMIT),
    ]);
    if (requestId !== compareRequestId) return;

    compareFiles.set(files);
    compareCommits.set(ahead);
    compareCommitsCapped.set(ahead.length >= COMPARE_PAGE_LIMIT);
    compareBehindCount.set(behind.length);
  } catch (e) {
    if (requestId !== compareRequestId) return;
    compareError.set(e instanceof Error ? e.message : String(e));
    compareFiles.set([]);
    compareCommits.set([]);
  } finally {
    if (requestId === compareRequestId) compareLoading.set(false);
  }
}

/**
 * Open the compare view for the given refs (either may be `null` so the user
 * fills in the missing side). Switches to the compare view and, when both
 * sides are set, kicks off the comparison.
 */
export function openCompare(a: string | null, b: string | null): void {
  const slice = getActiveRepoState().compare;
  slice.clear();
  slice.refA.set(a);
  slice.refB.set(b);
  activeViewStore.set("compare");
  if (a && b) void runCompare();
}

/** Set side A (base) and re-run if side B is present. */
export function setCompareRefA(a: string | null): Promise<void> {
  compareRefA.set(a);
  return a && get(compareRefB) ? runCompare() : Promise.resolve();
}

/** Set side B (compare) and re-run if side A is present. */
export function setCompareRefB(b: string | null): Promise<void> {
  compareRefB.set(b);
  return b && get(compareRefA) ? runCompare() : Promise.resolve();
}

/** Swap the two refs (ahead/behind flip) and re-run. */
export function swapCompareRefs(): Promise<void> {
  const a = get(compareRefA);
  const b = get(compareRefB);
  compareRefA.set(b);
  compareRefB.set(a);
  return a && b ? runCompare() : Promise.resolve();
}

/** Switch range semantics. Only the file diff changes between modes, so this
 *  re-runs the compare (the commit list/counts come back identical). */
export function setCompareMode(mode: CompareMode): Promise<void> {
  if (get(compareMode) === mode) return Promise.resolve();
  compareMode.set(mode);
  return get(compareRefA) && get(compareRefB) ? runCompare() : Promise.resolve();
}

/** Append the next page of ahead commits, resuming after the last-shown OID. */
export async function loadMoreCompareCommits(): Promise<void> {
  const a = get(compareRefA);
  const b = get(compareRefB);
  const current = get(compareCommits);
  if (!a || !b || current.length === 0 || get(compareLoadingMore)) return;

  compareLoadingMore.set(true);
  try {
    const anchor = current[current.length - 1].oid;
    const next = await getCommitsBetween(a, b, COMPARE_PAGE_LIMIT, anchor);
    // Guard: refs may have changed while paging.
    if (get(compareRefA) !== a || get(compareRefB) !== b) return;
    compareCommits.set([...get(compareCommits), ...next]);
    compareCommitsCapped.set(next.length >= COMPARE_PAGE_LIMIT);
  } finally {
    compareLoadingMore.set(false);
  }
}

/**
 * Load the per-file diff for `path` into the panel. Old side = the current
 * mode's "from" endpoint; new side = ref B. Reuses `fetchDiffSides`, so
 * binary/too-large blobs render the shared placeholder.
 */
export async function openCompareFileDiff(path: string): Promise<void> {
  const a = get(compareRefA);
  const b = get(compareRefB);
  if (!a || !b) return;

  const requestId = ++diffRequestId;
  compareSelectedFilePath.set(path);
  compareLoadingDiff.set(true);
  compareOpenDiff.set(null);
  compareDiffError.set(null);
  try {
    const from = diffFrom(a, get(compareMode), get(compareMergeBase));
    const diff = await fetchDiffSides(b, from, path);
    if (requestId !== diffRequestId) return;
    compareOpenDiff.set(diff);
  } catch (e) {
    if (requestId !== diffRequestId) return;
    compareDiffError.set(e instanceof Error ? e.message : String(e));
  } finally {
    if (requestId === diffRequestId) compareLoadingDiff.set(false);
  }
}

/** Close the per-file diff panel (keeps the compare selection). */
export function closeCompareFileDiff(): void {
  getActiveRepoState().compare.clearDiff();
}

/** Reset the active repo's compare state. */
export function clearCompare(): void {
  getActiveRepoState().compare.clear();
}
