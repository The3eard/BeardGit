/**
 * Folder-tree construction and favorite hoisting.
 *
 * The hoisting rule is per level: a starred branch rises to the top of the
 * level it lives on, it does not escape its folder and does not drag the
 * folder up with it. That's the behaviour worth pinning — it's the part that
 * looks like a bug if it silently changes.
 */
import { describe, it, expect } from "vitest";
import { buildBranchTree } from "../branch-tree";
import type { BranchInfo } from "$lib/types";

function branch(name: string, extra: Partial<BranchInfo> = {}): BranchInfo {
  return {
    name,
    is_head: false,
    is_remote: false,
    oid: "abc1234",
    upstream: null,
    ahead: 0,
    behind: 0,
    upstream_gone: false,
    ...extra,
  };
}

const names = (nodes: { name: string }[]) => nodes.map((n) => n.name);

describe("buildBranchTree", () => {
  it("nests slash-separated names under folder nodes", () => {
    const tree = buildBranchTree([branch("main"), branch("feat/a"), branch("feat/b")]);

    expect(names(tree)).toEqual(["main", "feat"]);
    const feat = tree.find((n) => n.name === "feat")!;
    expect(feat.isFolder).toBe(true);
    expect(names(feat.children)).toEqual(["a", "b"]);
    expect(feat.children.map((c) => c.fullPath)).toEqual(["feat/a", "feat/b"]);
  });

  it("keeps the incoming order when nothing is starred", () => {
    const tree = buildBranchTree([branch("beta"), branch("main"), branch("zeta")]);

    expect(names(tree)).toEqual(["beta", "main", "zeta"]);
    expect(tree.every((n) => !n.isFavorite)).toBe(true);
  });

  it("hoists a starred branch to the top of its level", () => {
    const tree = buildBranchTree(
      [branch("alpha"), branch("beta"), branch("main")],
      new Set(["main"]),
    );

    expect(names(tree)).toEqual(["main", "alpha", "beta"]);
    expect(tree[0].isFavorite).toBe(true);
  });

  it("hoists inside a folder without moving the folder or leaving it", () => {
    const tree = buildBranchTree(
      [branch("main"), branch("feat/a"), branch("feat/starred"), branch("feat/b")],
      new Set(["feat/starred"]),
    );

    expect(names(tree)).toEqual(["main", "feat"]);
    const feat = tree.find((n) => n.name === "feat")!;
    expect(names(feat.children)).toEqual(["starred", "a", "b"]);
  });

  it("preserves relative order among several starred branches", () => {
    const tree = buildBranchTree(
      [branch("a"), branch("b"), branch("c"), branch("d")],
      new Set(["b", "d"]),
    );

    expect(names(tree)).toEqual(["b", "d", "a", "c"]);
  });

  it("marks remote branches by their full name", () => {
    const tree = buildBranchTree(
      [branch("origin/main", { is_remote: true }), branch("origin/dev", { is_remote: true })],
      new Set(["origin/main"]),
    );

    const origin = tree[0];
    expect(origin.isFolder).toBe(true);
    expect(origin.isFavorite).toBe(false);
    expect(names(origin.children)).toEqual(["main", "dev"]);
    expect(origin.children[0].isFavorite).toBe(true);
  });

  it("ignores a star naming a branch that no longer exists", () => {
    const tree = buildBranchTree([branch("main")], new Set(["deleted-branch"]));

    expect(names(tree)).toEqual(["main"]);
    expect(tree[0].isFavorite).toBe(false);
  });
});
