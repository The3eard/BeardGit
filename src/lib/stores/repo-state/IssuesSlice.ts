/**
 * Per-repo Issues state (RepoState slice).
 *
 * Holds one repo's issue view: the state filter tab, the list + its loading
 * state, the selected issue's detail, and the repository label + milestone
 * caches. These used to be module-level singletons in `stores/issues.ts`,
 * shared by every open repo tab — so an issue list, selection, or filter
 * from repo A survived a switch to repo B. Giving each open repo its own
 * `IssuesSlice` makes the view follow the active tab (spec 08 slice pattern,
 * like `MrPrSlice`).
 *
 * The `stateFilter` tab is per-repo (each repo's issue list keeps its own
 * open/closed selection), as are the label and milestone caches (they mirror
 * one repo's forge).
 *
 * Fields are plain Svelte `writable`s (not `$state` runes) — see the note in
 * `./index.ts` for why the fallback was chosen for this migration step.
 */

import { writable } from "svelte/store";
import type { Issue, IssueDetail, IssueState, Label, Milestone } from "../../types";

export class IssuesSlice {
  /** Current state filter tab: open, closed, or all. */
  readonly stateFilter = writable<IssueState | "all">("open");
  /** Current list of issues matching the filter. */
  readonly list = writable<Issue[]>([]);
  /** Whether the list is currently loading. */
  readonly listLoading = writable(false);

  /** Currently selected issue number. */
  readonly selectedNumber = writable<number | null>(null);
  /** Full detail of the selected issue. */
  readonly detail = writable<IssueDetail | null>(null);
  /** Whether the detail view is loading. */
  readonly detailLoading = writable(false);

  /** Cache of repository labels (for pickers). Lazily loaded. */
  readonly labelsCache = writable<Label[]>([]);
  /** Whether the labels cache is currently loading. */
  readonly labelsCacheLoading = writable(false);

  /** Cache of repository milestones (for pickers). Lazily loaded. */
  readonly milestonesCache = writable<Milestone[]>([]);
  /** Whether the milestones cache is currently loading. */
  readonly milestonesCacheLoading = writable(false);

  /** Clear detail-only state without touching the list. */
  clearDetail(): void {
    this.selectedNumber.set(null);
    this.detail.set(null);
  }

  /** Reset all issue state (on project switch). */
  clear(): void {
    this.list.set([]);
    this.stateFilter.set("open");
    this.clearDetail();
  }
}
