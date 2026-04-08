/**
 * MR/PR store — manages merge request / pull request state.
 *
 * Handles list fetching with filter tabs, detail loading, polling
 * for updates on open MR/PRs, and a derived store mapping branches
 * to open MR/PRs for graph badges.
 */

import { writable, derived, get } from "svelte/store";
import type { MrPr, MrPrDetail, MrPrDiffFile, MrPrState } from "../types";
import {
  listMrPrs as apiList,
  getMrPrDetail as apiDetail,
  getMrPrDiff as apiDiff,
  createMrPr as apiCreate,
  editMrPr as apiEdit,
  mergeMrPr as apiMerge,
  closeMrPr as apiClose,
  approveMrPr as apiApprove,
  requestChangesMrPr as apiRequestChanges,
  addMrPrComment as apiAddComment,
} from "../api/tauri";

/** Current filter tab: open, closed, merged, or all. */
export const mrPrFilter = writable<MrPrState | "all">("open");

/** List of MR/PRs matching the current filter. */
export const mrPrList = writable<MrPr[]>([]);

/** Whether the list is loading. */
export const mrPrListLoading = writable(false);

/** Currently selected MR/PR number. */
export const selectedMrPrNumber = writable<number | null>(null);

/** Detail of the selected MR/PR. */
export const mrPrDetail = writable<MrPrDetail | null>(null);

/** Changed files for the selected MR/PR. */
export const mrPrDiffFiles = writable<MrPrDiffFile[]>([]);

/** Whether the detail is loading. */
export const mrPrDetailLoading = writable(false);

/** Map of branch name -> MrPr for open MR/PRs (used by graph for badges). */
export const mrPrByBranch = derived(mrPrList, ($list) => {
  const map = new Map<string, MrPr>();
  for (const item of $list) {
    if (item.state === "open") {
      map.set(item.source_branch, item);
    }
  }
  return map;
});

/** Fetch the MR/PR list with the current filter. */
export async function refreshMrPrList() {
  mrPrListLoading.set(true);
  try {
    const currentFilter = get(mrPrFilter);
    const filter = currentFilter !== "all" ? currentFilter : undefined;
    const list = await apiList(filter, 50);
    mrPrList.set(list);
  } catch {
    mrPrList.set([]);
  } finally {
    mrPrListLoading.set(false);
  }
}

/** Load detail + diff for a specific MR/PR. */
export async function loadMrPrDetail(number: number) {
  selectedMrPrNumber.set(number);
  mrPrDetailLoading.set(true);
  try {
    const [detail, diff] = await Promise.all([apiDetail(number), apiDiff(number)]);
    mrPrDetail.set(detail);
    mrPrDiffFiles.set(diff);
  } catch {
    mrPrDetail.set(null);
    mrPrDiffFiles.set([]);
  } finally {
    mrPrDetailLoading.set(false);
  }
}

/** Clear detail state (e.g., when navigating away). */
export function clearMrPrDetail() {
  selectedMrPrNumber.set(null);
  mrPrDetail.set(null);
  mrPrDiffFiles.set([]);
}

// ---------------------------------------------------------------------------
// Write operations
// ---------------------------------------------------------------------------

/** Create a new MR/PR and refresh the list. */
export async function createMrPr(
  source: string, target: string, title: string, body: string,
  draft: boolean, labels: string[], reviewers: string[]
): Promise<MrPr> {
  const result = await apiCreate(source, target, title, body, draft, labels, reviewers);
  await refreshMrPrList();
  return result;
}

/** Edit a MR/PR and refresh the detail. */
export async function editMrPr(number: number, title?: string, body?: string): Promise<void> {
  await apiEdit(number, title, body);
  await loadMrPrDetail(number);
}

/** Merge a MR/PR and refresh the list. */
export async function mergeMrPr(number: number, strategy: string): Promise<void> {
  await apiMerge(number, strategy);
  clearMrPrDetail();
  await refreshMrPrList();
}

/** Close a MR/PR and refresh the list. */
export async function closeMrPr(number: number): Promise<void> {
  await apiClose(number);
  clearMrPrDetail();
  await refreshMrPrList();
}

// ---------------------------------------------------------------------------
// Review operations
// ---------------------------------------------------------------------------

/** Approve a MR/PR and refresh the detail. */
export async function approveMrPr(number: number): Promise<void> {
  await apiApprove(number);
  await loadMrPrDetail(number);
}

/** Request changes on a MR/PR and refresh the detail. */
export async function requestChangesMrPr(number: number, body: string): Promise<void> {
  await apiRequestChanges(number, body);
  await loadMrPrDetail(number);
}

/** Add a general comment to a MR/PR and refresh the detail. */
export async function addMrPrComment(number: number, body: string): Promise<void> {
  await apiAddComment(number, body);
  await loadMrPrDetail(number);
}
