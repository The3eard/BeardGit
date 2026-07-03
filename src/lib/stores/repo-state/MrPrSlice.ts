/**
 * Per-repo MR/PR state (RepoState slice).
 *
 * Holds one repo's merge-request / pull-request view: the filter tab, the
 * list + its loading/error state, the selected item's detail + diff, the
 * per-file diff panel, and the repository label cache. These used to be
 * module-level singletons in `stores/mr-pr.ts`, shared by every open repo
 * tab — so a PR list, selection, or open file diff from repo A survived a
 * switch to repo B and could act against B's forge. Giving each open repo
 * its own `MrPrSlice` makes the view follow the active tab (spec 08 slice
 * pattern, like `CompareSlice`).
 *
 * The `filter` tab is per-repo (each repo's PR list keeps its own
 * open/closed/merged selection) and the `ensuredShas` cache is per-repo
 * (a sha present in repo A's object database says nothing about repo B).
 *
 * Fields are plain Svelte `writable`s (not `$state` runes) — see the note in
 * `./index.ts` for why the fallback was chosen for this migration step.
 */

import { writable } from "svelte/store";
import type { Label, MrPr, MrPrDetail, MrPrDiffFile, MrPrState } from "../../types";
import type { RawDiffContent } from "../graph";

/**
 * Currently-viewed PR per-file diff payload. Null when no file is selected.
 * Shares the `RawDiffContent` shape with the graph/branch/reflog diff panels
 * so `DiffEditor.svelte` can render it unchanged, plus an optional
 * `binary: boolean` flag for the placeholder branch.
 */
export interface PrRawDiffContent extends RawDiffContent {
  /** True when either side's blob was flagged binary. */
  binary: boolean;
}

export class MrPrSlice {
  /** Current filter tab: open, closed, merged, or all. */
  readonly filter = writable<MrPrState | "all">("open");
  /** List of MR/PRs matching the current filter. */
  readonly list = writable<MrPr[]>([]);
  /** Whether the list is loading. */
  readonly listLoading = writable(false);
  /** Last error raised while fetching the MR/PR list. Null on success. */
  readonly listError = writable<string | null>(null);

  /** Currently selected MR/PR number. */
  readonly selectedNumber = writable<number | null>(null);
  /** Detail of the selected MR/PR. */
  readonly detail = writable<MrPrDetail | null>(null);
  /** Changed files for the selected MR/PR. */
  readonly diffFiles = writable<MrPrDiffFile[]>([]);
  /** Whether the detail (summary + body + comments) is loading. */
  readonly detailLoading = writable(false);
  /** Last error raised while loading the selected MR/PR detail. Null on success. */
  readonly detailError = writable<string | null>(null);
  /** Whether the diff-files fetch is in flight for the selected MR/PR. */
  readonly diffLoading = writable(false);
  /** Last error raised while loading the selected MR/PR's diff files. */
  readonly diffError = writable<string | null>(null);

  /** Cache of repository labels, populated on demand by the label picker. */
  readonly repoLabels = writable<Label[]>([]);
  /** Whether the repository label cache is currently loading. */
  readonly repoLabelsLoading = writable(false);

  /** Diff content for the currently-selected PR file, or `null` if none. */
  readonly prFileDiff = writable<PrRawDiffContent | null>(null);
  /** True while a PR per-file diff is in flight. */
  readonly loadingPrFileDiff = writable(false);
  /** Last error raised during a PR per-file diff load, or `null`. */
  readonly prFileDiffError = writable<string | null>(null);
  /** Currently selected file path in the PR file list. */
  readonly selectedPrFilePath = writable<string | null>(null);

  /**
   * Commits already materialised locally (or with an ensure in flight),
   * keyed by sha — per-repo, since a sha present in one repo's odb says
   * nothing about another's. Without it, every file click in a PR re-ran the
   * `ensure_commit_local` preflight. Failed ensures are evicted so an
   * explicit retry can attempt one new fetch, but nothing retries
   * automatically.
   */
  readonly ensuredShas = new Map<string, Promise<void>>();

  /**
   * Per-repo last-wins guard for the PR per-file diff. Bumped at the start of
   * a load; a response whose captured id no longer matches is dropped, so a
   * newer request in THIS repo cancels an older one. Purely internal
   * sequencing, so a plain number rather than a writable.
   */
  prFileDiffRequestId = 0;

  /** Clear detail state (e.g., when navigating away). */
  clearDetail(): void {
    this.selectedNumber.set(null);
    this.detail.set(null);
    this.diffFiles.set([]);
  }

  /** Clear all MR/PR state (e.g., on project switch). */
  clear(): void {
    this.list.set([]);
    this.filter.set("open");
    this.clearDetail();
    // Ensured shas are per-repo facts — a sha present in the previous
    // project's odb says nothing about the new one.
    this.ensuredShas.clear();
  }
}
