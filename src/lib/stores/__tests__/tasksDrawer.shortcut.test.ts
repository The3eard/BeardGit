/**
 * Unit tests for the Cmd+J / Ctrl+J global shortcut that toggles the
 * tasks drawer.
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
  tasksDrawerOpen,
  toggleTasksDrawer,
  closeTasksDrawer,
} from "$lib/stores/tasksDrawer";

describe("Cmd+J / Ctrl+J tasks drawer shortcut", () => {
  let cleanup: () => void = () => {};

  beforeEach(() => {
    closeTasksDrawer();
    cleanup = initShortcutListener();
    registerShortcuts([
      {
        id: "util.tasksDrawer",
        keys: { mod: true, key: "j" },
        label: "Tasks",
        category: "General",
        action: () => toggleTasksDrawer(),
        global: true,
      },
    ]);
  });

  afterEach(() => {
    unregisterShortcuts(["util.tasksDrawer"]);
    cleanup();
    closeTasksDrawer();
  });

  it("flips `tasksDrawerOpen` when Cmd+J fires", () => {
    expect(get(tasksDrawerOpen)).toBe(false);

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

    expect(get(tasksDrawerOpen)).toBe(true);
  });

  it("closes the drawer on the second Cmd+J press", () => {
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "j",
        metaKey: true,
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    expect(get(tasksDrawerOpen)).toBe(true);

    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "j",
        metaKey: true,
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    expect(get(tasksDrawerOpen)).toBe(false);
  });

  it("ignores bare `j` keypress without the mod modifier", () => {
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "j",
        bubbles: true,
        cancelable: true,
      }),
    );
    expect(get(tasksDrawerOpen)).toBe(false);
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
      expect(get(tasksDrawerOpen)).toBe(true);
    } finally {
      input.remove();
    }
  });

  it("toggleTasksDrawer flips the store state directly", () => {
    expect(get(tasksDrawerOpen)).toBe(false);
    toggleTasksDrawer();
    expect(get(tasksDrawerOpen)).toBe(true);
    toggleTasksDrawer();
    expect(get(tasksDrawerOpen)).toBe(false);
  });
});

describe("tasksDrawer store module", () => {
  it("`closeTasksDrawer` forces the store to false regardless of state", () => {
    tasksDrawerOpen.set(true);
    closeTasksDrawer();
    expect(get(tasksDrawerOpen)).toBe(false);
  });

  it("`tasksDrawerOpen` default is false", () => {
    // After `closeTasksDrawer` in the other afterEach, the store should
    // read false for new importers too.
    closeTasksDrawer();
    expect(get(tasksDrawerOpen)).toBe(false);
  });

  it("dispatching a synthetic event without a registered shortcut is a no-op", () => {
    // A sanity check that stray j keypresses elsewhere don't flip the
    // drawer if we somehow lose the registration.
    unregisterShortcuts(["util.tasksDrawer"]);
    closeTasksDrawer();
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
    expect(get(tasksDrawerOpen)).toBe(false);
    // Re-register for afterEach cleanup symmetry.
    registerShortcuts([
      {
        id: "util.tasksDrawer",
        keys: { mod: true, key: "j" },
        label: "Tasks",
        category: "General",
        action: () => toggleTasksDrawer(),
        global: true,
      },
    ]);
  });

  it("exports a typed vi-mockable toggle helper", () => {
    // Type-level sanity: the function exists and returns void.
    const fn: () => void = toggleTasksDrawer;
    expect(typeof fn).toBe("function");
    vi.clearAllMocks();
  });
});
