/**
 * Per-repo staging-area state (RepoState slice).
 *
 * Holds one repo's file statuses, per-file diff stats, the open staging
 * diff, the commit-message draft, and the checkbox selection for each
 * list. Because every open repo owns its own `ChangesSlice`, the checkbox
 * selection now survives tab switches per-repo — previously it lived in a
 * module-global store that `clearChangesState` wiped on every switch, so
 * switching repos and back dropped the selection (spec 08 isolation bug).
 *
 * Fields are plain Svelte `writable`s (not `$state` runes) — see the note
 * in `./index.ts` for why the fallback was chosen for this migration step.
 */

import { writable } from "svelte/store";
import type { FileStatus, FileDiff, FileDiffStat } from "../../types";

export class ChangesSlice {
  /** Per-file status list (staged and unstaged combined). */
  readonly fileStatuses = writable<FileStatus[]>([]);
  /** Unstaged (workdir-vs-index) per-file stats — no hunks. */
  readonly unstagedStats = writable<FileDiffStat[]>([]);
  /** Staged (index-vs-HEAD) per-file stats. */
  readonly stagedStats = writable<FileDiffStat[]>([]);
  /** The file whose full diff is open in the staging pane, or `null`. */
  readonly openStagingFile = writable<{ path: string; isStaged: boolean } | null>(null);
  /** Full hunks/lines diff for {@link openStagingFile}, fetched on demand. */
  readonly openStagingDiff = writable<FileDiff | null>(null);
  /** Current commit-message draft. Cleared after a successful commit. */
  readonly commitMessage = writable("");
  /** Checkbox selection for the unstaged / staged file lists. */
  readonly unstagedSelection = writable<Set<string>>(new Set());
  readonly stagedSelection = writable<Set<string>>(new Set());

  /** Reset both checkbox selections. Mirrors the old `clearChangesSelection`. */
  clearSelection(): void {
    this.unstagedSelection.set(new Set());
    this.stagedSelection.set(new Set());
  }

  /**
   * Reset the staging-area view state (statuses, stats, open diff, and
   * checkbox selection). Mirrors the old `clearChangesState`; the commit
   * message draft is intentionally preserved, as before.
   */
  clear(): void {
    this.fileStatuses.set([]);
    this.unstagedStats.set([]);
    this.stagedStats.set([]);
    this.openStagingFile.set(null);
    this.openStagingDiff.set(null);
    this.clearSelection();
  }
}
