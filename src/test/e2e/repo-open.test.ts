/**
 * E2E: Repository open workflow
 *
 * Tests the data flow from calling openRepo() through IPC into the repo
 * store and file-statuses store. All Tauri IPC is mocked via setup.ts.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { mockInvokeResponse } from "../setup";
import type { RepoInfo, BranchInfo, FileStatus, ConflictStatus } from "$lib/types";

// Stores under test (imported after mock registration in setup.ts)
import {
  repoInfo,
  branches,
  isLoading,
  error,
  openRepo,
} from "$lib/stores/repo";
import { fileStatuses } from "$lib/stores/changes";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const MOCK_REPO_INFO: RepoInfo = {
  path: "/home/user/my-repo",
  head_branch: "main",
  head_oid: "abc123def456abc123def456abc123def456abc1",
  branch_count: 3,
};

const MOCK_BRANCHES: BranchInfo[] = [
  { name: "main", is_head: true, is_remote: false, oid: "abc123def456abc123def456abc123def456abc1", upstream: null, ahead: 0, behind: 0 },
  { name: "feature/cool", is_head: false, is_remote: false, oid: "def456abc123def456abc123def456abc123def4", upstream: null, ahead: 0, behind: 0 },
  { name: "origin/main", is_head: false, is_remote: true, oid: "abc123def456abc123def456abc123def456abc1", upstream: null, ahead: 0, behind: 0 },
];

const MOCK_FILE_STATUSES: FileStatus[] = [
  { path: "src/main.ts", status: "modified", is_staged: false },
  { path: "README.md", status: "added", is_staged: true },
];

/** Register all IPC mocks needed for a full openRepo() call. */
function setupOpenRepoMocks() {
  mockInvokeResponse("open_repo", MOCK_REPO_INFO);
  mockInvokeResponse("get_branches", MOCK_BRANCHES);
  mockInvokeResponse("detect_project", undefined);
  mockInvokeResponse("get_provider_status", { providers: [], active_index: null });
  mockInvokeResponse("get_file_statuses", MOCK_FILE_STATUSES);
  mockInvokeResponse("get_conflict_status", { state: "none", conflicted_files: [], can_continue: false } satisfies ConflictStatus);
  mockInvokeResponse("get_user_identities", []);
  mockInvokeResponse("list_submodules", []);
  mockInvokeResponse("get_status_summary", { ahead: 0, behind: 0, staged: 1, unstaged: 1, untracked: 0, conflicted: 0, stash_count: 0 });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("repo-open workflow", () => {
  beforeEach(() => {
    // Reset store state before each test
    repoInfo.set(null);
    branches.set([]);
    isLoading.set(false);
    error.set(null);
    fileStatuses.set([]);
  });

  it("starts with null repoInfo", () => {
    expect(get(repoInfo)).toBeNull();
  });

  it("sets isLoading to true then false after openRepo()", async () => {
    setupOpenRepoMocks();
    const loadingStates: boolean[] = [];
    const unsub = isLoading.subscribe((v) => loadingStates.push(v));

    await openRepo("/home/user/my-repo");
    unsub();

    // Should have gone true then false
    expect(loadingStates).toContain(true);
    expect(loadingStates[loadingStates.length - 1]).toBe(false);
  });

  it("populates repoInfo after successful openRepo()", async () => {
    setupOpenRepoMocks();
    await openRepo("/home/user/my-repo");

    const info = get(repoInfo);
    expect(info).not.toBeNull();
    expect(info!.path).toBe("/home/user/my-repo");
    expect(info!.head_branch).toBe("main");
    expect(info!.branch_count).toBe(3);
  });

  it("populates branches store after openRepo()", async () => {
    setupOpenRepoMocks();
    await openRepo("/home/user/my-repo");

    const list = get(branches);
    expect(list).toHaveLength(3);
    expect(list[0].name).toBe("main");
    expect(list[0].is_head).toBe(true);
  });

  it("populates fileStatuses store after openRepo()", async () => {
    setupOpenRepoMocks();
    await openRepo("/home/user/my-repo");

    const statuses = get(fileStatuses);
    expect(statuses).toHaveLength(2);
    expect(statuses[0].path).toBe("src/main.ts");
    expect(statuses[0].is_staged).toBe(false);
    expect(statuses[1].path).toBe("README.md");
    expect(statuses[1].is_staged).toBe(true);
  });

  it("sets error and clears loading when open_repo throws", async () => {
    mockInvokeResponse("open_repo", () => { throw new Error("not a git repo"); });

    await openRepo("/not/a/repo");

    expect(get(error)).toBeTruthy();
    expect(get(isLoading)).toBe(false);
    expect(get(repoInfo)).toBeNull();
  });

  it("clears error state on a successful openRepo() after a failed one", async () => {
    mockInvokeResponse("open_repo", () => { throw new Error("fail"); });
    await openRepo("/bad/path");
    expect(get(error)).toBeTruthy();

    setupOpenRepoMocks();
    await openRepo("/home/user/my-repo");

    expect(get(error)).toBeNull();
    expect(get(repoInfo)).not.toBeNull();
  });
});
