/**
 * E2E: Staging and commit workflow
 *
 * Tests the full changes store data flow: loading statuses, staging files,
 * creating a commit, and verifying state is refreshed afterwards.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { invokeMock, mockInvokeResponse } from "../setup";
import type { FileStatus, FileDiff } from "$lib/types";

import {
  fileStatuses,
  unstagedDiffs,
  stagedDiffs,
  commitMessage,
  refreshStatuses,
  refreshDiffs,
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

const MOCK_WORKDIR_DIFF: FileDiff[] = [
  {
    path: "src/app.ts",
    old_path: null,
    status: "modified",
    hunks: [],
    additions: 5,
    deletions: 2,
  },
];

const MOCK_INDEX_DIFF: FileDiff[] = [
  {
    path: "src/utils.ts",
    old_path: null,
    status: "modified",
    hunks: [],
    additions: 3,
    deletions: 1,
  },
];

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

  // ── refreshDiffs ────────────────────────────────────────────────────

  it("refreshDiffs populates unstagedDiffs and stagedDiffs", async () => {
    mockInvokeResponse("get_diff_workdir", MOCK_WORKDIR_DIFF);
    mockInvokeResponse("get_diff_index", MOCK_INDEX_DIFF);

    await refreshDiffs();

    const unstaged = get(unstagedDiffs);
    const staged = get(stagedDiffs);
    expect(unstaged).toHaveLength(1);
    expect(unstaged[0].path).toBe("src/app.ts");
    expect(staged).toHaveLength(1);
    expect(staged[0].path).toBe("src/utils.ts");
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
    unstagedDiffs.set(MOCK_WORKDIR_DIFF);
    stagedDiffs.set(MOCK_INDEX_DIFF);

    clearChangesState();

    expect(get(fileStatuses)).toHaveLength(0);
    expect(get(unstagedDiffs)).toHaveLength(0);
    expect(get(stagedDiffs)).toHaveLength(0);
  });
});
