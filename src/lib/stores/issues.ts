/**
 * Issues store — manages issue list, detail, filters, labels & milestones cache.
 *
 * Mirrors the MR/PR store pattern. The labels and milestones caches are
 * lazily populated via `refreshLabelsCache()` / `refreshMilestonesCache()`
 * the first time a picker mounts.
 *
 * Migrated to the RepoState container (spec 08): the view state below is a thin
 * facade over the active repo's `IssuesSlice` (see `repo-state/IssuesSlice.ts`).
 */

import { derived, get } from "svelte/store";
import type { Issue, IssueDetail, IssueState, Label, Milestone } from "../types";
import * as api from "../api/tauri";
import { fetchIntoStore } from "../utils/store-helpers";
import { activeField, getActiveRepoState } from "./repo-state";

// The issue view state below is per-repo: it lives in the active repo's
// `IssuesSlice` so switching project tabs shows that repo's list, selection,
// filter, and picker caches. These facades proxy the active slice so existing
// component imports keep working.

/** Current state filter tab: open, closed, or all. */
export const issueStateFilter = activeField<IssueState | "all">((rs) => rs.issues.stateFilter);

/** Current list of issues matching the filter. */
export const issueList = activeField<Issue[]>((rs) => rs.issues.list);

/** Whether the list is currently loading. */
export const issueListLoading = activeField<boolean>((rs) => rs.issues.listLoading);

/** Currently selected issue number. */
export const selectedIssueNumber = activeField<number | null>((rs) => rs.issues.selectedNumber);

/** Full detail of the selected issue. */
export const issueDetail = activeField<IssueDetail | null>((rs) => rs.issues.detail);

/** Whether the detail view is loading. */
export const issueDetailLoading = activeField<boolean>((rs) => rs.issues.detailLoading);

/** Cache of repository labels (for pickers). Lazily loaded. */
export const labelsCache = activeField<Label[]>((rs) => rs.issues.labelsCache);
/** Whether the labels cache is currently loading. */
export const labelsCacheLoading = activeField<boolean>((rs) => rs.issues.labelsCacheLoading);

/** Cache of repository milestones (for pickers). Lazily loaded. */
export const milestonesCache = activeField<Milestone[]>((rs) => rs.issues.milestonesCache);
/** Whether the milestones cache is currently loading. */
export const milestonesCacheLoading = activeField<boolean>((rs) => rs.issues.milestonesCacheLoading);

/** Derived: `Map<number, Issue>` for cross-ref resolution by number. */
export const issueByNumber = derived(issueList, ($list) => {
  const map = new Map<number, Issue>();
  for (const i of $list) map.set(i.number, i);
  return map;
});

/** Fetch the issue list with the current state filter. */
export async function refreshIssueList(): Promise<void> {
  // Capture the target slice up front so a late response lands in the repo it
  // belongs to, even if the user switches tabs mid-flight (RepoState renders
  // only the active slice, so writing back into an inactive repo's slice is
  // correct and invisible).
  const slice = getActiveRepoState().issues;
  const filter = get(slice.stateFilter);
  const state = filter === "all" ? undefined : (filter as IssueState);
  slice.listLoading.set(true);
  try {
    const items = await api.listIssues(
      state,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      50,
    );
    slice.list.set(items);
    // Invalidate selection if the currently-selected issue vanished.
    const selected = get(slice.selectedNumber);
    if (
      selected !== null &&
      !items.some((item) => item.number === selected)
    ) {
      slice.selectedNumber.set(null);
    }
  } catch {
    slice.list.set([]);
    slice.selectedNumber.set(null);
  } finally {
    slice.listLoading.set(false);
  }
}

/** Load detail for a specific issue and mark it selected. */
export async function loadIssueDetail(number: number): Promise<void> {
  const slice = getActiveRepoState().issues;
  slice.selectedNumber.set(number);
  slice.detailLoading.set(true);
  try {
    const d = await api.getIssue(number);
    slice.detail.set(d);
  } catch {
    slice.detail.set(null);
  } finally {
    slice.detailLoading.set(false);
  }
}

/** Clear detail-only state without touching the list. */
export function clearIssueDetail(): void {
  getActiveRepoState().issues.clearDetail();
}

/** Reset all issue state (on project switch). */
export function clearIssueState(): void {
  getActiveRepoState().issues.clear();
}

/** Populate the labels cache (no-op on error — cache stays empty). */
export async function refreshLabelsCache(): Promise<void> {
  const slice = getActiveRepoState().issues;
  await fetchIntoStore(slice.labelsCache, slice.labelsCacheLoading, api.listLabels, []);
}

/** Populate the milestones cache (no-op on error — cache stays empty). */
export async function refreshMilestonesCache(): Promise<void> {
  const slice = getActiveRepoState().issues;
  await fetchIntoStore(
    slice.milestonesCache,
    slice.milestonesCacheLoading,
    api.listMilestones,
    [],
  );
}

// ─── Write operations ────────────────────────────────────────────────

/** Create a new issue and refresh the list. */
export async function createIssue(
  title: string,
  body: string,
  labels: string[],
  assignees: string[],
  milestone: number | null,
): Promise<Issue> {
  const result = await api.createIssue(title, body, labels, assignees, milestone);
  await refreshIssueList();
  return result;
}

/** Edit an issue and reload detail. */
export async function editIssue(
  number: number,
  title?: string,
  body?: string,
): Promise<void> {
  await api.editIssue(number, title, body);
  await loadIssueDetail(number);
}

/** Close an issue and refresh list + detail. */
export async function closeIssue(number: number): Promise<void> {
  await api.closeIssue(number);
  await loadIssueDetail(number);
  await refreshIssueList();
}

/** Reopen an issue and refresh list + detail. */
export async function reopenIssue(number: number): Promise<void> {
  await api.reopenIssue(number);
  await loadIssueDetail(number);
  await refreshIssueList();
}

/** Post a comment on an issue and refresh detail. */
export async function addIssueComment(
  number: number,
  body: string,
): Promise<void> {
  await api.addIssueComment(number, body);
  await loadIssueDetail(number);
}

/** Add labels and reload detail. List row counts tolerate staleness until next refresh. */
export async function addIssueLabels(
  number: number,
  labels: string[],
): Promise<void> {
  await api.addIssueLabels(number, labels);
  await loadIssueDetail(number);
}

/** Remove labels and reload detail. List row counts tolerate staleness until next refresh. */
export async function removeIssueLabels(
  number: number,
  labels: string[],
): Promise<void> {
  await api.removeIssueLabels(number, labels);
  await loadIssueDetail(number);
}

/** Add assignees and reload detail. List row counts tolerate staleness until next refresh. */
export async function addIssueAssignees(
  number: number,
  assignees: string[],
): Promise<void> {
  await api.addIssueAssignees(number, assignees);
  await loadIssueDetail(number);
}

/** Remove assignees and reload detail. List row counts tolerate staleness until next refresh. */
export async function removeIssueAssignees(
  number: number,
  assignees: string[],
): Promise<void> {
  await api.removeIssueAssignees(number, assignees);
  await loadIssueDetail(number);
}

/** Set (or clear) the milestone on an issue and reload detail. List row counts tolerate staleness until next refresh. */
export async function setIssueMilestone(
  number: number,
  milestoneId: number | null,
): Promise<void> {
  await api.setIssueMilestone(number, milestoneId);
  await loadIssueDetail(number);
}
