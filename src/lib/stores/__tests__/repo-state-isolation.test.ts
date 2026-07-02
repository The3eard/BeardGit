/**
 * Two-repo state-isolation regression (spec 08 success criterion).
 *
 * Opens repo A and repo B through the real tab-switch machinery
 * (`switchToTab` in projects.ts), mutates B, switches back to A, and
 * asserts A's per-repo view state is untouched by activity in B:
 *   - branch list (branches.ts `branches`)
 *   - changes checkbox selection (changesSelection)
 *   - graph selection (graph.ts `selectedOid`) — see note below.
 *
 * Peripheral stores that the switch path only clears/refreshes (provider,
 * remotes, conflict, tags, stashes, …) are mocked to no-ops so the test
 * exercises only the stores under migration. All git IPC is driven through
 * the global `invoke` mock keyed on the currently-active repo path.
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { get, writable } from "svelte/store";
import { mockInvokeResponse } from "../../../test/setup";
import type { ProjectInfo, RepoInfo, BranchInfo } from "$lib/types";

// ── Peripheral stores mocked to no-ops (not under test) ───────────────
vi.mock("$lib/stores/provider", () => ({
  checkStatus: vi.fn(),
  stopAllPolling: vi.fn(),
  ciRuns: writable([]),
  selectedCiRun: writable(null),
  selectedCiRunId: writable(null),
  jobLog: writable(null),
  hasMoreCiRuns: writable(false),
  loadingDetail: writable(false),
  selectedJobId: writable(null),
  jobLogUnavailable: writable(false),
  loadingJobLog: writable(false),
}));
vi.mock("$lib/stores/project-cache", () => ({
  loadProjectSnapshot: vi.fn().mockResolvedValue(null),
  saveCurrentSnapshot: vi.fn().mockResolvedValue(undefined),
  restorePersistedViewport: vi.fn().mockReturnValue(false),
}));
vi.mock("$lib/stores/tags", () => ({ clearTagState: vi.fn(), refreshTags: vi.fn() }));
vi.mock("$lib/stores/stashes", () => ({ clearStashState: vi.fn(), refreshStashes: vi.fn() }));
vi.mock("$lib/stores/blame", () => ({ clearBlameState: vi.fn() }));
vi.mock("$lib/stores/worktrees", () => ({ clearWorktreeState: vi.fn(), refreshWorktrees: vi.fn() }));
vi.mock("$lib/stores/mr-pr", () => ({ clearMrPrState: vi.fn() }));
vi.mock("$lib/stores/issues", () => ({ clearIssueState: vi.fn() }));
vi.mock("$lib/stores/releases", () => ({ clearReleaseState: vi.fn() }));
vi.mock("$lib/stores/reflog", () => ({ clearReflogState: vi.fn(), loadReflog: vi.fn() }));
vi.mock("$lib/stores/conflict", () => ({ refreshConflictStatus: vi.fn() }));
vi.mock("$lib/stores/remotes", () => ({ refreshRemotes: vi.fn() }));
vi.mock("$lib/stores/mutations", () => ({ flushPendingForActiveProject: vi.fn() }));
vi.mock("$lib/stores/initRepoDialog", () => ({
  requestOpenInitRepoDialog: vi.fn(),
  closeInitRepoDialog: vi.fn(),
}));

import { switchToTab } from "../projects";
import { openTabs, activeTabIndex } from "../tabs";
import { branches, refreshBranches } from "../branches";
import { unstagedSelection } from "../changesSelection";
import { selectedOid, clearGraphState, viewport } from "../graph";
import { __resetRepoStateForTests } from "../repo-state";

const A: ProjectInfo = { name: "repoA", path: "/tmp/A", head_branch: "main", change_count: 0, is_worktree: false };
const B: ProjectInfo = { name: "repoB", path: "/tmp/B", head_branch: "main", change_count: 0, is_worktree: false };

function branch(name: string): BranchInfo {
  return { name, is_head: name.endsWith("main"), is_remote: false, oid: "0".repeat(40), upstream: null, ahead: 0, behind: 0 };
}

const branchesByPath: Record<string, BranchInfo[]> = {
  "/tmp/A": [branch("main"), branch("feature-A")],
  "/tmp/B": [branch("main"), branch("dev-B"), branch("wip-B")],
};

const repoInfoByIndex: Record<number, RepoInfo> = {
  0: { path: A.path, head_branch: "main", head_oid: "a".repeat(40), branch_count: 2 },
  1: { path: B.path, head_branch: "main", head_oid: "b".repeat(40), branch_count: 3 },
};

/** The repo the git IPC mock should answer for (flipped before each switch). */
let activeMockPath = A.path;

function seedMocks() {
  mockInvokeResponse("switch_project", (args?: Record<string, unknown>) => {
    const idx = Number(args?.projectIndex ?? args?.project_index ?? 0);
    return repoInfoByIndex[idx];
  });
  mockInvokeResponse("get_open_projects", [A, B]);
  mockInvokeResponse("get_branches", () => branchesByPath[activeMockPath] ?? []);
  mockInvokeResponse("get_file_statuses", []);
  mockInvokeResponse("get_user_identities", []);
  mockInvokeResponse("get_status_summary", {
    ahead: 0, behind: 0, staged: 0, unstaged: 0, untracked: 0, conflicted: 0, stash_count: 0,
  });
  mockInvokeResponse("detect_project", null);
  mockInvokeResponse("get_remotes", []);
}

/** Switch to a project tab, driving the IPC mock to answer for that repo. */
async function switchTo(project: ProjectInfo, tabIndex: number) {
  activeMockPath = project.path;
  await switchToTab(tabIndex);
}

describe("two-repo state isolation (spec 08)", () => {
  beforeEach(() => {
    seedMocks();
    // Fresh container so per-repo slices don't bleed across cases.
    __resetRepoStateForTests();
    openTabs.set([{ kind: "project", project: A }, { kind: "project", project: B }]);
    activeTabIndex.set(-1);
    activeMockPath = A.path;
    // Graph state is still a module-level singleton (not yet migrated).
    clearGraphState();
    viewport.set(null);
  });

  it("keeps repo A's branch list after mutating in repo B", async () => {
    await switchTo(A, 0);
    await refreshBranches(); // branch panel populates A's list
    expect(get(branches).map((b) => b.name)).toEqual(["main", "feature-A"]);

    await switchTo(B, 1);
    await refreshBranches(); // B's panel loads B's list
    expect(get(branches).map((b) => b.name)).toEqual(["main", "dev-B", "wip-B"]);

    // Switch back to A WITHOUT re-fetching — the list must already be A's.
    await switchTo(A, 0);
    expect(get(branches).map((b) => b.name)).toEqual(["main", "feature-A"]);
  });

  it("keeps repo A's changes selection after mutating in repo B", async () => {
    await switchTo(A, 0);
    unstagedSelection.set(new Set(["a-file.txt"]));

    await switchTo(B, 1);
    // Interact in B: a different selection.
    unstagedSelection.set(new Set(["b-file.txt"]));

    await switchTo(A, 0);
    expect(get(unstagedSelection)).toEqual(new Set(["a-file.txt"]));
  });

  // Graph selection is NOT isolated per-repo yet: `clearGraphState` wipes
  // `selectedOid` on every switch. graph.ts migrates to the RepoState
  // container in spec 08 step 3 — unskip and this must pass then.
  it.skip("keeps repo A's graph selection after mutating in repo B", async () => {
    await switchTo(A, 0);
    selectedOid.set("commit-a");

    await switchTo(B, 1);
    selectedOid.set("commit-b");

    await switchTo(A, 0);
    expect(get(selectedOid)).toBe("commit-a");
  });
});
