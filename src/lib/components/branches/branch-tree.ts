/** Shared tree node type for branch folder tree rendering. */
export interface BranchTreeNode {
  name: string;
  fullPath: string;
  isFolder: boolean;
  isHead: boolean;
  isRemote: boolean;
  oid: string;
  children: BranchTreeNode[];
}
