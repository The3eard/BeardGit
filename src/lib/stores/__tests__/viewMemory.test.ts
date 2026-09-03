import { afterEach, describe, expect, it } from "vitest";
import { get } from "svelte/store";
import { activeRepoPath } from "../repo-state";
import { __resetViewMemory, forgetScope, remembered, scoped } from "../viewMemory";

afterEach(() => {
  __resetViewMemory();
  activeRepoPath.set(null);
});

describe("viewMemory", () => {
  it("returns the same store for the same key, keeping the value", () => {
    const first = remembered("branches.filter", "");
    first.set("feat/");
    // A remount asks again with the initial value — the remembered one wins.
    const second = remembered("branches.filter", "");
    expect(second).toBe(first);
    expect(get(second)).toBe("feat/");
  });

  it("scopes keys to the active repository", () => {
    activeRepoPath.set("/repo/a");
    expect(scoped("k")).toBe("/repo/a::k");
    activeRepoPath.set(null);
    expect(scoped("k")).toBe("k");
  });

  it("forgetScope drops one repository's cells and nothing else", () => {
    activeRepoPath.set("/repo/a");
    remembered(scoped("k"), 1).set(2);
    activeRepoPath.set("/repo/b");
    remembered(scoped("k"), 1).set(3);
    remembered("global", 1).set(4);

    forgetScope("/repo/a");

    activeRepoPath.set("/repo/a");
    expect(get(remembered(scoped("k"), 1))).toBe(1);
    activeRepoPath.set("/repo/b");
    expect(get(remembered(scoped("k"), 1))).toBe(3);
    expect(get(remembered("global", 1))).toBe(4);
  });
});
