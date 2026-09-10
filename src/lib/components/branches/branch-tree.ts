import type { BranchInfo } from "../../types";
import { parseRemoteBranch } from "./parse-remote-branch";

/** Shared tree node type for branch folder tree rendering. */
export interface BranchTreeNode {
  name: string;
  fullPath: string;
  isFolder: boolean;
  isHead: boolean;
  isRemote: boolean;
  oid: string;
  /** Commits ahead of upstream. `0` for folders and untracked branches. */
  ahead: number;
  /** Commits behind upstream. `0` for folders and untracked branches. */
  behind: number;
  /** `true` when the branch's configured upstream is gone (deleted remote). */
  upstreamGone: boolean;
  /** `true` when the user starred the branch. Always `false` for folders. */
  isFavorite: boolean;
  /** {@link favoriteKey} of this branch — what the star toggle writes.
   *  Empty for folders. */
  favoriteKey: string;
  children: BranchTreeNode[];
}

/**
 * The key a branch is starred under: the branch, not the ref pointing at it.
 *
 * `develop` and `origin/develop` are one branch as far as the user is
 * concerned, so starring either marks both. A remote branch therefore drops
 * its remote segment — the first one, which is how libgit2 names remote refs —
 * and a local branch is its own key. `origin/HEAD` is a symbolic ref rather
 * than a branch, and {@link parseRemoteBranch} rejects it, so it keys to
 * itself and can never be starred by proxy.
 */
export function favoriteKey(branch: BranchInfo): string {
  if (!branch.is_remote) return branch.name;
  return parseRemoteBranch(branch.name)?.branch ?? branch.name;
}

/**
 * Build a folder tree from a flat branch list. Branches with `/` in their
 * name nest under folder nodes.
 *
 * Starred branches are hoisted to the front of the level they sit on, with
 * the incoming order preserved otherwise. Hoisting is per level on purpose:
 * a starred `feat/x` rises to the top of the `feat` folder, it does not leave
 * the folder and the folder itself does not move.
 */
export function buildBranchTree(
  branchList: BranchInfo[],
  favorites: ReadonlySet<string> = new Set(),
): BranchTreeNode[] {
  const root: BranchTreeNode[] = [];
  const childMaps = new WeakMap<BranchTreeNode[], Map<string, BranchTreeNode>>();

  function getMap(children: BranchTreeNode[]): Map<string, BranchTreeNode> {
    let map = childMaps.get(children);
    if (!map) {
      map = new Map();
      childMaps.set(children, map);
    }
    return map;
  }

  for (const branch of branchList) {
    const parts = branch.name.split("/");
    // Not `key`: the loop below uses that name for the child-map key.
    const favKey = favoriteKey(branch);
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
          ahead: isLeaf ? branch.ahead : 0,
          behind: isLeaf ? branch.behind : 0,
          upstreamGone: isLeaf ? branch.upstream_gone : false,
          isFavorite: isLeaf && favorites.has(favKey),
          favoriteKey: isLeaf ? favKey : "",
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

  return favorites.size === 0 ? root : hoistFavorites(root);
}

/** Move starred nodes to the front of every level, recursively. */
function hoistFavorites(nodes: BranchTreeNode[]): BranchTreeNode[] {
  for (const node of nodes) {
    if (node.children.length > 0) {
      node.children = hoistFavorites(node.children);
    }
  }
  const starred = nodes.filter((n) => n.isFavorite);
  return starred.length === 0 ? nodes : [...starred, ...nodes.filter((n) => !n.isFavorite)];
}
