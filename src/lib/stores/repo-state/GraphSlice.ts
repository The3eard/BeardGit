/**
 * Per-repo commit-graph state (RepoState slice).
 *
 * Holds one repo's rendered viewport window, scroll offset, commit selection,
 * ref-badge diff, and the file-diff panel. Because every open repo owns its
 * own `GraphSlice`, the graph position + selection survive tab switches as a
 * pointer swap (`setActiveRepoPath`) instead of the old `viewportCache` Map +
 * `clearGraphState` choreography that used to live in `stores/graph.ts` and be
 * driven from `projects.ts` (spec 08).
 *
 * View options (`graphViewOptions`) and the current user's identities
 * (`userEmails`) are NOT here: the first-parent toggle is a session preference
 * and the lane budget is derived from the window width, so both are
 * window-scoped and stay module-level in `graph.ts`.
 *
 * Fields are plain Svelte `writable`s (not `$state` runes) — see the note in
 * `./index.ts` for why the fallback was chosen for this migration step.
 */

import { writable } from "svelte/store";
import type { GraphViewport, CommitInfo, CommitFileChange } from "../../types";
import type { RawDiffContent } from "../graph";

export class GraphSlice {
  /** The rendered viewport window (nodes + lane geometry). */
  readonly viewport = writable<GraphViewport | null>(null);
  /** Scroll offset of the top visible row. */
  readonly graphOffset = writable(0);
  /** OID of the selected commit (drives CommitDetail). */
  readonly selectedOid = writable<string | null>(null);
  /** Group ID of the selected lane segment (dims other branches). */
  readonly selectedGroup = writable<number | null>(null);
  readonly selectedCommit = writable<CommitInfo | null>(null);
  readonly selectedCommitFiles = writable<CommitFileChange[]>([]);
  /** Ref name of the currently expanded merge-badge diff. */
  readonly selectedRef = writable<string | null>(null);
  /** Files changed between the merge commit's parents (for ref badge click). */
  readonly refFiles = writable<CommitFileChange[] | null>(null);
  readonly loadingRefFiles = writable(false);
  readonly fileDiffPanel = writable<RawDiffContent | null>(null);
  readonly loadingFileDiff = writable(false);

  /** Reset selection/detail state, keeping the viewport. Mirrors the old
   *  `clearGraphState` — the viewport is either restored from a sibling
   *  slice on tab switch or replaced when `loadViewport()` returns fresh. */
  clear(): void {
    this.selectedOid.set(null);
    this.selectedCommit.set(null);
    this.selectedCommitFiles.set([]);
    this.selectedRef.set(null);
    this.refFiles.set(null);
    this.loadingRefFiles.set(false);
    this.fileDiffPanel.set(null);
    this.loadingFileDiff.set(false);
    this.selectedGroup.set(null);
  }

  /** Full reset including the viewport. Mirrors the old `resetGraphState`. */
  reset(): void {
    this.clear();
    this.viewport.set(null);
    this.graphOffset.set(0);
  }
}
