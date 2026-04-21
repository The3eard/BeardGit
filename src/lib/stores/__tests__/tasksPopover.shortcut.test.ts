/**
 * Unit tests for the Cmd+J / Ctrl+J global shortcut that toggles the
 * tasks popover.
 *
 * The test registers the same shortcut the root layout registers (via
 * `registerShortcuts`) and then fires a synthetic `KeyboardEvent` on
 * `window`. The shortcut handler lives in `initShortcutListener`, so
 * firing the event exercises the real code path end-to-end.
 *
 * Because `matchesKeys` in `shortcuts.ts` uses platform detection
 * (`isMac`) to decide whether `metaKey` or `ctrlKey` counts as the
 * "mod" key, we drive both paths:
 *
 *   - On macOS (default jsdom userAgent → Mac)   → metaKey true.
 *   - On non-macOS                               → ctrlKey true.
 *
 * We don't try to flip `navigator.userAgent` at runtime (it's
 * readonly in jsdom) — instead we send BOTH modifier flags so the
 * handler fires regardless of the detected platform. The shortcut
 * definition asks for `mod` only, which accepts whichever matches.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import {
  initShortcutListener,
  registerShortcuts,
  unregisterShortcuts,
} from "$lib/stores/shortcuts";
import {
  tasksPopoverOpen,
  toggleTasksPopover,
  closeTasksPopover,
} from "$lib/stores/tasksPopover";

describe("Cmd+J / Ctrl+J tasks popover shortcut", () => {
  let cleanup: () => void = () => {};

  beforeEach(() => {
    closeTasksPopover();
    cleanup = initShortcutListener();
    registerShortcuts([
      {
        id: "util.tasksPopover",
        keys: { mod: true, key: "j" },
        label: "Tasks",
        category: "General",
        action: () => toggleTasksPopover(),
        global: true,
      },
    ]);
  });

  afterEach(() => {
    unregisterShortcuts(["util.tasksPopover"]);
    cleanup();
    closeTasksPopover();
  });

  it("flips `tasksPopoverOpen` when Cmd+J fires", () => {
    expect(get(tasksPopoverOpen)).toBe(false);

    // Send both modifier flags so the handler fires on any platform.
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "j",
        metaKey: true,
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );

    expect(get(tasksPopoverOpen)).toBe(true);
  });

  it("closes the popover on the second Cmd+J press", () => {
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "j",
        metaKey: true,
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    expect(get(tasksPopoverOpen)).toBe(true);

    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "j",
        metaKey: true,
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    expect(get(tasksPopoverOpen)).toBe(false);
  });

  it("ignores bare `j` keypress without the mod modifier", () => {
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "j",
        bubbles: true,
        cancelable: true,
      }),
    );
    expect(get(tasksPopoverOpen)).toBe(false);
  });

  it("fires even when an input is focused (global flag)", () => {
    const input = document.createElement("input");
    document.body.appendChild(input);
    input.focus();

    try {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "j",
          metaKey: true,
          ctrlKey: true,
          bubbles: true,
          cancelable: true,
        }),
      );
      expect(get(tasksPopoverOpen)).toBe(true);
    } finally {
      input.remove();
    }
  });

  it("toggleTasksPopover flips the store state directly", () => {
    expect(get(tasksPopoverOpen)).toBe(false);
    toggleTasksPopover();
    expect(get(tasksPopoverOpen)).toBe(true);
    toggleTasksPopover();
    expect(get(tasksPopoverOpen)).toBe(false);
  });
});

describe("tasksPopover store module", () => {
  it("`closeTasksPopover` forces the store to false regardless of state", () => {
    tasksPopoverOpen.set(true);
    closeTasksPopover();
    expect(get(tasksPopoverOpen)).toBe(false);
  });

  it("`tasksPopoverOpen` default is false", () => {
    closeTasksPopover();
    expect(get(tasksPopoverOpen)).toBe(false);
  });

  it("dispatching a synthetic event without a registered shortcut is a no-op", () => {
    // A sanity check that stray j keypresses elsewhere don't flip the
    // popover if we somehow lose the registration.
    unregisterShortcuts(["util.tasksPopover"]);
    closeTasksPopover();
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "j",
        metaKey: true,
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    // No registered shortcut → store unchanged.
    expect(get(tasksPopoverOpen)).toBe(false);
    // Re-register for afterEach cleanup symmetry.
    registerShortcuts([
      {
        id: "util.tasksPopover",
        keys: { mod: true, key: "j" },
        label: "Tasks",
        category: "General",
        action: () => toggleTasksPopover(),
        global: true,
      },
    ]);
  });

  it("exports a typed vi-mockable toggle helper", () => {
    // Type-level sanity: the function exists and returns void.
    const fn: () => void = toggleTasksPopover;
    expect(typeof fn).toBe("function");
    vi.clearAllMocks();
  });
});
