/**
 * Branches store — branch listing, selection with commit history,
 * and branch operations (checkout, delete, merge).
 *
 * Selection loads the last 30 commits on the branch with a last-wins
 * async guard. Per-commit detail and file-level diffs are loaded on demand.
 */

import { writable, derived, get } from "svelte/store";
import type { BranchInfo, CommitInfo, CommitFileChange } from "../types";
import type { RawDiffContent } from "./graph";
import { getCommitDetail, getCommitFiles, getFileAtCommitText as getFileAtCommit } from "../api/tauri";
import {
  getBranches as apiBranches,
  getBranchCommits,
  checkoutBranch as apiCheckout,
  deleteBranch as apiDelete,
  mergeBranch as apiMerge,
} from "../api/tauri";
import { runMutation } from "../api/runMutation";
import { fetchListIntoStore } from "../utils/store-helpers";

export const branches = writable<BranchInfo[]>([]);
export const branchesLoading = writable(false);

/**
 * Per-project branch list cache for instant rendering on tab switch.
 *
 * Mirrors `graph.ts`'s `viewportCache` pattern. Branch lists are
 * cheap to fetch (~100 ms for repos with thousands of refs) but
 * the round-trip is still visible after `apiSwitchProject`. Caching
 * the last seen list lets us paint immediately and reconcile when
 * the fresh fetch arrives. Keyed by repo path.
 *
 * The cache is purely RAM — no disk persistence. On cold start the
 * usual loading spinner shows until `refreshBranches` resolves.
 */
const branchCache = new Map<string, BranchInfo[]>();

/** Save the current branch list under `projectPath` for a future tab switch. */
export function cacheBranchesForProject(projectPath: string): void {
  const list = get(branches);
  if (list.length > 0) branchCache.set(projectPath, list);
}

/** Restore a cached branch list. Returns `true` if the cache had an entry. */
export function restoreCachedBranches(projectPath: string): boolean {
  const cached = branchCache.get(projectPath);
  if (cached) {
    branches.set(cached);
    return true;
  }
  branches.set([]);
  return false;
}
export const selectedBranchName = writable<string | null>(null);
export const selectedBranchCommits = writable<CommitInfo[]>([]);
export const loadingDetail = writable(false);
export const branchFileDiff = writable<RawDiffContent | null>(null);
export const branchSelectedCommit = writable<CommitInfo | null>(null);
export const branchSelectedFiles = writable<CommitFileChange[]>([]);

export async function selectBranchCommit(oid: string) {
  branchSelectedCommit.set(null);
  branchSelectedFiles.set([]);
  branchFileDiff.set(null);
  const [commit, files] = await Promise.all([
    getCommitDetail(oid),
    getCommitFiles(oid).catch(() => [] as CommitFileChange[]),
  ]);
  branchSelectedCommit.set(commit);
  branchSelectedFiles.set(files);
}

export function closeBranchCommitDetail() {
  branchSelectedCommit.set(null);
  branchSelectedFiles.set([]);
  branchFileDiff.set(null);
}

export const localBranches = derived(branches, ($b) => $b.filter((b) => !b.is_remote));
export const remoteBranches = derived(branches, ($b) => $b.filter((b) => b.is_remote));
export const selectedBranchInfo = derived(
  [branches, selectedBranchName],
  ([$b, $name]) => ($name ? $b.find((b) => b.name === $name) ?? null : null),
);

export async function refreshBranches() {
  await fetchListIntoStore(
    branches,
    branchesLoading,
    selectedBranchName,
    apiBranches,
    [],
    (b) => b.name,
  );
  // If selection was cleared, also clear dependent state
  if (get(selectedBranchName) === null) {
    selectedBranchCommits.set([]);
    loadingDetail.set(false);
  }
}

export function selectBranch(name: string) {
  selectedBranchName.set(name);
  loadingDetail.set(true);
  selectedBranchCommits.set([]);
  const expectedBranch = name;
  getBranchCommits(name, 30)
    .then((commits) => {
      if (get(selectedBranchName) === expectedBranch) {
        selectedBranchCommits.set(commits);
        loadingDetail.set(false);
      }
    })
    .catch(() => {
      if (get(selectedBranchName) === expectedBranch) {
        loadingDetail.set(false);
      }
    });
}

export async function doCheckout(name: string) {
  await runMutation({
    kind: "checkout",
    invoke: () => apiCheckout(name),
    successToast: () => `Checked out ${name}`,
    failureToastPrefix: "Checkout failed",
  });
  // Branch list refresh is driven by the project-mutated event.
}

export async function doDeleteBranch(name: string, force = false) {
  await runMutation({
    kind: "branch_delete",
    invoke: () => apiDelete(name, force),
    successToast: () => `Deleted branch ${name}`,
    failureToastPrefix: "Branch delete failed",
  });
  if (get(selectedBranchName) === name) {
    selectedBranchName.set(null);
    selectedBranchCommits.set([]);
  }
  // Branch list refresh is driven by the project-mutated event.
}

export async function doMergeBranch(name: string) {
  await runMutation({
    kind: "merge",
    invoke: () => apiMerge(name),
    successToast: () => `Merged ${name}`,
    failureToastPrefix: "Merge failed",
  });
  // Branch list refresh is driven by the project-mutated event.
}

/** Reset all branch selection/detail state. Called on repo switch. */
export function clearBranchState() {
  selectedBranchName.set(null);
  selectedBranchCommits.set([]);
  loadingDetail.set(false);
  branchFileDiff.set(null);
  branchSelectedCommit.set(null);
  branchSelectedFiles.set([]);
}
