/**
 * Unit tests for the one-level submenu support added to
 * `ContextMenu.svelte`. Only covers the submenu API — existing flat
 * menu behaviour is covered by the consumers' integration tests.
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
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
