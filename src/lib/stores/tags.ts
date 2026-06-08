/**
 * Tags store — paginated tag listing, search, selection with detail loading,
 * and CRUD mutations (create/delete/push).
 *
 * Tags are loaded in pages of 30. Client-side filtering is attempted first;
 * if no results match, a backend search via `git tag -l` is used as fallback.
 * Selection fires 3 parallel fetches (commit detail, stats, files) with a
 * last-wins async guard.
 */

import { writable, derived, get } from "svelte/store";
import type { TagInfo, CommitInfo, CommitStats, CommitFileChange } from "../types";
import {
  listTagsPaginated,
  searchTags as apiSearchTags,
  getCommitDetail,
  getCommitStats,
  getCommitFiles,
  createTag as apiCreateTag,
  deleteTag as apiDeleteTag,
  pushTag as apiPushTag,
} from "../api/tauri";
import { runMutation } from "../api/runMutation";
import { fetchPageIntoStore } from "../utils/store-helpers";

// ---------------------------------------------------------------------------
// List state
// ---------------------------------------------------------------------------

export const tags = writable<TagInfo[]>([]);
export const tagsLoading = writable(false);
export const hasMoreTags = writable(false);
export const tagFilter = writable("");

const PAGE_SIZE = 30;
let currentPage = 1;

/** Snapshot of tags before filter was applied, so we can restore without re-fetching. */
let preFilterSnapshot: TagInfo[] = [];

// ---------------------------------------------------------------------------
// Detail state
// ---------------------------------------------------------------------------

export const selectedTagName = writable<string | null>(null);
export const loadingDetail = writable(false);
export const selectedCommitInfo = writable<CommitInfo | null>(null);
export const selectedCommitStats = writable<CommitStats | null>(null);
export const selectedCommitFiles = writable<CommitFileChange[] | null>(null);

// ---------------------------------------------------------------------------
// Derived
// ---------------------------------------------------------------------------

/** The TagInfo for the currently selected tag (from the loaded list). */
export const selectedTagInfo = derived(
  [tags, selectedTagName],
  ([$tags, $name]) => ($name ? $tags.find((t) => t.name === $name) ?? null : null),
);

/** Client-side filtered tags. */
export const filteredTags = derived(
  [tags, tagFilter],
  ([$tags, $filter]) => {
    if (!$filter) return $tags;
    const q = $filter.toLowerCase();
    return $tags.filter((t) => t.name.toLowerCase().includes(q));
  },
);

// ---------------------------------------------------------------------------
// List functions
// ---------------------------------------------------------------------------

export async function refreshTags() {
  currentPage = 1;
  await fetchPageIntoStore(
    tags,
    tagsLoading,
    hasMoreTags,
    0,
    () => listTagsPaginated(PAGE_SIZE, 1),
    PAGE_SIZE,
  );
  preFilterSnapshot = get(tags);

  // Clear selection if tag no longer exists
  const name = get(selectedTagName);
  if (name && !get(tags).some((t) => t.name === name)) {
    selectedTagName.set(null);
    selectedCommitInfo.set(null);
    selectedCommitStats.set(null);
    selectedCommitFiles.set(null);
    loadingDetail.set(false);
  }
}

export async function loadMoreTags() {
  currentPage++;
  await fetchPageIntoStore(
    tags,
    tagsLoading,
    hasMoreTags,
    currentPage - 1, // > 0 triggers append
    () => listTagsPaginated(PAGE_SIZE, currentPage),
    PAGE_SIZE,
  );
  preFilterSnapshot = get(tags);
}

/** Backend fallback when client-side filter yields no results. */
export async function searchTagsBackend(query: string) {
  try {
    const results = await apiSearchTags(query);
    tags.set(results);
    hasMoreTags.set(false); // Search returns all matches
  } catch {
    tags.set([]);
  }
}

/** Restore the pre-filter tag list (called when filter is cleared). */
export function restorePreFilterTags() {
  tags.set(preFilterSnapshot);
  hasMoreTags.set(preFilterSnapshot.length >= PAGE_SIZE);
}

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

export function selectTag(name: string) {
  // 1. Synchronous — instant highlight + spinner
  selectedTagName.set(name);
  loadingDetail.set(true);
  selectedCommitInfo.set(null);
  selectedCommitStats.set(null);
  selectedCommitFiles.set(null);

  // 2. Find the tag to get its commit_oid
  const tag = get(tags).find((t) => t.name === name);
  if (!tag) {
    loadingDetail.set(false);
    return;
  }

  // 3. Capture name for last-wins guard
  const expectedTag = name;

  // 4. Fire all calls in parallel — do not block UI
  Promise.all([
    getCommitDetail(tag.commit_oid),
    getCommitStats(tag.commit_oid),
    getCommitFiles(tag.commit_oid),
  ]).then(([commitInfo, commitStats, commitFiles]) => {
    // Last-wins guard: only apply if still selected
    if (get(selectedTagName) === expectedTag) {
      selectedCommitInfo.set(commitInfo);
      selectedCommitStats.set(commitStats);
      selectedCommitFiles.set(commitFiles);
      loadingDetail.set(false);
    }
  }).catch(() => {
    if (get(selectedTagName) === expectedTag) {
      loadingDetail.set(false);
    }
  });
}

// ---------------------------------------------------------------------------
// Mutations (create, delete, push)
// ---------------------------------------------------------------------------

export async function doCreateTag(name: string, target: string, message: string | null) {
  await runMutation({
    kind: "tag_create",
    invoke: () => apiCreateTag(name, target, message),
    successToast: () => `Tagged ${name}`,
    failureToastPrefix: "Tag create failed",
  });
  // Tag list refresh is now driven by the mutation dispatcher
  // (refs_changed → refreshTags in stores/mutations.ts), so no explicit
  // refresh here — that would double-fetch.
}

export async function doDeleteTag(name: string) {
  await runMutation({
    kind: "tag_delete",
    invoke: () => apiDeleteTag(name),
    successToast: () => `Deleted tag ${name}`,
    failureToastPrefix: "Tag delete failed",
  });
  // Clear selection if deleted tag was selected
  if (get(selectedTagName) === name) {
    selectedTagName.set(null);
    selectedCommitInfo.set(null);
    selectedCommitStats.set(null);
    selectedCommitFiles.set(null);
    loadingDetail.set(false);
  }
  // Refresh is driven by the mutation dispatcher (refs_changed → refreshTags).
}

export async function doPushTag(tagName: string | null, remote: string) {
  return apiPushTag(tagName, remote);
}

/** Reset all tag selection/detail state. Called on repo switch. */
export function clearTagState() {
  tags.set([]);
  selectedTagName.set(null);
  selectedCommitInfo.set(null);
  selectedCommitStats.set(null);
  selectedCommitFiles.set(null);
  loadingDetail.set(false);
  tagFilter.set("");
  hasMoreTags.set(false);
}
