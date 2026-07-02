/**
 * Branches store — branch listing, selection with commit history,
 * and branch operations (checkout, delete, merge).
 *
 * Selection loads the last 30 commits on the branch with a last-wins
 * async guard. Per-commit detail and file-level diffs are loaded on demand.
 *
 * ── Migrated to the RepoState container (spec 08) ─────────────────────
 * The branch list + selection now live per-repo in `RepoState.branches`
 * (see `repo-state/BranchesSlice.ts`). The exports below are thin facades
 * over the *active* repo's slice, so the branch list survives tab switches
 * for free — this replaced the hand-rolled `branchCache` Map + the
 * `cacheBranchesForProject`/`restoreCachedBranches` choreography that used
 * to live here and be driven from `projects.ts`.
 */

import { derived, get } from "svelte/store";
import type { BranchInfo, CommitInfo, CommitFileChange, BatchDeleteResult } from "../types";
import type { RawDiffContent } from "./graph";
import { getCommitDetail, getCommitFiles, getFileAtCommitText as getFileAtCommit } from "../api/tauri";
import {
  getBranches as apiBranches,
  getBranchCommits,
  checkoutBranch as apiCheckout,
  deleteBranch as apiDelete,
  deleteBranches as apiDeleteBatch,
  mergeBranch as apiMerge,
} from "../api/tauri";
import { runMutation } from "../api/runMutation";
import { fetchListIntoStore } from "../utils/store-helpers";
import { activeField, getActiveRepoState } from "./repo-state";

// Facades over the active repo's BranchesSlice. Components keep importing
// these unchanged; the data is now per-repo.
export const branches = activeField<BranchInfo[]>((rs) => rs.branches.list);
export const branchesLoading = activeField<boolean>((rs) => rs.branches.loading);
export const selectedBranchName = activeField<string | null>((rs) => rs.branches.selectedName);
export const selectedBranchCommits = activeField<CommitInfo[]>((rs) => rs.branches.selectedCommits);
export const loadingDetail = activeField<boolean>((rs) => rs.branches.loadingDetail);
export const branchFileDiff = activeField<RawDiffContent | null>((rs) => rs.branches.fileDiff);
export const branchSelectedCommit = activeField<CommitInfo | null>((rs) => rs.branches.selectedCommit);
export const branchSelectedFiles = activeField<CommitFileChange[]>((rs) => rs.branches.selectedFiles);

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

/**
 * Delete a batch of local branches in one command (the cleanup dialog).
 * `force` is the subset of `names` to force-delete (`-d` for the rest).
 * Routes through `runMutation`, so a success/failure toast reports the
 * deleted/failed counts; the single `project-mutated` event drives the
 * branch-list refresh. Returns the full result so the dialog can surface
 * per-branch failures. Clears selection if the selected branch was removed.
 */
export async function doDeleteBranches(
  names: string[],
  force: string[],
): Promise<BatchDeleteResult> {
  const result = await runMutation({
    kind: "branch_delete_batch",
    invoke: () => apiDeleteBatch(names, force),
    successToast: (r) =>
      r.failed.length === 0
        ? `Deleted ${r.deleted.length} branch${r.deleted.length === 1 ? "" : "es"}`
        : `Deleted ${r.deleted.length}, ${r.failed.length} failed`,
    failureToastPrefix: "Branch cleanup failed",
  });
  const sel = get(selectedBranchName);
  if (sel && result.deleted.includes(sel)) {
    selectedBranchName.set(null);
    selectedBranchCommits.set([]);
  }
  // Branch list refresh is driven by the project-mutated event.
  return result;
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

/** Reset the active repo's branch selection/detail state. Called on repo switch. */
export function clearBranchState() {
  getActiveRepoState().branches.clear();
}
