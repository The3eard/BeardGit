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

import { getErrorMessage } from "$lib/api/errors";
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
  // Capture the target slice up front so a late response lands in the repo it
  // belongs to, even if the user switches tabs mid-flight. The per-slice
  // `requestId` still cancels an older compare superseded by a newer one in the
  // SAME repo. (RepoState renders only the active slice, so writing back into
  // an inactive repo's slice is correct and invisible.)
  const slice = getActiveRepoState().compare;
  const a = get(slice.refA);
  const b = get(slice.refB);
  if (!a || !b) return;

  const requestId = ++slice.requestId;
  slice.loading.set(true);
  slice.error.set(null);
  slice.clearDiff();

  try {
    const mergeBase = await getMergeBase(a, b).catch(() => null);
    if (requestId !== slice.requestId) return;
    slice.mergeBase.set(mergeBase);

    const from = diffFrom(a, get(slice.mode), mergeBase);
    const [files, ahead, behind] = await Promise.all([
      getDiffBetweenCommits(from, b),
      getCommitsBetween(a, b, COMPARE_PAGE_LIMIT),
      getCommitsBetween(b, a, COMPARE_PAGE_LIMIT),
    ]);
    if (requestId !== slice.requestId) return;

    slice.files.set(files);
    slice.commits.set(ahead);
    slice.commitsCapped.set(ahead.length >= COMPARE_PAGE_LIMIT);
    slice.behindCount.set(behind.length);
  } catch (e) {
    if (requestId !== slice.requestId) return;
    slice.error.set(getErrorMessage(e));
    slice.files.set([]);
    slice.commits.set([]);
  } finally {
    if (requestId === slice.requestId) slice.loading.set(false);
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
  // Same capture-the-slice guard as runCompare: a late page response lands in
  // its own repo's slice, so a mid-flight tab switch can't append the page into
  // another repo that happens to share the same refA/refB.
  const slice = getActiveRepoState().compare;
  const a = get(slice.refA);
  const b = get(slice.refB);
  const current = get(slice.commits);
  if (!a || !b || current.length === 0 || get(slice.loadingMore)) return;

  slice.loadingMore.set(true);
  try {
    const anchor = current[current.length - 1].oid;
    const next = await getCommitsBetween(a, b, COMPARE_PAGE_LIMIT, anchor);
    // Guard: refs may have changed while paging.
    if (get(slice.refA) !== a || get(slice.refB) !== b) return;
    slice.commits.set([...get(slice.commits), ...next]);
    slice.commitsCapped.set(next.length >= COMPARE_PAGE_LIMIT);
  } finally {
    slice.loadingMore.set(false);
  }
}

/**
 * Load the per-file diff for `path` into the panel. Old side = the current
 * mode's "from" endpoint; new side = ref B. Reuses `fetchDiffSides`, so
 * binary/too-large blobs render the shared placeholder.
 */
export async function openCompareFileDiff(path: string): Promise<void> {
  // Same capture-the-slice guard as runCompare: a late diff response lands in
  // its own repo, and the per-slice `diffRequestId` cancels a superseded diff
  // request within that repo.
  const slice = getActiveRepoState().compare;
  const a = get(slice.refA);
  const b = get(slice.refB);
  if (!a || !b) return;

  const requestId = ++slice.diffRequestId;
  slice.selectedFilePath.set(path);
  slice.loadingDiff.set(true);
  slice.openDiff.set(null);
  slice.diffError.set(null);
  try {
    const from = diffFrom(a, get(slice.mode), get(slice.mergeBase));
    const diff = await fetchDiffSides(b, from, path);
    if (requestId !== slice.diffRequestId) return;
    slice.openDiff.set(diff);
  } catch (e) {
    if (requestId !== slice.diffRequestId) return;
    slice.diffError.set(getErrorMessage(e));
  } finally {
    if (requestId === slice.diffRequestId) slice.loadingDiff.set(false);
  }
}

/** Close the per-file diff panel (keeps the compare selection). */
export function closeCompareFileDiff(): void {
  getActiveRepoState().compare.clearDiff();
}
