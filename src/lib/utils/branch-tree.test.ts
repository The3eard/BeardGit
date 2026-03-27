import { describe, it, expect } from "vitest";

interface TreeNode {
  name: string;
  fullPath: string;
  isFolder: boolean;
  isHead: boolean;
  isRemote: boolean;
  oid: string;
  children: TreeNode[];
}

interface BranchInfo {
  name: string;
  is_head: boolean;
  is_remote: boolean;
  oid: string;
}

// Mirror of the Map-based buildTree from BranchList.svelte
function buildTree(branchList: BranchInfo[]): TreeNode[] {
  const root: TreeNode[] = [];
  const childMaps = new WeakMap<TreeNode[], Map<string, TreeNode>>();

  function getMap(children: TreeNode[]): Map<string, TreeNode> {
    let map = childMaps.get(children);
    if (!map) {
      map = new Map();
      childMaps.set(children, map);
    }
    return map;
  }

  for (const branch of branchList) {
    const parts = branch.name.split("/");
    let current = root;

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLeaf = i === parts.length - 1;
      const key = `${part}:${isLeaf ? "leaf" : "folder"}`;
      const map = getMap(current);

      let existing = map.get(key);
      if (!existing) {
        existing = {
          name: part,
          fullPath: isLeaf ? branch.name : parts.slice(0, i + 1).join("/"),
          isFolder: !isLeaf,
          isHead: isLeaf && branch.is_head,
          isRemote: branch.is_remote,
          oid: isLeaf ? branch.oid : "",
          children: [],
        };
        current.push(existing);
        map.set(key, existing);
      }
      if (!isLeaf) {
        current = existing.children;
      }
    }
  }
  return root;
}

describe("buildTree", () => {
  it("handles simple branch names", () => {
    const branches: BranchInfo[] = [
      { name: "main", is_head: true, is_remote: false, oid: "abc123" },
      { name: "develop", is_head: false, is_remote: false, oid: "def456" },
    ];
    const tree = buildTree(branches);
    expect(tree).toHaveLength(2);
    expect(tree[0].name).toBe("main");
    expect(tree[0].isFolder).toBe(false);
    expect(tree[0].isHead).toBe(true);
    expect(tree[1].name).toBe("develop");
  });

  it("creates folder structure for slashed names", () => {
    const branches: BranchInfo[] = [
      { name: "feature/auth", is_head: false, is_remote: false, oid: "aaa" },
      { name: "feature/graph", is_head: false, is_remote: false, oid: "bbb" },
    ];
    const tree = buildTree(branches);
    expect(tree).toHaveLength(1);
    expect(tree[0].name).toBe("feature");
    expect(tree[0].isFolder).toBe(true);
    expect(tree[0].children).toHaveLength(2);
    expect(tree[0].children[0].name).toBe("auth");
    expect(tree[0].children[1].name).toBe("graph");
  });

  it("handles deeply nested paths", () => {
    const branches: BranchInfo[] = [
      { name: "feature/ui/sidebar/resize", is_head: false, is_remote: false, oid: "ccc" },
    ];
    const tree = buildTree(branches);
    expect(tree[0].name).toBe("feature");
    expect(tree[0].children[0].name).toBe("ui");
    expect(tree[0].children[0].children[0].name).toBe("sidebar");
    expect(tree[0].children[0].children[0].children[0].name).toBe("resize");
    expect(tree[0].children[0].children[0].children[0].isFolder).toBe(false);
  });

  it("returns empty array for empty input", () => {
    expect(buildTree([])).toEqual([]);
  });

  it("mixes simple and nested branches", () => {
    const branches: BranchInfo[] = [
      { name: "main", is_head: true, is_remote: false, oid: "aaa" },
      { name: "feature/auth", is_head: false, is_remote: false, oid: "bbb" },
    ];
    const tree = buildTree(branches);
    expect(tree).toHaveLength(2);
    expect(tree[0].name).toBe("main");
    expect(tree[0].isFolder).toBe(false);
    expect(tree[1].name).toBe("feature");
    expect(tree[1].isFolder).toBe(true);
  });

  it("preserves remote flag", () => {
    const branches: BranchInfo[] = [
      { name: "origin/main", is_head: false, is_remote: true, oid: "aaa" },
    ];
    const tree = buildTree(branches);
    expect(tree[0].name).toBe("origin");
    expect(tree[0].isRemote).toBe(true);
    expect(tree[0].children[0].name).toBe("main");
    expect(tree[0].children[0].isRemote).toBe(true);
  });
});
