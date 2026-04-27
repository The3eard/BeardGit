/**
 * E2E: Tag management workflow
 *
 * Tests refreshTags, selectTag, doCreateTag, doDeleteTag, and the filter
 * derived stores via the tags store with mocked IPC.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { mockInvokeResponse } from "../setup";
import type { TagInfo, CommitInfo, CommitStats, CommitFileChange } from "$lib/types";

import {
  tags,
  tagsLoading,
  hasMoreTags,
  tagFilter,
  selectedTagName,
  loadingDetail,
  selectedCommitInfo,
  selectedCommitStats,
  selectedCommitFiles,
  selectedTagInfo,
  filteredTags,
  refreshTags,
  doCreateTag,
  doDeleteTag,
  selectTag,
  clearTagState,
} from "$lib/stores/tags";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const MOCK_TAGS: TagInfo[] = [
  {
    name: "v1.0.0",
    object_oid: "tagobj1111",
    commit_oid: "aaaa1111",
    annotated: true,
    message: "First stable release",
    tagger_name: "Alice",
    tagger_email: "alice@example.com",
    date: "2024-01-01T00:00:00Z",
  },
  {
    name: "v1.1.0",
    object_oid: "tagobj2222",
    commit_oid: "bbbb2222",
    annotated: true,
    message: "Feature update",
    tagger_name: "Bob",
    tagger_email: "bob@example.com",
    date: "2024-02-01T00:00:00Z",
  },
  {
    name: "v2.0.0-rc1",
    object_oid: "tagobj3333",
    commit_oid: "cccc3333",
    annotated: false,
    message: "",
    tagger_name: "",
    tagger_email: "",
    date: "2024-03-01T00:00:00Z",
  },
];

const MOCK_COMMIT_INFO: CommitInfo = {
  oid: "aaaa1111",
  summary: "feat: first stable release",
  body: "",
  author: "Alice",
  email: "alice@example.com",
  timestamp: 1704067200,
  parents: ["prev0000"],
  refs: ["tag: v1.0.0"],
};

const MOCK_COMMIT_STATS: CommitStats = {
  files_changed: 5,
  insertions: 120,
  deletions: 30,
};

const MOCK_COMMIT_FILES: CommitFileChange[] = [
  { path: "src/main.ts", status: "modified" },
  { path: "CHANGELOG.md", status: "modified" },
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("tag operations workflow", () => {
  beforeEach(() => {
    clearTagState();
  });

  // ── refreshTags ──────────────────────────────────────────────────────

  it("refreshTags populates tags store", async () => {
    mockInvokeResponse("list_tags_paginated", MOCK_TAGS);

    await refreshTags();

    const list = get(tags);
    expect(list).toHaveLength(3);
    expect(list[0].name).toBe("v1.0.0");
    expect(list[1].name).toBe("v1.1.0");
  });

  it("refreshTags sets loading true then false", async () => {
    mockInvokeResponse("list_tags_paginated", MOCK_TAGS);

    const loadingHistory: boolean[] = [];
    const unsub = tagsLoading.subscribe((v) => loadingHistory.push(v));
    await refreshTags();
    unsub();

    expect(loadingHistory).toContain(true);
    expect(loadingHistory[loadingHistory.length - 1]).toBe(false);
  });

  it("refreshTags sets empty list on error", async () => {
    mockInvokeResponse("list_tags_paginated", () => { throw new Error("git error"); });

    await refreshTags();

    expect(get(tags)).toHaveLength(0);
    expect(get(tagsLoading)).toBe(false);
  });

  it("refreshTags sets hasMoreTags false when fewer than PAGE_SIZE returned", async () => {
    mockInvokeResponse("list_tags_paginated", MOCK_TAGS); // 3 < 30

    await refreshTags();

    expect(get(hasMoreTags)).toBe(false);
  });

  it("refreshTags clears selection when selected tag no longer exists", async () => {
    tags.set(MOCK_TAGS);
    selectedTagName.set("v1.1.0");

    // Return only v1.0.0
    mockInvokeResponse("list_tags_paginated", [MOCK_TAGS[0]]);

    await refreshTags();

    expect(get(selectedTagName)).toBeNull();
    expect(get(selectedCommitInfo)).toBeNull();
  });

  // ── filteredTags derived store ────────────────────────────────────────

  it("filteredTags returns all tags when filter is empty", async () => {
    mockInvokeResponse("list_tags_paginated", MOCK_TAGS);
    await refreshTags();

    tagFilter.set("");
    expect(get(filteredTags)).toHaveLength(3);
  });

  it("filteredTags filters by name substring", async () => {
    mockInvokeResponse("list_tags_paginated", MOCK_TAGS);
    await refreshTags();

    tagFilter.set("v1");
    const filtered = get(filteredTags);
    expect(filtered).toHaveLength(2);
    expect(filtered.every((t) => t.name.startsWith("v1"))).toBe(true);
  });

  it("filteredTags is case-insensitive", async () => {
    mockInvokeResponse("list_tags_paginated", MOCK_TAGS);
    await refreshTags();

    tagFilter.set("RC");
    const filtered = get(filteredTags);
    expect(filtered).toHaveLength(1);
    expect(filtered[0].name).toBe("v2.0.0-rc1");
  });

  // ── selectedTagInfo derived store ─────────────────────────────────────

  it("selectedTagInfo returns the TagInfo for the selected name", async () => {
    mockInvokeResponse("list_tags_paginated", MOCK_TAGS);
    await refreshTags();

    selectedTagName.set("v1.1.0");

    const info = get(selectedTagInfo);
    expect(info).not.toBeNull();
    expect(info!.commit_oid).toBe("bbbb2222");
    expect(info!.annotated).toBe(true);
  });

  // ── selectTag ───────────────────────────────────────────────────────

  it("selectTag loads commit detail, stats, and files", async () => {
    tags.set(MOCK_TAGS);
    mockInvokeResponse("get_commit_detail", MOCK_COMMIT_INFO);
    mockInvokeResponse("get_commit_stats", MOCK_COMMIT_STATS);
    mockInvokeResponse("get_commit_files", MOCK_COMMIT_FILES);

    selectTag("v1.0.0");

    // Wait for async detail loading
    await new Promise((r) => setTimeout(r, 20));

    expect(get(selectedTagName)).toBe("v1.0.0");
    expect(get(loadingDetail)).toBe(false);

    const info = get(selectedCommitInfo);
    expect(info).not.toBeNull();
    expect(info!.summary).toBe("feat: first stable release");

    const stats = get(selectedCommitStats);
    expect(stats).not.toBeNull();
    expect(stats!.files_changed).toBe(5);

    const files = get(selectedCommitFiles);
    expect(files).not.toBeNull();
    expect(files!).toHaveLength(2);
  });

  it("selectTag sets selectedTagName synchronously before detail loads", () => {
    tags.set(MOCK_TAGS);
    mockInvokeResponse("get_commit_detail", MOCK_COMMIT_INFO);
    mockInvokeResponse("get_commit_stats", MOCK_COMMIT_STATS);
    mockInvokeResponse("get_commit_files", MOCK_COMMIT_FILES);

    selectTag("v1.0.0");

    expect(get(selectedTagName)).toBe("v1.0.0");
  });

  // ── doCreateTag ──────────────────────────────────────────────────────

  it("doCreateTag calls IPC and refreshes tags", async () => {
    mockInvokeResponse("create_tag", undefined);
    const updated: TagInfo[] = [
      ...MOCK_TAGS,
      {
        name: "v3.0.0",
        object_oid: "tagobj4444",
        commit_oid: "dddd4444",
        annotated: true,
        message: "Major release",
        tagger_name: "Alice",
        tagger_email: "alice@example.com",
        date: "2024-04-01T00:00:00Z",
      },
    ];
    mockInvokeResponse("list_tags_paginated", updated);

    await doCreateTag("v3.0.0", "dddd4444", "Major release");

    expect(get(tags)).toHaveLength(4);
    expect(get(tags).find((t) => t.name === "v3.0.0")).toBeDefined();
  });

  // ── doDeleteTag ──────────────────────────────────────────────────────

  it("doDeleteTag removes the tag and refreshes", async () => {
    tags.set(MOCK_TAGS);
    mockInvokeResponse("delete_tag", undefined);
    mockInvokeResponse("list_tags_paginated", [MOCK_TAGS[0], MOCK_TAGS[2]]);

    await doDeleteTag("v1.1.0");

    expect(get(tags)).toHaveLength(2);
    expect(get(tags).find((t) => t.name === "v1.1.0")).toBeUndefined();
  });

  it("doDeleteTag clears selection when the selected tag is deleted", async () => {
    tags.set(MOCK_TAGS);
    selectedTagName.set("v1.0.0");
    selectedCommitInfo.set(MOCK_COMMIT_INFO);

    mockInvokeResponse("delete_tag", undefined);
    mockInvokeResponse("list_tags_paginated", [MOCK_TAGS[1], MOCK_TAGS[2]]);

    await doDeleteTag("v1.0.0");

    expect(get(selectedTagName)).toBeNull();
    expect(get(selectedCommitInfo)).toBeNull();
    expect(get(selectedCommitStats)).toBeNull();
    expect(get(selectedCommitFiles)).toBeNull();
  });

  // ── clearTagState ────────────────────────────────────────────────────

  it("clearTagState resets all tag state", () => {
    tags.set(MOCK_TAGS);
    selectedTagName.set("v1.0.0");
    selectedCommitInfo.set(MOCK_COMMIT_INFO);
    selectedCommitStats.set(MOCK_COMMIT_STATS);
    selectedCommitFiles.set(MOCK_COMMIT_FILES);
    tagFilter.set("v1");
    hasMoreTags.set(true);

    clearTagState();

    expect(get(tags)).toHaveLength(0);
    expect(get(selectedTagName)).toBeNull();
    expect(get(selectedCommitInfo)).toBeNull();
    expect(get(selectedCommitStats)).toBeNull();
    expect(get(selectedCommitFiles)).toBeNull();
    expect(get(tagFilter)).toBe("");
    expect(get(hasMoreTags)).toBe(false);
  });
});
