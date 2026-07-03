/**
 * MR/PR store — manages merge request / pull request state.
 *
 * Handles list fetching with filter tabs, detail loading, polling
 * for updates on open MR/PRs, and a derived store mapping branches
 * to open MR/PRs for graph badges.
 *
 * Migrated to the RepoState container (spec 08): the view state below is a thin
 * facade over the active repo's `MrPrSlice` (see `repo-state/MrPrSlice.ts`),
 * which also holds the former `ensuredShas` per-tab cache.
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

import { derived, get } from "svelte/store";
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
  replyToReviewComment as apiReplyToReviewComment,
  listLabels as apiListLabels,
  checkoutMrPrLocally as apiCheckoutLocally,
  addMrPrInlineComment as apiAddInlineComment,
} from "../api/tauri";
import { runMutation } from "../api/runMutation";
import { withTimeout } from "../utils/withTimeout";
import { addToast } from "./toast";
import { activeField, getActiveRepoState, type MrPrSlice } from "./repo-state";
import type { PrRawDiffContent } from "./repo-state/MrPrSlice";
import * as m from "$lib/paraglide/messages";

/**
 * Timeout for the detail+diff fetch. Protects against the
 * ~3.4k-file PR scenario documented at the top of this file where
 * `gh api --paginate` hangs and strands the UI in a loading state.
 */
const DETAIL_TIMEOUT_MS = 15_000;

// The MR/PR view state below is per-repo: it lives in the active repo's
// `MrPrSlice` so switching project tabs shows that repo's list, selection,
// and open diff (and never acts against the wrong repo's forge). These
// facades proxy the active slice so existing component imports keep working.

/** Current filter tab: open, closed, merged, or all. */
export const mrPrFilter = activeField<MrPrState | "all">((rs) => rs.mrPr.filter);

/** List of MR/PRs matching the current filter. */
export const mrPrList = activeField<MrPr[]>((rs) => rs.mrPr.list);

/** Whether the list is loading. */
export const mrPrListLoading = activeField<boolean>((rs) => rs.mrPr.listLoading);

/**
 * Last error raised while fetching the MR/PR list. Null on success.
 *
 * The list fetch silently falls back to `[]` on IPC error so the UI
 * doesn't crash, but that hid real failures (stale auth, CLI not found,
 * 401 from the forge). MrPrList reads this store and renders the error
 * message inline so users know *why* the list is empty.
 */
export const mrPrListError = activeField<string | null>((rs) => rs.mrPr.listError);

/** Currently selected MR/PR number. */
export const selectedMrPrNumber = activeField<number | null>((rs) => rs.mrPr.selectedNumber);

/** Detail of the selected MR/PR. */
export const mrPrDetail = activeField<MrPrDetail | null>((rs) => rs.mrPr.detail);

/** Changed files for the selected MR/PR. */
export const mrPrDiffFiles = activeField<MrPrDiffFile[]>((rs) => rs.mrPr.diffFiles);

/** Whether the detail (summary + body + comments) is loading. */
export const mrPrDetailLoading = activeField<boolean>((rs) => rs.mrPr.detailLoading);

/**
 * Last error raised while loading the selected MR/PR detail. Null on
 * success or when no load has been attempted. `MrPrDetail.svelte` reads
 * this store via `ForgeDetailShell` to render an inline error banner
 * with a retry button so users aren't stuck staring at a blank pane.
 */
export const mrPrDetailError = activeField<string | null>((rs) => rs.mrPr.detailError);

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
export const mrPrDiffLoading = activeField<boolean>((rs) => rs.mrPr.diffLoading);

/** Last error raised while loading the selected MR/PR's diff files. */
export const mrPrDiffError = activeField<string | null>((rs) => rs.mrPr.diffError);

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
  // Capture the target slice up front so a late response lands in the repo it
  // belongs to, even if the user switches tabs mid-flight (RepoState renders
  // only the active slice, so writing back into an inactive repo's slice is
  // correct and invisible).
  const slice = getActiveRepoState().mrPr;
  const currentFilter = get(slice.filter);
  const filter = currentFilter !== "all" ? currentFilter : undefined;
  slice.listLoading.set(true);
  try {
    const data = await apiList(filter, 50);
    slice.list.set(data);
    slice.listError.set(null);
  } catch (err) {
    slice.list.set([]);
    slice.listError.set(err instanceof Error ? err.message : String(err));
  } finally {
    slice.listLoading.set(false);
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
  // Capture the target slice so both the meta and diff branches (and a late
  // response after a tab switch) land in the repo this load belongs to.
  const slice = getActiveRepoState().mrPr;
  slice.selectedNumber.set(number);
  const metaP = loadMrPrDetailMeta(slice, number);
  const diffP = loadMrPrDetailDiff(slice, number);
  // `allSettled` so one branch failing doesn't abort the other —
  // each branch already reports its own toast / store error.
  await Promise.allSettled([metaP, diffP]);
}

async function loadMrPrDetailMeta(slice: MrPrSlice, number: number): Promise<void> {
  slice.detailLoading.set(true);
  slice.detailError.set(null);
  try {
    const detail = await withTimeout(apiDetail(number), DETAIL_TIMEOUT_MS);
    slice.detail.set(detail);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    slice.detail.set(null);
    slice.detailError.set(msg);
    addToast({
      message: m.mrpr_load_failed({ number: number.toString(), error: msg }),
      type: "error",
    });
  } finally {
    slice.detailLoading.set(false);
  }
}

async function loadMrPrDetailDiff(slice: MrPrSlice, number: number): Promise<void> {
  slice.diffLoading.set(true);
  slice.diffError.set(null);
  try {
    const diff = await withTimeout(apiDiff(number), DETAIL_TIMEOUT_MS);
    slice.diffFiles.set(diff);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    slice.diffFiles.set([]);
    slice.diffError.set(msg);
  } finally {
    slice.diffLoading.set(false);
  }
}

/** Clear detail state (e.g., when navigating away). */
export function clearMrPrDetail() {
  getActiveRepoState().mrPr.clearDetail();
}

/** Clear all MR/PR state (e.g., on project switch). */
export function clearMrPrState() {
  getActiveRepoState().mrPr.clear();
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

/**
 * Post an inline review comment on a file + line, then refresh the
 * detail so the new comment appears in both the bottom comments section
 * and the inline gutter layer. `number` is taken from the caller's scope
 * so the function stays usable from outside the store (e.g. the
 * +page.svelte commentsLayerFor factory).
 */
export async function postReviewComment(
  number: number,
  path: string,
  line: number,
  body: string,
): Promise<void> {
  const detail = get(mrPrDetail);
  if (!detail) throw new Error("no PR detail loaded");
  const { base_sha, head_sha } = detail.summary;
  await runMutation({
    kind: "pr_comment_post",
    invoke: () => apiAddInlineComment(number, path, line, body, base_sha, head_sha),
    successToast: () => "Comment posted",
    failureToastPrefix: "Post failed",
  });
  await loadMrPrDetail(number);
}

// ---------------------------------------------------------------------------
// Phase 8.2 — Labels, reviewers, draft lifecycle, reopen, resolve, checkout
// ---------------------------------------------------------------------------

/** Cache of repository labels, populated on demand by the label picker. */
export const repoLabels = activeField<Label[]>((rs) => rs.mrPr.repoLabels);
/** Whether the repository label cache is currently loading. */
export const repoLabelsLoading = activeField<boolean>((rs) => rs.mrPr.repoLabelsLoading);

/** Fetch repository labels into the cache (no-op on error — list stays empty). */
export async function loadRepoLabels(): Promise<void> {
  const slice = getActiveRepoState().mrPr;
  slice.repoLabelsLoading.set(true);
  try {
    const labels = await apiListLabels();
    slice.repoLabels.set(labels);
  } catch {
    slice.repoLabels.set([]);
  } finally {
    slice.repoLabelsLoading.set(false);
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
 * Reply to an existing review-comment thread on a MR/PR.
 *
 * `threadId` is what the parser stored on the inline comment's
 * `discussion_id` field — opaque to the frontend.
 */
export async function replyToReviewComment(
  number: number,
  threadId: string,
  body: string,
): Promise<void> {
  await runMutation({
    kind: "pr_comment_reply",
    invoke: () => apiReplyToReviewComment(number, threadId, body),
    successToast: () => "Reply posted",
    failureToastPrefix: "Reply failed",
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

export type { PrRawDiffContent } from "./repo-state/MrPrSlice";

/** Diff content for the currently-selected PR file, or `null` if none. */
export const prFileDiff = activeField<PrRawDiffContent | null>((rs) => rs.mrPr.prFileDiff);
/** True while `loadPrFileDiff` is in flight. */
export const loadingPrFileDiff = activeField<boolean>((rs) => rs.mrPr.loadingPrFileDiff);
/** Last error raised during `loadPrFileDiff`, or `null`. */
export const prFileDiffError = activeField<string | null>((rs) => rs.mrPr.prFileDiffError);

/**
 * Currently selected file path in the PR file list. Drives the
 * `selected` row highlight + prev/next navigation cursor.
 */
export const selectedPrFilePath = activeField<string | null>((rs) => rs.mrPr.selectedPrFilePath);

/**
 * Ensure `sha` exists in `slice`'s repo's local object database, deduped per
 * sha via the slice's `ensuredShas` cache. Without it, every file click in a
 * PR re-ran the `ensure_commit_local` preflight — and when the commit
 * couldn't be fetched, every click (and every `[` / `]` file-nav keystroke)
 * spawned a fresh failing `git fetch` task. Fork-head clone URLs can be
 * unfetchable (e.g. anonymous https against a private fork), while the base
 * repo advertises PR head objects too — so a failed fetch from `remoteUrl`
 * falls back to `origin` before surfacing the error. Failed ensures are
 * evicted so an explicit retry can attempt one new fetch, but nothing retries
 * automatically.
 */
function ensureShaLocal(slice: MrPrSlice, sha: string, remoteUrl: string | null): Promise<void> {
  const inFlight = slice.ensuredShas.get(sha);
  if (inFlight) return inFlight;
  const attempt = (async () => {
    const { ensureCommitLocal } = await import("../api/tauri");
    try {
      await ensureCommitLocal(sha, remoteUrl);
    } catch (err) {
      if (remoteUrl === null) throw err;
      await ensureCommitLocal(sha, null);
    }
  })().catch((err: unknown) => {
    slice.ensuredShas.delete(sha);
    throw err;
  });
  slice.ensuredShas.set(sha, attempt);
  return attempt;
}

/**
 * Loads the diff for `path` in the PR summarised by `detail`. Ensures
 * BOTH the base and head commits are local first — `baseRefOid` is the
 * remote base-branch tip and is routinely absent from the local odb, in
 * which case the old `getFileAtCommit(base_sha)` failure was silently
 * swallowed and the whole file rendered as added. With presence
 * guaranteed up front, a per-file read error only means "path absent at
 * that commit" (added/deleted files), which legitimately maps to an
 * empty side. Swaps to a binary placeholder if either blob is flagged
 * binary. Sets `prFileDiffError` on failure.
 */
export async function loadPrFileDiff(detail: MrPrDetail, path: string): Promise<void> {
  // Capture the target slice so a late diff response (and the per-repo
  // `ensuredShas` cache) lands in the repo this load belongs to, even if the
  // user switches tabs mid-flight. The per-slice `prFileDiffRequestId` still
  // cancels an older diff superseded by a newer one in the SAME repo.
  const slice = getActiveRepoState().mrPr;
  const { base_sha, head_sha, head_repo_url } = detail.summary;
  const requestId = ++slice.prFileDiffRequestId;
  slice.prFileDiff.set(null);
  slice.prFileDiffError.set(null);
  slice.loadingPrFileDiff.set(true);
  slice.selectedPrFilePath.set(path);
  try {
    if (!base_sha || !head_sha) {
      throw new Error(m.pr_diff_missing_shas());
    }
    const { getFileAtCommit } = await import("../api/tauri");
    // Sequential on purpose: concurrent `git fetch` children race on
    // FETCH_HEAD. Both calls are cheap no-ops once the sha is cached.
    await ensureShaLocal(slice, head_sha, head_repo_url);
    await ensureShaLocal(slice, base_sha, null);
    const [oldR, newR] = await Promise.all([
      getFileAtCommit(base_sha, path).catch(() => ({ kind: "text" as const, data: "" })),
      getFileAtCommit(head_sha, path).catch(() => ({ kind: "text" as const, data: "" })),
    ]);
    if (requestId !== slice.prFileDiffRequestId) return;
    const binary = oldR.kind === "binary" || newR.kind === "binary";
    slice.prFileDiff.set({
      oldContent: oldR.kind === "text" ? oldR.data : "",
      newContent: newR.kind === "text" ? newR.data : "",
      filename: path,
      binary,
    });
  } catch (e) {
    if (requestId !== slice.prFileDiffRequestId) return;
    slice.prFileDiffError.set(e instanceof Error ? e.message : String(e));
  } finally {
    if (requestId === slice.prFileDiffRequestId) slice.loadingPrFileDiff.set(false);
  }
}

/** Close the PR diff panel (back-to-list affordance). */
export function closePrFileDiff(): void {
  const slice = getActiveRepoState().mrPr;
  slice.prFileDiff.set(null);
  slice.prFileDiffError.set(null);
  slice.selectedPrFilePath.set(null);
}

// ---------------------------------------------------------------------------
// PR diff keyboard shortcuts
// ---------------------------------------------------------------------------

import { registerShortcuts, unregisterShortcuts } from "./shortcuts";

/**
 * Handlers supplied by `+page.svelte` so the store doesn't depend on
 * route-local scope. `onPrev` / `onNext` cycle the PR file selection.
 */
export interface PrDiffShortcutHandlers {
  onPrev: () => void;
  onNext: () => void;
}

/**
 * Register bracket-key file navigation bindings. `[` for prev, `]` for
 * next. Registration is idempotent — duplicate calls replace the prior
 * handlers.
 */
export function registerPrDiffShortcuts(h: PrDiffShortcutHandlers): void {
  registerShortcuts([
    {
      id: "prDiff.prev",
      keys: { key: "[" },
      label: "Previous file in PR",
      category: "PR",
      action: h.onPrev,
    },
    {
      id: "prDiff.next",
      keys: { key: "]" },
      label: "Next file in PR",
      category: "PR",
      action: h.onNext,
    },
  ]);
}

/** Remove the PR file-nav shortcuts from the global registry. */
export function unregisterPrDiffShortcuts(): void {
  unregisterShortcuts(["prDiff.prev", "prDiff.next"]);
}
