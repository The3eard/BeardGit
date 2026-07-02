/**
 * E2E: Branch management workflow
 *
 * Tests refreshBranches, selectBranch, doCheckout, doDeleteBranch, and
 * doMergeBranch via the branches store with mocked IPC.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { invokeMock, mockInvokeResponse } from "../setup";
import type { BranchInfo, CommitInfo } from "$lib/types";

import {
  branches,
  branchesLoading,
  selectedBranchName,
  selectedBranchCommits,
  loadingDetail,
  localBranches,
  remoteBranches,
  selectedBranchInfo,
  refreshBranches,
  selectBranch,
  doCheckout,
  doDeleteBranch,
  doMergeBranch,
  clearBranchState,
} from "$lib/stores/branches";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const MOCK_BRANCHES: BranchInfo[] = [
  { name: "main", is_head: true, is_remote: false, oid: "aaaa1111", upstream: null, ahead: 0, behind: 0, upstream_gone: false },
  { name: "feature/auth", is_head: false, is_remote: false, oid: "bbbb2222", upstream: null, ahead: 0, behind: 0, upstream_gone: false },
  { name: "origin/main", is_head: false, is_remote: true, oid: "aaaa1111", upstream: null, ahead: 0, behind: 0, upstream_gone: false },
  { name: "origin/feature/auth", is_head: false, is_remote: true, oid: "bbbb2222", upstream: null, ahead: 0, behind: 0, upstream_gone: false },
];

const MOCK_COMMITS: CommitInfo[] = [
  {
    oid: "aaaa1111",
    summary: "feat: initial commit",
    body: "",
    author: "Alice",
    email: "alice@example.com",
    timestamp: 1700000000,
    parents: [],
    refs: ["HEAD -> main", "origin/main"],
  },
  {
    oid: "bbbb2222",
    summary: "chore: add tests",
    body: "",
    author: "Bob",
    email: "bob@example.com",
    timestamp: 1700001000,
    parents: ["aaaa1111"],
    refs: [],
  },
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("branch operations workflow", () => {
  beforeEach(() => {
    clearBranchState();
    branches.set([]);
  });

  // ── refreshBranches ─────────────────────────────────────────────────

  it("refreshBranches populates branches store", async () => {
    mockInvokeResponse("get_branches", MOCK_BRANCHES);

    await refreshBranches();

    const list = get(branches);
    expect(list).toHaveLength(4);
    expect(list[0].name).toBe("main");
  });

  it("refreshBranches sets loading true then false", async () => {
    mockInvokeResponse("get_branches", MOCK_BRANCHES);

    const loadingHistory: boolean[] = [];
    const unsub = branchesLoading.subscribe((v) => loadingHistory.push(v));

    await refreshBranches();
    unsub();

    expect(loadingHistory).toContain(true);
    expect(loadingHistory[loadingHistory.length - 1]).toBe(false);
  });

  it("refreshBranches sets empty list on error", async () => {
    mockInvokeResponse("get_branches", () => { throw new Error("git error"); });

    await refreshBranches();

    expect(get(branches)).toHaveLength(0);
    expect(get(branchesLoading)).toBe(false);
  });

  it("refreshBranches clears selection when selected branch no longer exists", async () => {
    // Set up an initial state with a selected branch
    branches.set(MOCK_BRANCHES);
    selectedBranchName.set("feature/auth");

    // Return branches without feature/auth
    const remaining: BranchInfo[] = [MOCK_BRANCHES[0], MOCK_BRANCHES[2]];
    mockInvokeResponse("get_branches", remaining);

    await refreshBranches();

    expect(get(selectedBranchName)).toBeNull();
    expect(get(selectedBranchCommits)).toHaveLength(0);
  });

  // ── derived stores ──────────────────────────────────────────────────

  it("localBranches filters remote branches", async () => {
    mockInvokeResponse("get_branches", MOCK_BRANCHES);
    await refreshBranches();

    const local = get(localBranches);
    expect(local).toHaveLength(2);
    expect(local.every((b) => !b.is_remote)).toBe(true);
  });

  it("remoteBranches filters local branches", async () => {
    mockInvokeResponse("get_branches", MOCK_BRANCHES);
    await refreshBranches();

    const remote = get(remoteBranches);
    expect(remote).toHaveLength(2);
    expect(remote.every((b) => b.is_remote)).toBe(true);
  });

  it("selectedBranchInfo returns the matching BranchInfo", async () => {
    mockInvokeResponse("get_branches", MOCK_BRANCHES);
    await refreshBranches();

    selectedBranchName.set("feature/auth");

    const info = get(selectedBranchInfo);
    expect(info).not.toBeNull();
    expect(info!.oid).toBe("bbbb2222");
  });

  // ── selectBranch ────────────────────────────────────────────────────

  it("selectBranch loads commits for the branch", async () => {
    branches.set(MOCK_BRANCHES);
    mockInvokeResponse("get_branch_commits", MOCK_COMMITS);

    selectBranch("main");

    // Wait for async commit loading
    await new Promise((r) => setTimeout(r, 20));

    expect(get(selectedBranchName)).toBe("main");
    expect(get(selectedBranchCommits)).toHaveLength(2);
    expect(get(loadingDetail)).toBe(false);
  });

  it("selectBranch sets selectedBranchName immediately (synchronous)", () => {
    branches.set(MOCK_BRANCHES);
    mockInvokeResponse("get_branch_commits", MOCK_COMMITS);

    selectBranch("feature/auth");

    // The name must be set synchronously, before commits load
    expect(get(selectedBranchName)).toBe("feature/auth");
  });

  // ── doCheckout ──────────────────────────────────────────────────────
  //
  // Branch list refresh is now driven by the `project-mutated` event
  // (see mutations.ts) rather than an inline `refreshBranches()` call —
  // these tests therefore assert the IPC invocation. The refresh path
  // itself is covered by `mutations.test.ts`.

  it("doCheckout invokes checkout_branch IPC with the branch name", async () => {
    branches.set(MOCK_BRANCHES);
    mockInvokeResponse("checkout_branch", undefined);

    await doCheckout("feature/auth");

    const call = invokeMock.mock.calls.find((c) => c[0] === "checkout_branch");
    expect(call).toBeDefined();
    expect(call?.[1]).toEqual({ name: "feature/auth" });
  });

  // ── doDeleteBranch ───────────────────────────────────────────────────

  it("doDeleteBranch invokes delete_branch IPC and clears selection of the deleted branch", async () => {
    branches.set(MOCK_BRANCHES);
    selectedBranchName.set("feature/auth");

    mockInvokeResponse("delete_branch", undefined);

    await doDeleteBranch("feature/auth");

    const call = invokeMock.mock.calls.find((c) => c[0] === "delete_branch");
    expect(call?.[1]).toEqual({ name: "feature/auth", force: false });
    expect(get(selectedBranchName)).toBeNull();
  });

  it("doDeleteBranch does not clear selection when deleting non-selected branch", async () => {
    branches.set(MOCK_BRANCHES);
    selectedBranchName.set("main");

    mockInvokeResponse("delete_branch", undefined);

    await doDeleteBranch("feature/auth");

    expect(get(selectedBranchName)).toBe("main");
  });

  // ── doMergeBranch ────────────────────────────────────────────────────

  it("doMergeBranch invokes merge_branch IPC with the branch name", async () => {
    branches.set(MOCK_BRANCHES);
    mockInvokeResponse("merge_branch", "Fast-forward");

    await doMergeBranch("feature/auth");

    const call = invokeMock.mock.calls.find((c) => c[0] === "merge_branch");
    expect(call?.[1]).toEqual({ branch: "feature/auth" });
  });

  // ── clearBranchState ─────────────────────────────────────────────────

  it("clearBranchState resets all branch state", () => {
    branches.set(MOCK_BRANCHES);
    selectedBranchName.set("main");
    selectedBranchCommits.set(MOCK_COMMITS);
    loadingDetail.set(true);

    clearBranchState();

    expect(get(selectedBranchName)).toBeNull();
    expect(get(selectedBranchCommits)).toHaveLength(0);
    expect(get(loadingDetail)).toBe(false);
  });
});
