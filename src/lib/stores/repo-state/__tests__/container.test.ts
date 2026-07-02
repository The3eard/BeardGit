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
