import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

vi.mock("$lib/api/tauri", () => ({
  openProject: vi.fn(),
  closeProject: vi.fn().mockResolvedValue(undefined),
  switchProject: vi.fn(),
  getOpenProjects: vi.fn().mockResolvedValue([]),
  getActiveProjectIndex: vi.fn().mockResolvedValue(null),
  restoreProjects: vi.fn().mockResolvedValue([]),
  getBranches: vi.fn().mockResolvedValue([]),
  getStatusSummary: vi.fn().mockResolvedValue({
    ahead: 0, behind: 0, staged: 0, unstaged: 0, untracked: 0, stash_count: 0,
  }),
  detectProject: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ setTitle: vi.fn() }),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

vi.mock("$lib/stores/initRepoDialog", () => ({
  requestOpenInitRepoDialog: vi.fn(),
  closeInitRepoDialog: vi.fn(),
}));

import { closeTab } from "../projects";
import { openTabs, activeTabIndex } from "../tabs";
import { repoInfo, branches } from "../repo";
import type { ProjectInfo, RepoInfo, BranchInfo } from "$lib/types";

const project: ProjectInfo = {
  name: "demo",
  path: "/tmp/demo",
  head_branch: "main",
  change_count: 0,
  is_worktree: false,
};

const fakeRepo: RepoInfo = {
  path: "/tmp/demo",
  head_branch: "main",
  head_oid: "deadbeef",
  branch_count: 1,
};

const fakeBranches: BranchInfo[] = [
  {
    name: "main",
    is_head: true,
    is_remote: false,
    oid: "deadbeef",
    upstream: null,
    ahead: 0,
    behind: 0,
    upstream_gone: false,
  },
];

describe("closeTab — close-to-empty state cleanup", () => {
  beforeEach(() => {
    openTabs.set([{ kind: "project", project }]);
    activeTabIndex.set(0);
    repoInfo.set(fakeRepo);
    branches.set(fakeBranches);
  });

  it("clears repoInfo and branches when the last tab is closed", async () => {
    await closeTab(0);

    expect(get(openTabs)).toHaveLength(0);
    expect(get(activeTabIndex)).toBe(-1);
    expect(get(repoInfo)).toBeNull();
    expect(get(branches)).toEqual([]);
  });
});
