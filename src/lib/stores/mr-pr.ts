/**
 * MR/PR store — manages merge request / pull request state.
 *
 * Handles list fetching with filter tabs, detail loading, polling
 * for updates on open MR/PRs, and a derived store mapping branches
 * to open MR/PRs for graph badges.
 *
 * DIAGNOSIS 2026-04-21 — "PR #18 infinite loading spinner" bug
 * ------------------------------------------------------------
 * Symptom: opening PR #18 in the `beardgit_test` project leaves the
 * detail pane stuck on a loading spinner forever.
 *
 * Ruled out: the `loadMrPrDetail` function below (see ~line 97) does
 * call `mrPrDetailLoading.set(false)` inside a `finally` block, so
 * the classic "missing finally" state-leak bug is NOT the cause.
 *
 * Actual root cause: PR #18 has ~3.4k changed files. The backend
 * runs `gh api repos/{owner}/{repo}/pulls/18/files --paginate`,
 * which over a slow/congested network can take >60s and in practice
 * appears to hang indefinitely (the subprocess never returns).
 * Because neither the TS caller nor the Rust spawn has a timeout,
 * the awaited `apiDiff(number)` promise never settles — so the
 * `try`/`finally` never reaches `finally`, loading stays true, and
 * the spinner is forever.
 *
 * Fix path (upcoming phases):
 *  - TS side: wrap the detail+diff fetch in a 15 s `withTimeout`
 *    helper so the spinner always clears; on timeout, surface a
 *    toast and show the shared `ForgeDetailShell` error state with
 *    a retry button.
 *  - Rust side: add `wait_timeout(20s)` on the spawned `gh`/`glab`
 *    process and cap the diff payload so runaway outputs are
 *    truncated rather than streamed indefinitely.
 */

import { writable, derived, get } from "svelte/store";
import type { Label, MrPr, MrPrDetail, MrPrDiffFile, MrPrState, TaskId } from "../types";
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
  addMrPrLabels as apiAddLabels,
  removeMrPrLabels as apiRemoveLabels,
  addMrPrReviewers as apiAddReviewers,
  removeMrPrReviewers as apiRemoveReviewers,
  markMrPrReady as apiMarkReady,
  markMrPrDraft as apiMarkDraft,
  reopenMrPr as apiReopen,
  resolveDiscussion as apiResolveDiscussion,
  unresolveDiscussion as apiUnresolveDiscussion,
  listLabels as apiListLabels,
  checkoutMrPrLocally as apiCheckoutLocally,
} from "../api/tauri";
import { runMutation } from "../api/runMutation";
import { fetchIntoStore } from "../utils/store-helpers";
import { withTimeout } from "../utils/withTimeout";
import { addToast } from "./toast";
import * as m from "$lib/paraglide/messages";

/**
 * Timeout for the detail+diff fetch. Protects against the
 * ~3.4k-file PR scenario documented at the top of this file where
 * `gh api --paginate` hangs and strands the UI in a loading state.
 */
const DETAIL_TIMEOUT_MS = 15_000;

/** Current filter tab: open, closed, merged, or all. */
export const mrPrFilter = writable<MrPrState | "all">("open");

/** List of MR/PRs matching the current filter. */
export const mrPrList = writable<MrPr[]>([]);

/** Whether the list is loading. */
export const mrPrListLoading = writable(false);

/**
 * Last error raised while fetching the MR/PR list. Null on success.
 *
 * The list fetch silently falls back to `[]` on IPC error so the UI
 * doesn't crash, but that hid real failures (stale auth, CLI not found,
 * 401 from the forge). MrPrList reads this store and renders the error
 * message inline so users know *why* the list is empty.
 */
export const mrPrListError = writable<string | null>(null);

/** Currently selected MR/PR number. */
export const selectedMrPrNumber = writable<number | null>(null);

/** Detail of the selected MR/PR. */
export const mrPrDetail = writable<MrPrDetail | null>(null);

/** Changed files for the selected MR/PR. */
export const mrPrDiffFiles = writable<MrPrDiffFile[]>([]);

/** Whether the detail is loading. */
export const mrPrDetailLoading = writable(false);

/**
 * Last error raised while loading the selected MR/PR detail. Null on
 * success or when no load has been attempted. `MrPrDetail.svelte` reads
 * this store via `ForgeDetailShell` to render an inline error banner
 * with a retry button so users aren't stuck staring at a blank pane.
 */
export const mrPrDetailError = writable<string | null>(null);

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
  const currentFilter = get(mrPrFilter);
  const filter = currentFilter !== "all" ? currentFilter : undefined;
  mrPrListLoading.set(true);
  try {
    const data = await apiList(filter, 50);
    mrPrList.set(data);
    mrPrListError.set(null);
  } catch (err) {
    mrPrList.set([]);
    mrPrListError.set(err instanceof Error ? err.message : String(err));
  } finally {
    mrPrListLoading.set(false);
  }
}

/**
 * Load detail + diff for a specific MR/PR.
 *
 * The combined fetch is raced against a {@link DETAIL_TIMEOUT_MS}
 * timer via `withTimeout` so a hung IPC call (e.g. a huge paginated
 * diff) can't leave the detail pane stuck on a spinner. On any
 * failure — network, provider error, or timeout — the error is
 * surfaced both to the `mrPrDetailError` store (for the inline
 * banner) and to a user-facing toast, then the loading flag is
 * cleared in `finally`.
 */
export async function loadMrPrDetail(number: number): Promise<void> {
  selectedMrPrNumber.set(number);
  mrPrDetailLoading.set(true);
  mrPrDetailError.set(null);
  try {
    const [detail, diff] = await withTimeout(
      Promise.all([apiDetail(number), apiDiff(number)]),
      DETAIL_TIMEOUT_MS,
    );
    mrPrDetail.set(detail);
    mrPrDiffFiles.set(diff);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    mrPrDetail.set(null);
    mrPrDiffFiles.set([]);
    mrPrDetailError.set(msg);
    addToast({
      message: m.mrpr_load_failed({ number: number.toString(), error: msg }),
      type: "error",
    });
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

/** Clear all MR/PR state (e.g., on project switch). */
export function clearMrPrState() {
  mrPrList.set([]);
  mrPrFilter.set("open");
  clearMrPrDetail();
}

// ---------------------------------------------------------------------------
// Write operations
// ---------------------------------------------------------------------------

/** Create a new MR/PR and refresh the list. */
export async function createMrPr(
  source: string, target: string, title: string, body: string,
  draft: boolean, labels: string[], reviewers: string[]
): Promise<MrPr> {
  const result = await runMutation({
    kind: "mr_pr_create",
    invoke: () =>
      apiCreate(source, target, title, body, draft, labels, reviewers),
    successToast: (r) => `Opened PR #${r.number}`,
    failureToastPrefix: "PR create failed",
  });
  await refreshMrPrList();
  return result;
}

/** Edit a MR/PR and refresh the detail. */
export async function editMrPr(number: number, title?: string, body?: string): Promise<void> {
  await runMutation({
    kind: "mr_pr_edit",
    invoke: () => apiEdit(number, title, body),
    successToast: () => `Updated PR #${number}`,
    failureToastPrefix: "PR edit failed",
  });
  await loadMrPrDetail(number);
}

/** Merge a MR/PR and refresh the list. */
export async function mergeMrPr(number: number, strategy: string): Promise<void> {
  await runMutation({
    kind: "mr_pr_merge",
    invoke: () => apiMerge(number, strategy),
    successToast: () => `Merged PR #${number}`,
    failureToastPrefix: "PR merge failed",
  });
  clearMrPrDetail();
  await refreshMrPrList();
}

/** Close a MR/PR and refresh the list. */
export async function closeMrPr(number: number): Promise<void> {
  await runMutation({
    kind: "mr_pr_close",
    invoke: () => apiClose(number),
    successToast: () => `Closed PR #${number}`,
    failureToastPrefix: "PR close failed",
  });
  clearMrPrDetail();
  await refreshMrPrList();
}

// ---------------------------------------------------------------------------
// Review operations
// ---------------------------------------------------------------------------

/** Approve a MR/PR and refresh the detail. */
export async function approveMrPr(number: number): Promise<void> {
  await runMutation({
    kind: "mr_pr_approve",
    invoke: () => apiApprove(number),
    successToast: () => `Approved PR #${number}`,
    failureToastPrefix: "Approve failed",
  });
  await loadMrPrDetail(number);
}

/** Request changes on a MR/PR and refresh the detail. */
export async function requestChangesMrPr(number: number, body: string): Promise<void> {
  await runMutation({
    kind: "mr_pr_request_changes",
    invoke: () => apiRequestChanges(number, body),
    successToast: () => `Requested changes on PR #${number}`,
    failureToastPrefix: "Request changes failed",
  });
  await loadMrPrDetail(number);
}

/** Add a general comment to a MR/PR and refresh the detail. */
export async function addMrPrComment(number: number, body: string): Promise<void> {
  await runMutation({
    kind: "mr_pr_comment",
    invoke: () => apiAddComment(number, body),
    successToast: () => `Commented on PR #${number}`,
    failureToastPrefix: "Comment failed",
  });
  await loadMrPrDetail(number);
}

// ---------------------------------------------------------------------------
// Phase 8.2 — Labels, reviewers, draft lifecycle, reopen, resolve, checkout
// ---------------------------------------------------------------------------

/** Cache of repository labels, populated on demand by the label picker. */
export const repoLabels = writable<Label[]>([]);
/** Whether the repository label cache is currently loading. */
export const repoLabelsLoading = writable(false);

/** Fetch repository labels into the cache (no-op on error — list stays empty). */
export async function loadRepoLabels(): Promise<void> {
  repoLabelsLoading.set(true);
  try {
    const labels = await apiListLabels();
    repoLabels.set(labels);
  } catch {
    repoLabels.set([]);
  } finally {
    repoLabelsLoading.set(false);
  }
}

/** Add labels to a MR/PR and refresh the detail. */
export async function addMrPrLabels(number: number, labels: string[]): Promise<void> {
  await runMutation({
    kind: "mr_pr_labels_add",
    invoke: () => apiAddLabels(number, labels),
    successToast: () => `Added ${labels.length} label${labels.length === 1 ? "" : "s"}`,
    failureToastPrefix: "Add labels failed",
  });
  await loadMrPrDetail(number);
}

/** Remove labels from a MR/PR and refresh the detail. */
export async function removeMrPrLabels(number: number, labels: string[]): Promise<void> {
  await runMutation({
    kind: "mr_pr_labels_remove",
    invoke: () => apiRemoveLabels(number, labels),
    successToast: () => `Removed ${labels.length} label${labels.length === 1 ? "" : "s"}`,
    failureToastPrefix: "Remove labels failed",
  });
  await loadMrPrDetail(number);
}

/** Add reviewers to a MR/PR and refresh the detail. */
export async function addMrPrReviewers(number: number, reviewers: string[]): Promise<void> {
  await runMutation({
    kind: "mr_pr_reviewers_add",
    invoke: () => apiAddReviewers(number, reviewers),
    successToast: () => `Added ${reviewers.length} reviewer${reviewers.length === 1 ? "" : "s"}`,
    failureToastPrefix: "Add reviewers failed",
  });
  await loadMrPrDetail(number);
}

/** Remove reviewers from a MR/PR and refresh the detail. */
export async function removeMrPrReviewers(number: number, reviewers: string[]): Promise<void> {
  await runMutation({
    kind: "mr_pr_reviewers_remove",
    invoke: () => apiRemoveReviewers(number, reviewers),
    successToast: () => `Removed ${reviewers.length} reviewer${reviewers.length === 1 ? "" : "s"}`,
    failureToastPrefix: "Remove reviewers failed",
  });
  await loadMrPrDetail(number);
}

/** Mark a draft MR/PR as ready for review and refresh the detail. */
export async function markMrPrReady(number: number): Promise<void> {
  await runMutation({
    kind: "mr_pr_mark_ready",
    invoke: () => apiMarkReady(number),
    successToast: () => `Marked PR #${number} as ready`,
    failureToastPrefix: "Mark ready failed",
  });
  await loadMrPrDetail(number);
}

/** Convert a ready MR/PR back to draft and refresh the detail. */
export async function markMrPrDraft(number: number): Promise<void> {
  await runMutation({
    kind: "mr_pr_mark_draft",
    invoke: () => apiMarkDraft(number),
    successToast: () => `Marked PR #${number} as draft`,
    failureToastPrefix: "Mark draft failed",
  });
  await loadMrPrDetail(number);
}

/** Reopen a closed MR/PR, refresh the detail, and refresh the list. */
export async function reopenMrPr(number: number): Promise<void> {
  await runMutation({
    kind: "mr_pr_reopen",
    invoke: () => apiReopen(number),
    successToast: () => `Reopened PR #${number}`,
    failureToastPrefix: "Reopen failed",
  });
  await loadMrPrDetail(number);
  await refreshMrPrList();
}

/** Mark a GitLab discussion as resolved and refresh the detail. */
export async function resolveDiscussion(number: number, discussionId: string): Promise<void> {
  await runMutation({
    kind: "mr_pr_discussion_resolve",
    invoke: () => apiResolveDiscussion(number, discussionId),
    successToast: () => "Discussion resolved",
    failureToastPrefix: "Resolve failed",
  });
  await loadMrPrDetail(number);
}

/** Mark a GitLab discussion as unresolved and refresh the detail. */
export async function unresolveDiscussion(number: number, discussionId: string): Promise<void> {
  await runMutation({
    kind: "mr_pr_discussion_unresolve",
    invoke: () => apiUnresolveDiscussion(number, discussionId),
    successToast: () => "Discussion reopened",
    failureToastPrefix: "Reopen discussion failed",
  });
  await loadMrPrDetail(number);
}

/**
 * Kick off a MR/PR local checkout.
 *
 * Returns the task ID immediately — progress streams to the task popover
 * and the final `CheckoutResult` arrives via a `mr-pr-checked-out` event.
 */
export async function checkoutMrPrLocally(number: number): Promise<TaskId> {
  return apiCheckoutLocally(number);
}
