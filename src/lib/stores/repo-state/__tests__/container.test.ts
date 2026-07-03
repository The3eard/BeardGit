/**
 * Unit tests for the RepoState container + facade factory (spec 08).
 */

import { describe, it, expect, beforeEach } from "vitest";
import { get, type Writable } from "svelte/store";
import {
  RepoState,
  BranchesSlice,
  ChangesSlice,
  activeField,
  createRepoState,
  dropRepoState,
  setActiveRepoPath,
  getActiveRepoState,
  __resetRepoStateForTests,
} from "..";
import { currentSource } from "../../../components/requests/stores";
import { mrPrList, mrPrFilter, selectedMrPrNumber } from "../../mr-pr";
import { issueList, issueStateFilter, selectedIssueNumber } from "../../issues";
import type { MrPr, Issue } from "../../../types";

describe("RepoState + slices", () => {
  it("aggregates a branches + changes slice per path", () => {
    const rs = new RepoState("/tmp/x");
    expect(rs.path).toBe("/tmp/x");
    expect(rs.branches).toBeInstanceOf(BranchesSlice);
    expect(rs.changes).toBeInstanceOf(ChangesSlice);
  });

  it("BranchesSlice.clear resets selection/detail but keeps the list", () => {
    const s = new BranchesSlice();
    s.list.set([{ name: "main" } as never]);
    s.selectedName.set("main");
    s.selectedCommits.set([{ oid: "a" } as never]);
    s.clear();
    expect(get(s.selectedName)).toBeNull();
    expect(get(s.selectedCommits)).toEqual([]);
    // The list itself is not part of clear() — it survives.
    expect(get(s.list)).toHaveLength(1);
  });

  it("ChangesSlice.clear resets statuses + selection but keeps the commit draft", () => {
    const s = new ChangesSlice();
    s.fileStatuses.set([{ path: "a" } as never]);
    s.unstagedSelection.set(new Set(["a"]));
    s.commitMessage.set("wip");
    s.clear();
    expect(get(s.fileStatuses)).toEqual([]);
    expect(get(s.unstagedSelection)).toEqual(new Set());
    // Commit message draft is intentionally preserved across clear().
    expect(get(s.commitMessage)).toBe("wip");
  });
});

describe("container lifecycle", () => {
  beforeEach(() => __resetRepoStateForTests());

  it("createRepoState is idempotent per path", () => {
    const a1 = createRepoState("/tmp/a");
    const a2 = createRepoState("/tmp/a");
    expect(a1).toBe(a2);
  });

  it("dropRepoState removes the entry so a later create is fresh", () => {
    const a1 = createRepoState("/tmp/a");
    a1.branches.list.set([{ name: "main" } as never]);
    dropRepoState("/tmp/a");
    const a2 = createRepoState("/tmp/a");
    expect(a2).not.toBe(a1);
    expect(get(a2.branches.list)).toEqual([]);
  });
});

describe("activeField facade", () => {
  beforeEach(() => __resetRepoStateForTests());

  const list = () => activeField<string[]>((rs) => rs.branches.list as unknown as Writable<string[]>);

  it("reads/writes route to the active repo's slice and isolate repos", () => {
    createRepoState("/tmp/a");
    createRepoState("/tmp/b");
    const facade = list();

    setActiveRepoPath("/tmp/a");
    facade.set(["a-main"]);
    expect(get(facade)).toEqual(["a-main"]);

    setActiveRepoPath("/tmp/b");
    // B starts empty — A's write did not leak.
    expect(get(facade)).toEqual([]);
    facade.set(["b-dev"]);
    expect(get(facade)).toEqual(["b-dev"]);

    // Switch back: A's value is intact (pointer swap, no restore call).
    setActiveRepoPath("/tmp/a");
    expect(get(facade)).toEqual(["a-main"]);
  });

  it("re-emits to subscribers on active-repo switch", () => {
    createRepoState("/tmp/a");
    createRepoState("/tmp/b");
    const facade = list();
    setActiveRepoPath("/tmp/a");
    facade.set(["a-main"]);
    setActiveRepoPath("/tmp/b");
    facade.set(["b-dev"]);

    const seen: string[][] = [];
    const unsub = facade.subscribe((v) => seen.push(v));
    // Initial emit is B (currently active).
    expect(seen.at(-1)).toEqual(["b-dev"]);
    setActiveRepoPath("/tmp/a");
    expect(seen.at(-1)).toEqual(["a-main"]);
    unsub();
  });

  it("falls back to a detached slice when no repo is active", () => {
    const facade = list();
    setActiveRepoPath(null);
    // Writes/reads land on the detached fallback, so tests that never open
    // a project still behave like a plain writable.
    facade.set(["detached"]);
    expect(get(facade)).toEqual(["detached"]);
    expect(getActiveRepoState().path).toBe("");
  });
});

// Regression for the Requests-panel leak: the selected `.http` source must
// not survive a repo switch (it drove Send/Save against the wrong repo).
describe("requests selection isolation across repos", () => {
  beforeEach(() => __resetRepoStateForTests());

  it("keeps each repo's currentSource and restores it on switch-back", () => {
    createRepoState("/tmp/a");
    createRepoState("/tmp/b");

    setActiveRepoPath("/tmp/a");
    currentSource.set({ kind: "project", path: "users/get.http" });
    expect(get(currentSource)).toEqual({ kind: "project", path: "users/get.http" });

    // Switch to B: A's selection must not leak.
    setActiveRepoPath("/tmp/b");
    expect(get(currentSource)).toBeNull();
    currentSource.set({ kind: "project", path: "orders/list.http" });

    // Switch back to A: its selection is intact (pointer swap, no restore).
    setActiveRepoPath("/tmp/a");
    expect(get(currentSource)).toEqual({ kind: "project", path: "users/get.http" });
  });
});

// Regression for the MR/PR view leak: the list, filter, and selection must
// follow the active tab instead of bleeding across repos (spec 08).
describe("mr-pr view isolation across repos", () => {
  beforeEach(() => __resetRepoStateForTests());

  it("keeps each repo's list, filter, and selection and restores them on switch-back", () => {
    createRepoState("/tmp/a");
    createRepoState("/tmp/b");
    const prA = { number: 1 } as MrPr;
    const prB = { number: 99 } as MrPr;

    setActiveRepoPath("/tmp/a");
    mrPrList.set([prA]);
    mrPrFilter.set("merged");
    selectedMrPrNumber.set(1);

    // Switch to B: A's PR state must not leak — B starts at defaults.
    setActiveRepoPath("/tmp/b");
    expect(get(mrPrList)).toEqual([]);
    expect(get(mrPrFilter)).toBe("open");
    expect(get(selectedMrPrNumber)).toBeNull();
    mrPrList.set([prB]);
    selectedMrPrNumber.set(99);

    // Switch back to A: its state is intact (pointer swap, no restore).
    setActiveRepoPath("/tmp/a");
    expect(get(mrPrList)).toEqual([prA]);
    expect(get(mrPrFilter)).toBe("merged");
    expect(get(selectedMrPrNumber)).toBe(1);
  });
});

// Regression for the Issues view leak: the list, filter, and selection must
// follow the active tab instead of bleeding across repos (spec 08).
describe("issues view isolation across repos", () => {
  beforeEach(() => __resetRepoStateForTests());

  it("keeps each repo's list, filter, and selection and restores them on switch-back", () => {
    createRepoState("/tmp/a");
    createRepoState("/tmp/b");
    const issueA = { number: 1 } as Issue;
    const issueB = { number: 42 } as Issue;

    setActiveRepoPath("/tmp/a");
    issueList.set([issueA]);
    issueStateFilter.set("closed");
    selectedIssueNumber.set(1);

    // Switch to B: A's issue state must not leak — B starts at defaults.
    setActiveRepoPath("/tmp/b");
    expect(get(issueList)).toEqual([]);
    expect(get(issueStateFilter)).toBe("open");
    expect(get(selectedIssueNumber)).toBeNull();
    issueList.set([issueB]);
    selectedIssueNumber.set(42);

    // Switch back to A: its state is intact (pointer swap, no restore).
    setActiveRepoPath("/tmp/a");
    expect(get(issueList)).toEqual([issueA]);
    expect(get(issueStateFilter)).toBe("closed");
    expect(get(selectedIssueNumber)).toBe(1);
  });
});
