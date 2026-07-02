/**
 * Issues store — manages issue list, detail, filters, labels & milestones cache.
 *
 * Mirrors the MR/PR store pattern. The labels and milestones caches are
 * lazily populated via `refreshLabelsCache()` / `refreshMilestonesCache()`
 * the first time a picker mounts.
 *
 * TODO(spec 08): migrate into the RepoState container
 * (`stores/repo-state/`) as an `IssuesSlice`. See `stores/branches.ts` for
 * the migrated facade pattern.
 */

import { writable, derived, get } from "svelte/store";
import type { Issue, IssueDetail, IssueState, Label, Milestone } from "../types";
import * as api from "../api/tauri";
import { fetchIntoStore } from "../utils/store-helpers";

/** Current state filter tab: open, closed, or all. */
export const issueStateFilter = writable<IssueState | "all">("open");

/** Current list of issues matching the filter. */
export const issueList = writable<Issue[]>([]);

/** Whether the list is currently loading. */
export const issueListLoading = writable(false);

/** Currently selected issue number. */
export const selectedIssueNumber = writable<number | null>(null);

/** Full detail of the selected issue. */
export const issueDetail = writable<IssueDetail | null>(null);

/** Whether the detail view is loading. */
export const issueDetailLoading = writable(false);

/** Cache of repository labels (for pickers). Lazily loaded. */
export const labelsCache = writable<Label[]>([]);
/** Whether the labels cache is currently loading. */
export const labelsCacheLoading = writable(false);

/** Cache of repository milestones (for pickers). Lazily loaded. */
export const milestonesCache = writable<Milestone[]>([]);
/** Whether the milestones cache is currently loading. */
export const milestonesCacheLoading = writable(false);

/** Derived: `Map<number, Issue>` for cross-ref resolution by number. */
export const issueByNumber = derived(issueList, ($list) => {
  const map = new Map<number, Issue>();
  for (const i of $list) map.set(i.number, i);
  return map;
});

/** Fetch the issue list with the current state filter. */
export async function refreshIssueList(): Promise<void> {
  const filter = get(issueStateFilter);
  const state = filter === "all" ? undefined : (filter as IssueState);
  issueListLoading.set(true);
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
    issueList.set(items);
    // Invalidate selection if the currently-selected issue vanished.
    const selected = get(selectedIssueNumber);
    if (
      selected !== null &&
      !items.some((item) => item.number === selected)
    ) {
      selectedIssueNumber.set(null);
    }
  } catch {
    issueList.set([]);
    selectedIssueNumber.set(null);
  } finally {
    issueListLoading.set(false);
  }
}

/** Load detail for a specific issue and mark it selected. */
export async function loadIssueDetail(number: number): Promise<void> {
  selectedIssueNumber.set(number);
  issueDetailLoading.set(true);
  try {
    const d = await api.getIssue(number);
    issueDetail.set(d);
  } catch {
    issueDetail.set(null);
  } finally {
    issueDetailLoading.set(false);
  }
}

/** Clear detail-only state without touching the list. */
export function clearIssueDetail(): void {
  selectedIssueNumber.set(null);
  issueDetail.set(null);
}

/** Reset all issue state (on project switch). */
export function clearIssueState(): void {
  issueList.set([]);
  issueStateFilter.set("open");
  clearIssueDetail();
}

/** Populate the labels cache (no-op on error — cache stays empty). */
export async function refreshLabelsCache(): Promise<void> {
  await fetchIntoStore(labelsCache, labelsCacheLoading, api.listLabels, []);
}

/** Populate the milestones cache (no-op on error — cache stays empty). */
export async function refreshMilestonesCache(): Promise<void> {
  await fetchIntoStore(
    milestonesCache,
    milestonesCacheLoading,
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
