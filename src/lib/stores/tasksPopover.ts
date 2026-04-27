/**
 * Tasks popover open/closed state.
 *
 * Restores the pre-cluster-0.3 "click the statusbar icon → popover with
 * the list of recent tasks → drill into a detailed console view" UX
 * after it was replaced by a bottom drawer. The popover itself is
 * anchored to the `TasksSlot` button in the statusbar and mounted in
 * `+page.svelte` so three independent surfaces can drive it:
 *
 *   1. The statusbar **TasksSlot** click handler.
 *   2. The global **Cmd+J / Ctrl+J** shortcut registered in `+page.svelte`.
 *   3. Future programmatic openers (e.g. "show me that failure" from a
 *      toast action).
 *
 * Kept separate from the data-bearing `tasks.ts` aggregator because the
 * aggregator wants to be importable from non-UI contexts (tests,
 * background polling) without dragging a UI-toggle flag along.
 *
 * The popover drills two levels deep — top-level "Tasks" list, then a
 * per-task detail view with streamed stdout/stderr. The selected task
 * and drill-down state are owned by the legacy `taskPanel.ts` store
 * (`selectedTaskId`, `selectedOutput`, `panelMode`) so the output
 * accumulator keeps running independently of this open/closed flag.
 */

import { writable } from "svelte/store";

/**
 * Whether the tasks popover is currently visible.
 *
 * Defaults to `false` on app start — the popover is always opt-in.
 */
export const tasksPopoverOpen = writable<boolean>(false);

/**
 * Id of the task whose detail view the popover should pre-select on
 * its next open transition, or `null` when the popover should open in
 * list mode.
 *
 * Wired by the runMutation failure escalation path (the "See details"
 * toast action): a failing mutation registers an ad-hoc task record,
 * then calls {@link openTasksPopover} with that id so the popover
 * opens straight on the error output instead of the generic list.
 * `TasksPopover` consumes this store in its `open` → `true` effect,
 * then clears it so the pre-select doesn't leak into the next open.
 */
export const tasksPopoverPendingDetail = writable<string | null>(null);

/**
 * Flip the popover between open and closed.
 *
 * Used by both the statusbar TasksSlot click handler and the global
 * Cmd+J / Ctrl+J keyboard shortcut.
 */
export function toggleTasksPopover(): void {
  tasksPopoverOpen.update((open) => !open);
}

/** Explicitly close the popover (used by outside-click / Esc / X). */
export function closeTasksPopover(): void {
  tasksPopoverOpen.set(false);
}

/**
 * Explicitly open the popover.
 *
 * Accepts an optional task id that, when provided, instructs
 * `TasksPopover` to pre-select that row's detail view instead of the
 * default list. Used by the toast "See details" failure-escalation
 * action so users land directly on the failing task's output.
 */
export function openTasksPopover(pendingDetailId?: string): void {
  if (pendingDetailId !== undefined) {
    tasksPopoverPendingDetail.set(pendingDetailId);
  }
  tasksPopoverOpen.set(true);
}
