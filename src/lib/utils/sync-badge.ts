/**
 * Pure helpers for the toolbar Push/Pull sync badges.
 *
 * The badge shows the ahead (Push) / behind (Pull) commit count of the
 * current branch vs its upstream. Data source is the same
 * `activeRepoStatus` store the status bar reads — no new command, no
 * polling; it refreshes through the mutation-events pipeline.
 */

/** Counts above this render as "99+" so the badge never grows wide. */
const MAX_BADGE_COUNT = 99;

/**
 * A badge is shown only for a finite, positive integer count. Zero (in
 * sync), a missing upstream, or a detached HEAD all yield 0 → no badge.
 */
export function shouldShowSyncBadge(count: number | null | undefined): boolean {
  return typeof count === "number" && Number.isFinite(count) && count > 0;
}

/**
 * Format a positive count for the tiny pill: the number itself, capped at
 * "99+" so absurd divergences don't blow out the badge width. Assumes the
 * caller has already gated on {@link shouldShowSyncBadge}.
 */
export function formatSyncBadge(count: number): string {
  return count > MAX_BADGE_COUNT ? `${MAX_BADGE_COUNT}+` : String(count);
}
