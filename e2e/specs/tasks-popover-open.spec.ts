import { expect } from "@wdio/globals";
import { openFixtureProject } from "../helpers/project";

/**
 * Phase 10 smoke test for the Tasks popover regression.
 *
 * Exercises the end-to-end plumbing that the unit suites cover only in
 * isolation:
 *
 *   1. Clicking the statusbar tasks slot toggles `tasksPopoverOpen`
 *      (unit-tested in `tasksPopover.shortcut.test.ts`).
 *   2. `+page.svelte` subscribes to that store and mounts `<TasksPopover>`
 *      with `open={$tasksPopoverOpen}` (unit-tested in
 *      `TasksPopover.test.ts` with a static `open=true` prop).
 *
 * What those two suites miss is the *click event plumbing* — specifically
 * the `<svelte:window onclick={handleClickOutside}>` handler inside
 * `TasksPopover`. On a real click the button's `onclick` fires first and
 * flips the store, then the same click bubbles up to the window handler.
 * If the render pass has committed by the time the window handler runs
 * and `popoverEl` is already bound, the target (the statusbar button) is
 * *not* inside the popover → `onClose()` fires → the popover closes
 * before the user ever sees it.
 *
 * This is the reason the issue only reproduces in a real browser/webview
 * and not in jsdom: jsdom defers the re-render to a microtask, so
 * `popoverEl` is still `undefined` when the window handler runs and the
 * short-circuit in the guard prevents the close.
 */
describe("Tasks popover smoke", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
    await openFixtureProject("simple-repo");
  });

  it("opens the popover when the statusbar slot is clicked", async () => {
    const slot = await $('[data-testid="statusbar-tasks-slot"]');
    await slot.waitForDisplayed({ timeout: 5000 });
    await slot.click();

    const popover = await $('[data-testid="tasks-popover"]');
    await popover.waitForExist({ timeout: 2000 });
    await expect(popover).toBeDisplayed();
  });

  it("Cmd+J also opens the popover", async () => {
    // Ensure the popover starts closed (previous test left it open).
    await browser.keys(["Escape"]);
    const beforePopover = await $('[data-testid="tasks-popover"]');
    await beforePopover.waitForExist({ reverse: true, timeout: 2000 });

    await browser.keys(["Meta", "j"]);
    const popover = await $('[data-testid="tasks-popover"]');
    await popover.waitForExist({ timeout: 2000 });
    await expect(popover).toBeDisplayed();
  });

  it("Escape closes the popover", async () => {
    await browser.keys(["Escape"]);
    const popover = await $('[data-testid="tasks-popover"]');
    await popover.waitForExist({ reverse: true, timeout: 2000 });
  });
});
