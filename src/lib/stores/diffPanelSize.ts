/**
 * Diff-panel sizing store.
 *
 * The bottom diff panel (commit/branch/reflog/PR-MR file diffs) is
 * resizable on both axes via `ResizableDiffPanel`. The size lives here —
 * module-level rather than component-local — so it persists when the user
 * switches between views (each view mounts a fresh panel instance, but
 * they should all open at the size the user last set). Kept in memory
 * only, matching the previous behaviour where the height reset on restart.
 */

import { writable } from "svelte/store";

/** Default panel height in px, used on first paint and by the reset. */
export const DIFF_PANEL_DEFAULT_HEIGHT = 250;

/** Current panel height in px. */
export const diffPanelHeight = writable<number>(DIFF_PANEL_DEFAULT_HEIGHT);

/**
 * Current panel width in px, or `null` for the default full-width layout.
 * A number means the user has narrowed the panel; the leftover space sits
 * to its right.
 */
export const diffPanelWidth = writable<number | null>(null);

/** Restore the default height (bound to the height handle's double-click). */
export function resetDiffPanelHeight(): void {
  diffPanelHeight.set(DIFF_PANEL_DEFAULT_HEIGHT);
}

/** Restore full width (bound to the width handle's double-click). */
export function resetDiffPanelWidth(): void {
  diffPanelWidth.set(null);
}
