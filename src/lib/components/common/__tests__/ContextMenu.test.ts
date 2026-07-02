/**
 * Unit tests for the one-level submenu support added to
 * `ContextMenu.svelte`. Only covers the submenu API — existing flat
 * menu behaviour is covered by the consumers' integration tests.
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import { flushSync } from "svelte";
import ContextMenu from "../ContextMenu.svelte";
import type { MenuItem } from "../ContextMenu.svelte";

afterEach(() => cleanup());

describe("ContextMenu submenu", () => {
  it("renders a chevron on items that have children", () => {
    const items: MenuItem[] = [
      { label: "Flat" },
      {
        label: "Has submenu",
        children: [{ label: "Child A" }, { label: "Child B" }],
      },
    ];
    const { getByText, container } = render(ContextMenu, {
      props: { items, x: 10, y: 10, visible: true, onClose: () => {} },
    });
    expect(getByText("Has submenu")).toBeTruthy();
    const chevrons = container.querySelectorAll(".submenu-chevron");
    expect(chevrons.length).toBe(1);
  });

  it("renders a flyout after hovering a parent item", async () => {
    const items: MenuItem[] = [
      {
        label: "Parent",
        children: [{ label: "Alpha" }, { label: "Beta" }],
      },
    ];
    const { getByText, container } = render(ContextMenu, {
      props: { items, x: 10, y: 10, visible: true, onClose: () => {} },
    });
    const parent = getByText("Parent");
    await fireEvent.mouseEnter(parent);
    expect(container.querySelector(".submenu")).not.toBeNull();
    expect(getByText("Alpha")).toBeTruthy();
    expect(getByText("Beta")).toBeTruthy();
  });

  it("fires the child action and calls onClose when a leaf is clicked", async () => {
    const onAlpha = vi.fn();
    const onClose = vi.fn();
    const items: MenuItem[] = [
      {
        label: "Parent",
        children: [{ label: "Alpha", action: onAlpha }],
      },
    ];
    const { getByText } = render(ContextMenu, {
      props: { items, x: 10, y: 10, visible: true, onClose },
    });
    await fireEvent.mouseEnter(getByText("Parent"));
    await fireEvent.click(getByText("Alpha"));
    expect(onAlpha).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("does not dispatch the parent action when it has children", async () => {
    const onParent = vi.fn();
    const onClose = vi.fn();
    const items: MenuItem[] = [
      {
        label: "Parent",
        action: onParent,
        children: [{ label: "Alpha" }],
      },
    ];
    const { getByText } = render(ContextMenu, {
      props: { items, x: 10, y: 10, visible: true, onClose },
    });
    await fireEvent.click(getByText("Parent"));
    expect(onParent).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });
});

describe("ContextMenu edge clamping", () => {
  // jsdom reports 0 for offsetWidth/Height, so stub the menu box to a known
  // size and shrink the viewport to exercise the flip/clamp path.
  const MENU_W = 200;
  const MENU_H = 150;

  function stubLayout(viewportW: number, viewportH: number) {
    const defs = [
      ["offsetWidth", MENU_W],
      ["offsetHeight", MENU_H],
    ] as const;
    for (const [prop, val] of defs) {
      Object.defineProperty(HTMLElement.prototype, prop, {
        configurable: true,
        get() {
          return val;
        },
      });
    }
    Object.defineProperty(window, "innerWidth", { configurable: true, value: viewportW });
    Object.defineProperty(window, "innerHeight", { configurable: true, value: viewportH });
  }

  afterEach(() => {
    for (const prop of ["offsetWidth", "offsetHeight"]) {
      delete (HTMLElement.prototype as unknown as Record<string, unknown>)[prop];
    }
  });

  it("clamps a menu opened near the right/bottom edge back inside the viewport", () => {
    stubLayout(300, 300);
    const items: MenuItem[] = [{ label: "One", action: () => {} }];
    // cursor deep in the bottom-right corner of a 300x300 window
    const { container } = render(ContextMenu, {
      props: { items, x: 290, y: 290, visible: true, onClose: () => {} },
    });
    flushSync();

    const menu = container.querySelector(".context-menu") as HTMLElement;
    const left = parseFloat(menu.style.left);
    const top = parseFloat(menu.style.top);

    expect(menu.style.visibility).toBe("visible");
    expect(left).toBeGreaterThanOrEqual(8);
    expect(left + MENU_W).toBeLessThanOrEqual(300 - 8);
    expect(top).toBeGreaterThanOrEqual(8);
    expect(top + MENU_H).toBeLessThanOrEqual(300 - 8);
  });
});
