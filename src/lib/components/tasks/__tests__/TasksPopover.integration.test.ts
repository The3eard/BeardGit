/**
 * Integration test for the Tasks popover open regression (Phase 10).
 *
 * The sibling `TasksPopover.test.ts` renders the popover with a static
 * `open=true` prop, so it never exercises the *click-to-open* path.
 * Similarly, `TasksSlot.test.ts` asserts the click handler fires but
 * stops there. The regression lives in the seam between the two:
 *
 *   1. User clicks `<TasksSlot>` → `toggleTasksPopover()` flips the
 *      `tasksPopoverOpen` store from `false` → `true`.
 *   2. `<TasksPopover>` reacts to the new `open=true` and renders the
 *      `<div data-testid="tasks-popover" bind:this={popoverEl}>`.
 *   3. The same click event bubbles up to `<svelte:window onclick>` →
 *      `handleClickOutside` runs. If the render pass has committed by
 *      then, `popoverEl` is bound, `event.target` is the statusbar
 *      button (outside the popover), `onClose()` fires, and the popover
 *      closes before the user ever sees it.
 *
 * This test wires the real stores end-to-end — no store mocks — and
 * performs a real DOM click on the slot. After the click and a tick,
 * the popover must still be open.
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { get } from "svelte/store";
import { closeTasksPopover, tasksPopoverOpen } from "$lib/stores/tasksPopover";
import TasksPopoverHarness from "./TasksPopoverHarness.svelte";

beforeEach(() => {
  closeTasksPopover();
});

afterEach(() => {
  cleanup();
  closeTasksPopover();
});

describe("TasksPopover ↔ TasksSlot integration", () => {
  it("stays open after clicking the statusbar slot", async () => {
    const { getByTestId, queryByTestId } = render(TasksPopoverHarness);
    await tick();

    // Sanity: nothing rendered yet.
    expect(queryByTestId("tasks-popover")).toBeNull();
    expect(get(tasksPopoverOpen)).toBe(false);

    // Dispatch a real bubbling click — this mirrors what a browser does
    // when the user clicks the button: target fires `onclick`, then the
    // event bubbles to <svelte:window onclick>.
    const slot = getByTestId("statusbar-tasks-slot");
    slot.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    await tick();
    await tick();

    // The store flipped, so the popover should be mounted AND visible.
    expect(get(tasksPopoverOpen)).toBe(true);
    expect(queryByTestId("tasks-popover")).not.toBeNull();
  });

  it("closes on a subsequent click outside the popover", async () => {
    const { getByTestId, queryByTestId } = render(TasksPopoverHarness);
    await tick();

    // Open via click.
    getByTestId("statusbar-tasks-slot").dispatchEvent(
      new MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await tick();
    await tick();
    expect(queryByTestId("tasks-popover")).not.toBeNull();

    // Click on the bare <body> — outside the popover and outside the slot.
    document.body.dispatchEvent(
      new MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await tick();
    await tick();

    expect(get(tasksPopoverOpen)).toBe(false);
    expect(queryByTestId("tasks-popover")).toBeNull();
  });
});
