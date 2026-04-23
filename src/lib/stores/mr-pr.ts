/**
 * MR/PR store — manages merge request / pull request state.
 *
 * Handles list fetching with filter tabs, detail loading, polling
 * for updates on open MR/PRs, and a derived store mapping branches
 * to open MR/PRs for graph badges.
 *
 * PR hang mitigation
 * ------------------
 * Three layers guard against a hung detail or diff fetch:
 *   1. TS side — each fetch is raced against {@link DETAIL_TIMEOUT_MS}
 *      (15 s) via `withTimeout` so neither load can strand the UI on
 *      a spinner.
 *   2. Rust side — `get_mr_pr_diff_impl` caps the `gh api …
 *      /pulls/{n}/files --paginate` child at 20 s and the parsed
 *      payload at 50 MB (see `crates/cli-provider/src/github/mr_pr.rs`).
 *   3. Store-level decoupling — meta (summary/body/comments) and
 *      diff (changed files) each have their own loading / error
 *      state so a slow diff fetch can't gate the metadata render.
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

/** Whether the detail (summary + body + comments) is loading. */
export const mrPrDetailLoading = writable(false);

/**
 * Last error raised while loading the selected MR/PR detail. Null on
 * success or when no load has been attempted. `MrPrDetail.svelte` reads
 * this store via `ForgeDetailShell` to render an inline error banner
 * with a retry button so users aren't stuck staring at a blank pane.
 */
export const mrPrDetailError = writable<string | null>(null);

/**
 * Whether the diff-files fetch is in flight for the selected MR/PR.
 *
 * Tracked independently from {@link mrPrDetailLoading} so the summary
 * / body / comments can paint as soon as `get_mr_pr_detail` lands —
 * without waiting on the often-slower `gh api …/pulls/{n}/files
 * --paginate` call. The "changed files" section renders its own
 * inline spinner / error banner driven by this store + {@link
 * mrPrDiffError}.
 */
export const mrPrDiffLoading = writable(false);

/** Last error raised while loading the selected MR/PR's diff files. */
export const mrPrDiffError = writable<string | null>(null);

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
 * Meta (`apiDetail`) and diff (`apiDiff`) are fetched concurrently
 * but track their own loading + error state. This way a slow diff
 * fetch (e.g. the 3.4k-file PR that inspired the timeout machinery)
 * doesn't gate the summary / body / comments render — the user
 * sees the PR metadata as soon as `gh pr view` lands, and the
 * "changed files" section reports its own spinner / error inline.
 *
 * Both fetches are individually capped by {@link DETAIL_TIMEOUT_MS}
 * via `withTimeout` so a hung IPC call can't strand the UI on a
 * spinner.
 */
export async function loadMrPrDetail(number: number): Promise<void> {
  selectedMrPrNumber.set(number);
  const metaP = loadMrPrDetailMeta(number);
  const diffP = loadMrPrDetailDiff(number);
  // `allSettled` so one branch failing doesn't abort the other —
  // each branch already reports its own toast / store error.
  await Promise.allSettled([metaP, diffP]);
}

async function loadMrPrDetailMeta(number: number): Promise<void> {
  mrPrDetailLoading.set(true);
  mrPrDetailError.set(null);
  try {
    const detail = await withTimeout(apiDetail(number), DETAIL_TIMEOUT_MS);
    mrPrDetail.set(detail);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    mrPrDetail.set(null);
    mrPrDetailError.set(msg);
    addToast({
      message: m.mrpr_load_failed({ number: number.toString(), error: msg }),
      type: "error",
    });
  } finally {
    mrPrDetailLoading.set(false);
  }
}

async function loadMrPrDetailDiff(number: number): Promise<void> {
  mrPrDiffLoading.set(true);
  mrPrDiffError.set(null);
  try {
    const diff = await withTimeout(apiDiff(number), DETAIL_TIMEOUT_MS);
    mrPrDiffFiles.set(diff);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    mrPrDiffFiles.set([]);
    mrPrDiffError.set(msg);
  } finally {
    mrPrDiffLoading.set(false);
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

// ─── PR per-file diff panel ──────────────────────────────────────────────────

import type { RawDiffContent } from "./graph";

/**
 * Currently-viewed PR per-file diff payload. Null when no file is selected.
 * Shares the `RawDiffContent` shape with the graph/branch/reflog diff panels
 * so `DiffEditor.svelte` can render it unchanged, plus an optional
 * `binary: boolean` flag for the placeholder branch.
 */
export interface PrRawDiffContent extends RawDiffContent {
  /** True when either side's blob was flagged binary. */
  binary: boolean;
}

/** Diff content for the currently-selected PR file, or `null` if none. */
export const prFileDiff = writable<PrRawDiffContent | null>(null);
/** True while `loadPrFileDiff` is in flight. */
export const loadingPrFileDiff = writable(false);
/** Last error raised during `loadPrFileDiff`, or `null`. */
export const prFileDiffError = writable<string | null>(null);

/**
 * Currently selected file path in the PR file list. Drives the
 * `selected` row highlight + prev/next navigation cursor.
 */
export const selectedPrFilePath = writable<string | null>(null);

/**
 * Loads the diff for `path` in the PR summarised by `detail`. Ensures
 * the head commit is local (fork PRs), then reads both base and head
 * blobs in parallel. Swaps to a binary placeholder if either blob is
 * flagged binary. Sets `prFileDiffError` on failure.
 */
export async function loadPrFileDiff(detail: MrPrDetail, path: string): Promise<void> {
  const { base_sha, head_sha, head_repo_url } = detail.summary;
  prFileDiff.set(null);
  prFileDiffError.set(null);
  loadingPrFileDiff.set(true);
  selectedPrFilePath.set(path);
  try {
    const { ensureCommitLocal, getFileAtCommit } = await import("../api/tauri");
    await ensureCommitLocal(head_sha, head_repo_url);
    const [oldR, newR] = await Promise.all([
      getFileAtCommit(base_sha, path).catch(() => ({ kind: "text" as const, data: "" })),
      getFileAtCommit(head_sha, path).catch(() => ({ kind: "text" as const, data: "" })),
    ]);
    const binary = oldR.kind === "binary" || newR.kind === "binary";
    prFileDiff.set({
      oldContent: oldR.kind === "text" ? oldR.data : "",
      newContent: newR.kind === "text" ? newR.data : "",
      filename: path,
      binary,
    });
  } catch (e) {
    prFileDiffError.set(e instanceof Error ? e.message : String(e));
  } finally {
    loadingPrFileDiff.set(false);
  }
}

/** Close the PR diff panel (back-to-list affordance). */
export function closePrFileDiff(): void {
  prFileDiff.set(null);
  prFileDiffError.set(null);
  selectedPrFilePath.set(null);
}
