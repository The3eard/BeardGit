/**
 * Tasks drawer open/closed state.
 *
 * Owned by a top-level store (not component-local) so three independent
 * surfaces can drive it:
 *
 *   1. The statusbar **TasksSlot** click handler.
 *   2. The global **Cmd+J / Ctrl+J** shortcut registered in `+page.svelte`.
 *   3. Future programmatic openers (e.g. "show me that failure in the
 *      drawer" from a toast action).
 *
 * The TasksDrawer component binds to `tasksDrawerOpen` so any writer wins.
 *
 * Kept in its own module — not folded into `tasks.ts` — because the
 * aggregator store owns the *data* (feed, error flags) while this module
 * owns the *UI toggle*. Separating them means the aggregator can be
 * imported in non-UI contexts (tests, background polling) without
 * dragging a drawer-open flag that doesn't belong there.
 */

import { writable } from "svelte/store";

/**
 * Whether the tasks drawer is currently visible.
 *
 * Defaults to `false` on app start — the drawer is always opt-in.
 */
export const tasksDrawerOpen = writable<boolean>(false);

/**
 * Flip the drawer between open and closed.
 *
 * Used by both the statusbar TasksSlot click handler and the global
 * Cmd+J / Ctrl+J keyboard shortcut.
 */
export function toggleTasksDrawer(): void {
  tasksDrawerOpen.update((open) => !open);
}

/** Explicitly close the drawer (used by the drawer's `onClose` prop). */
export function closeTasksDrawer(): void {
  tasksDrawerOpen.set(false);
}

/** Explicitly open the drawer. */
export function openTasksDrawer(): void {
  tasksDrawerOpen.set(true);
}
