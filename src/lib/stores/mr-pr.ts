/**
 * MR/PR store — manages merge request / pull request state.
 *
 * Handles list fetching with filter tabs, detail loading, and polling
 * for updates on open MR/PRs.
 */

import { writable } from "svelte/store";
import type { MrPr, MrPrDetail, MrPrDiffFile, MrPrState } from "../types";
import {
  listMrPrs as apiList,
  getMrPrDetail as apiDetail,
  getMrPrDiff as apiDiff,
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

/** Fetch the MR/PR list with the current filter. */
export async function refreshMrPrList() {
  mrPrListLoading.set(true);
  try {
    let filter: string | undefined;
    const currentFilter = getCurrentFilter();
    if (currentFilter !== "all") {
      filter = currentFilter;
    }
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

/** Helper to get current filter value synchronously. */
function getCurrentFilter(): MrPrState | "all" {
  let val: MrPrState | "all" = "open";
  mrPrFilter.subscribe((v) => {
    val = v;
  })();
  return val;
}
