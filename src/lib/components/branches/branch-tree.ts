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
  children: BranchTreeNode[];
}
