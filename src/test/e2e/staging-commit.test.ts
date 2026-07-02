/**
 * E2E: Staging and commit workflow
 *
 * Tests the full changes store data flow: loading statuses, staging files,
 * creating a commit, and verifying state is refreshed afterwards.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { invokeMock, mockInvokeResponse } from "../setup";
import type { FileStatus, FileDiff, FileDiffStat } from "$lib/types";

import {
  fileStatuses,
  unstagedStats,
  stagedStats,
  openStagingFile,
  openStagingDiff,
  commitMessage,
  refreshStatuses,
  refreshDiffs,
  loadStagingDiff,
  closeStagingDiff,
  stageFiles,
  unstageFiles,
  stageAll,
  unstageAll,
  commit,
  clearChangesState,
} from "$lib/stores/changes";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const UNSTAGED_STATUSES: FileStatus[] = [
  { path: "src/app.ts", status: "modified", is_staged: false },
  { path: "src/utils.ts", status: "modified", is_staged: false },
];

const STAGED_STATUSES: FileStatus[] = [
  { path: "src/app.ts", status: "modified", is_staged: true },
  { path: "src/utils.ts", status: "modified", is_staged: true },
];

const PARTIAL_STAGED_STATUSES: FileStatus[] = [
  { path: "src/app.ts", status: "modified", is_staged: true },
  { path: "src/utils.ts", status: "modified", is_staged: false },
];

const EMPTY_STATUSES: FileStatus[] = [];

const MOCK_WORKDIR_STATS: FileDiffStat[] = [
  { path: "src/app.ts", old_path: null, status: "modified", additions: 5, deletions: 2 },
];

const MOCK_INDEX_STATS: FileDiffStat[] = [
  { path: "src/utils.ts", old_path: null, status: "modified", additions: 3, deletions: 1 },
];

const MOCK_FILE_DIFF: FileDiff = {
  path: "src/app.ts",
  old_path: null,
  status: "modified",
  hunks: [
    {
      header: "@@ -1,2 +1,3 @@",
      old_start: 1,
      old_lines: 2,
      new_start: 1,
      new_lines: 3,
      lines: [
        { origin: "+", content: "added\n", old_lineno: null, new_lineno: 1 },
      ],
    },
  ],
  additions: 5,
  deletions: 2,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("staging and commit workflow", () => {
  beforeEach(() => {
    clearChangesState();
    commitMessage.set("");
  });

  // ── refreshStatuses ─────────────────────────────────────────────────

  it("refreshStatuses populates fileStatuses store", async () => {
    mockInvokeResponse("get_file_statuses", UNSTAGED_STATUSES);

    await refreshStatuses();

    const statuses = get(fileStatuses);
    expect(statuses).toHaveLength(2);
    expect(statuses[0].path).toBe("src/app.ts");
    expect(statuses[0].is_staged).toBe(false);
  });

  it("refreshStatuses handles empty status list", async () => {
    mockInvokeResponse("get_file_statuses", EMPTY_STATUSES);

    await refreshStatuses();

    expect(get(fileStatuses)).toHaveLength(0);
  });

  // ── refreshDiffs (stats-only + lazy per-file fetch) ─────────────────

  it("refreshDiffs fetches stats only — never the full hunk sets", async () => {
    mockInvokeResponse("get_diff_stats_workdir", MOCK_WORKDIR_STATS);
    mockInvokeResponse("get_diff_stats_index", MOCK_INDEX_STATS);

    await refreshDiffs();

    // Stats stores populated with the light per-file summaries.
    expect(get(unstagedStats)).toHaveLength(1);
    expect(get(unstagedStats)[0].path).toBe("src/app.ts");
    expect(get(unstagedStats)[0].additions).toBe(5);
    expect(get(stagedStats)).toHaveLength(1);
    expect(get(stagedStats)[0].path).toBe("src/utils.ts");

    // The heavy full-hunk endpoints must NOT be called on refresh.
    expect(invokeMock.mock.calls.some((c) => c[0] === "get_diff_workdir")).toBe(false);
    expect(invokeMock.mock.calls.some((c) => c[0] === "get_diff_index")).toBe(false);
    // No file is open, so no per-file diff is fetched either.
    expect(invokeMock.mock.calls.some((c) => c[0] === "get_diff_file")).toBe(false);
  });

  it("loadStagingDiff fetches a single file's hunks on selection", async () => {
    mockInvokeResponse("get_diff_file", MOCK_FILE_DIFF);

    await loadStagingDiff("src/app.ts", false);

    const call = invokeMock.mock.calls.find((c) => c[0] === "get_diff_file");
    expect(call).toBeDefined();
    expect(call?.[1]).toEqual({ path: "src/app.ts", staged: false });
    expect(get(openStagingFile)).toEqual({ path: "src/app.ts", isStaged: false });
    expect(get(openStagingDiff)?.hunks).toHaveLength(1);
  });

  it("refreshDiffs re-fetches the open file's full diff", async () => {
    mockInvokeResponse("get_diff_stats_workdir", MOCK_WORKDIR_STATS);
    mockInvokeResponse("get_diff_stats_index", MOCK_INDEX_STATS);
    mockInvokeResponse("get_diff_file", MOCK_FILE_DIFF);

    // Open a file, then simulate a mutation refresh.
    await loadStagingDiff("src/app.ts", false);
    invokeMock.mockClear();
    await refreshDiffs();

    // Stats fetched AND the open file re-fetched (so the pane stays live).
    expect(invokeMock.mock.calls.some((c) => c[0] === "get_diff_stats_workdir")).toBe(true);
    const fileCall = invokeMock.mock.calls.find((c) => c[0] === "get_diff_file");
    expect(fileCall?.[1]).toEqual({ path: "src/app.ts", staged: false });
  });

  it("closeStagingDiff clears the open file and diff", async () => {
    mockInvokeResponse("get_diff_file", MOCK_FILE_DIFF);
    await loadStagingDiff("src/app.ts", false);
    closeStagingDiff();
    expect(get(openStagingFile)).toBeNull();
    expect(get(openStagingDiff)).toBeNull();
  });

  // ── stageFiles ──────────────────────────────────────────────────────
  //
  // Refresh of statuses/diffs is now driven by the `project-mutated`
  // event listener (see mutations.ts) — the store mutation functions no
  // longer refresh in-line. These tests therefore assert the IPC was
  // invoked with the correct payload; the refresh path is covered by
  // mutations.test.ts.

  it("stageFiles invokes stage_files IPC with the given paths", async () => {
    mockInvokeResponse("stage_files", undefined);

    await stageFiles(["src/app.ts"]);

    const stageCall = invokeMock.mock.calls.find((c) => c[0] === "stage_files");
    expect(stageCall).toBeDefined();
    expect(stageCall?.[1]).toEqual({ paths: ["src/app.ts"] });
  });

  it("stageFiles handles multiple paths", async () => {
    mockInvokeResponse("stage_files", undefined);

    await stageFiles(["src/app.ts", "src/utils.ts"]);

    const stageCall = invokeMock.mock.calls.find((c) => c[0] === "stage_files");
    expect(stageCall?.[1]).toEqual({ paths: ["src/app.ts", "src/utils.ts"] });
  });

  // ── unstageFiles ────────────────────────────────────────────────────

  it("unstageFiles invokes unstage_files IPC with the given paths", async () => {
    mockInvokeResponse("unstage_files", undefined);

    await unstageFiles(["src/app.ts"]);

    const unstageCall = invokeMock.mock.calls.find(
      (c) => c[0] === "unstage_files",
    );
    expect(unstageCall).toBeDefined();
    expect(unstageCall?.[1]).toEqual({ paths: ["src/app.ts"] });
  });

  // ── stageAll / unstageAll ────────────────────────────────────────────

  it("stageAll invokes stage_all IPC", async () => {
    mockInvokeResponse("stage_all", undefined);

    await stageAll();

    expect(invokeMock.mock.calls.some((c) => c[0] === "stage_all")).toBe(true);
  });

  it("unstageAll invokes unstage_all IPC", async () => {
    mockInvokeResponse("unstage_all", undefined);

    await unstageAll();

    expect(invokeMock.mock.calls.some((c) => c[0] === "unstage_all")).toBe(true);
  });

  // ── commit ──────────────────────────────────────────────────────────

  it("commit calls IPC and clears the message draft", async () => {
    mockInvokeResponse("create_commit", "newcommitoid123");

    commitMessage.set("fix: resolve null pointer");
    await commit("fix: resolve null pointer");

    expect(get(commitMessage)).toBe("");
    const commitCall = invokeMock.mock.calls.find(
      (c) => c[0] === "create_commit",
    );
    expect(commitCall?.[1]).toEqual({ message: "fix: resolve null pointer" });
  });

  it("commit does not clear message on IPC failure", async () => {
    mockInvokeResponse("create_commit", () => { throw new Error("nothing staged"); });

    commitMessage.set("feat: add feature");
    await expect(commit("feat: add feature")).rejects.toThrow();

    // Message must still be there so user doesn't lose their work
    expect(get(commitMessage)).toBe("feat: add feature");
  });

  // ── clearChangesState ────────────────────────────────────────────────

  it("clearChangesState resets all stores to empty", () => {
    fileStatuses.set(STAGED_STATUSES);
    unstagedStats.set(MOCK_WORKDIR_STATS);
    stagedStats.set(MOCK_INDEX_STATS);
    openStagingFile.set({ path: "src/app.ts", isStaged: false });

    clearChangesState();

    expect(get(fileStatuses)).toHaveLength(0);
    expect(get(unstagedStats)).toHaveLength(0);
    expect(get(stagedStats)).toHaveLength(0);
    expect(get(openStagingFile)).toBeNull();
    expect(get(openStagingDiff)).toBeNull();
  });
});
