/**
 * Per-repo branch state (RepoState slice).
 *
 * Holds one repo's branch list, selection, and per-commit detail. Because
 * every open repo owns its own `BranchesSlice`, the list survives tab
 * switches for free — this replaces the hand-rolled `branchCache` Map that
 * used to live in `stores/branches.ts` (spec 08).
 *
 * Fields are plain Svelte `writable`s (not `$state` runes) — see the note
 * in `./index.ts` for why the fallback was chosen for this migration step.
 */

import { writable } from "svelte/store";
import type { BranchInfo, CommitInfo, CommitFileChange } from "../../types";
import type { RawDiffContent } from "../graph";

export class BranchesSlice {
  /** The repo's branch list (local + remote), as served by `get_branches`. */
  readonly list = writable<BranchInfo[]>([]);
  readonly loading = writable(false);
  readonly selectedName = writable<string | null>(null);
  readonly selectedCommits = writable<CommitInfo[]>([]);
  readonly loadingDetail = writable(false);
  readonly fileDiff = writable<RawDiffContent | null>(null);
  readonly selectedCommit = writable<CommitInfo | null>(null);
  readonly selectedFiles = writable<CommitFileChange[]>([]);

  /** Reset selection/detail state. Mirrors the old `clearBranchState`. */
  clear(): void {
    this.selectedName.set(null);
    this.selectedCommits.set([]);
    this.loadingDetail.set(false);
    this.fileDiff.set(null);
    this.selectedCommit.set(null);
    this.selectedFiles.set([]);
  }
}
