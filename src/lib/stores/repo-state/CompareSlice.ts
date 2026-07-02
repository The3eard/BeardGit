/**
 * Per-repo compare-view state (RepoState slice).
 *
 * Holds one repo's "compare any ref against any ref" state: the two chosen
 * refs, the dot-mode, the resolved merge-base, the ahead/behind commit list
 * and counts, the changed-file list, and the open per-file diff. Because every
 * open repo owns its own `CompareSlice`, a comparison survives tab switches
 * per-repo (spec 10, following the spec-08 slice pattern).
 *
 * Fields are plain Svelte `writable`s (not `$state` runes) — see the note in
 * `./index.ts` for why the fallback was chosen for this migration step.
 */

import { writable } from "svelte/store";
import type { CommitInfo, CommitFileChange } from "../../types";
import type { RawDiffContent } from "../graph";

/**
 * Range semantics for the compare:
 * - `three-dot` (default): `merge-base(A,B)..B` — "what B adds" (PR semantics).
 * - `two-dot`: direct tree diff of A vs B (`git diff A..B`).
 */
export type CompareMode = "three-dot" | "two-dot";

export class CompareSlice {
  /** Base ref (side A) — branch name, tag, `HEAD`, or SHA. */
  readonly refA = writable<string | null>(null);
  /** Compare ref (side B). */
  readonly refB = writable<string | null>(null);
  /** Active range semantics. */
  readonly mode = writable<CompareMode>("three-dot");
  /** Resolved merge-base OID of A and B, or `null` if unrelated/unloaded. */
  readonly mergeBase = writable<string | null>(null);

  /** Commits B adds over A (`A..B`), newest-first — the main "ahead" list.
   *  The ahead *count* is `commits.length` (with a `+` when {@link commitsCapped}). */
  readonly commits = writable<CommitInfo[]>([]);
  /** Commits A is ahead of B ("behind") — used for the summary count only. */
  readonly behindCount = writable<number>(0);
  /** `true` when the ahead list hit the page limit (a "Load more" is available). */
  readonly commitsCapped = writable<boolean>(false);
  /** `true` while a "Load more" page fetch is in flight. */
  readonly loadingMore = writable<boolean>(false);

  /** Changed files for the current mode (three-dot merge-base..B or two-dot A..B). */
  readonly files = writable<CommitFileChange[]>([]);

  /** `true` while the top-level compare (commits + files) is loading. */
  readonly loading = writable<boolean>(false);
  /** Last error from a compare load, or `null`. */
  readonly error = writable<string | null>(null);

  /** Path of the file whose diff is open in the panel, or `null`. */
  readonly selectedFilePath = writable<string | null>(null);
  /** Old/new content for the open file diff (drives the DiffEditor). */
  readonly openDiff = writable<RawDiffContent | null>(null);
  /** `true` while the per-file diff is loading. */
  readonly loadingDiff = writable<boolean>(false);
  /** Last error from a per-file diff fetch, or `null`. */
  readonly diffError = writable<string | null>(null);

  /** Reset the open per-file diff (keeps the compare selection). */
  clearDiff(): void {
    this.selectedFilePath.set(null);
    this.openDiff.set(null);
    this.loadingDiff.set(false);
    this.diffError.set(null);
  }

  /** Reset all compare state. Called on repo switch and when clearing. */
  clear(): void {
    this.refA.set(null);
    this.refB.set(null);
    this.mode.set("three-dot");
    this.mergeBase.set(null);
    this.commits.set([]);
    this.behindCount.set(0);
    this.commitsCapped.set(false);
    this.loadingMore.set(false);
    this.files.set([]);
    this.loading.set(false);
    this.error.set(null);
    this.clearDiff();
  }
}
