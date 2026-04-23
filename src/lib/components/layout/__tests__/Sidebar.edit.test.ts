import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, cleanup, fireEvent } from "@testing-library/svelte";
import { get } from "svelte/store";
import Sidebar from "../Sidebar.svelte";
import { providerStatus } from "$lib/stores/provider";
import { sidebarLayout } from "$lib/stores/sidebarLayout";
import { toasts } from "$lib/stores/toast";

afterEach(() => cleanup());

beforeEach(() => {
  providerStatus.set({ providers: [], active_index: null });
  sidebarLayout.set({ order: [], hidden: [] });
  toasts.set([]);
});

async function clickPencil(getByTestId: (id: string) => HTMLElement) {
  await fireEvent.click(getByTestId("sidebar-edit-toggle"));
}

describe("Sidebar — edit mode", () => {
  it("enters edit mode when the pencil button is clicked", async () => {
    const { getByTestId, queryByTestId } = render(Sidebar, {
      props: { activeView: "graph" },
    });
    expect(queryByTestId("sidebar-edit-done")).toBeNull();
    await clickPencil(getByTestId);
    expect(getByTestId("sidebar-edit-done")).toBeTruthy();
  });

  it("exits edit mode via Done", async () => {
    const { getByTestId, queryByTestId } = render(Sidebar, {
      props: { activeView: "graph" },
    });
    await clickPencil(getByTestId);
    await fireEvent.click(getByTestId("sidebar-edit-done"));
    expect(queryByTestId("sidebar-edit-done")).toBeNull();
  });

  it("exits edit mode via Escape", async () => {
    const { getByTestId, queryByTestId } = render(Sidebar, {
      props: { activeView: "graph" },
    });
    await clickPencil(getByTestId);
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(queryByTestId("sidebar-edit-done")).toBeNull();
  });

  it("shows all items including hidden ones in edit mode", async () => {
    sidebarLayout.set({ order: [], hidden: ["bisect"] });
    const { getByTestId, queryByTestId } = render(Sidebar, {
      props: { activeView: "graph" },
    });
    expect(queryByTestId("nav-bisect")).toBeNull();
    await clickPencil(getByTestId);
    expect(getByTestId("nav-bisect")).toBeTruthy();
  });

  it("eye toggle flips an item between visible and hidden", async () => {
    const { getByTestId } = render(Sidebar, { props: { activeView: "graph" } });
    await clickPencil(getByTestId);
    await fireEvent.click(getByTestId("sidebar-hide-bisect"));
    expect(get(sidebarLayout).hidden).toContain("bisect");
    await fireEvent.click(getByTestId("sidebar-hide-bisect"));
    expect(get(sidebarLayout).hidden).not.toContain("bisect");
  });

  it("guards the last visible item — toggling is a no-op and a toast is shown", async () => {
    // Hide everything except graph.
    sidebarLayout.set({
      order: [],
      hidden: [
        "changes",
        "branches",
        "tags",
        "stashes",
        "worktrees",
        "reflog",
        "bisect",
        "submodules",
        "ai-config",
        "ai-sessions",
      ],
    });
    const { getByTestId } = render(Sidebar, { props: { activeView: "graph" } });
    await clickPencil(getByTestId);
    const before = [...get(sidebarLayout).hidden];
    await fireEvent.click(getByTestId("sidebar-hide-graph"));
    expect(get(sidebarLayout).hidden).toEqual(before);
    expect(get(toasts).some((t) => t.message.includes("At least one"))).toBe(
      true,
    );
  });

  it("ArrowDown on the drag handle moves the item down one slot", async () => {
    const { getByTestId } = render(Sidebar, { props: { activeView: "graph" } });
    await clickPencil(getByTestId);
    const handle = getByTestId("sidebar-reorder-graph");
    await fireEvent.keyDown(handle, { key: "ArrowDown" });
    expect(get(sidebarLayout).order[0]).toBe("changes");
    expect(get(sidebarLayout).order[1]).toBe("graph");
  });

  it("Reset restores defaults", async () => {
    sidebarLayout.set({
      order: ["ai-sessions", "graph"],
      hidden: ["bisect"],
    });
    const { getByTestId } = render(Sidebar, { props: { activeView: "graph" } });
    await clickPencil(getByTestId);
    await fireEvent.click(getByTestId("sidebar-edit-reset"));
    expect(get(sidebarLayout).hidden).toEqual([]);
    expect(get(sidebarLayout).order.length).toBe(11);
    expect(get(sidebarLayout).order[0]).toBe("graph");
  });

  it("forces edit mode off when the sidebar collapses", async () => {
    const { getByTestId, queryByTestId, rerender } = render(Sidebar, {
      props: { activeView: "graph", collapsed: false },
    });
    await clickPencil(getByTestId);
    expect(getByTestId("sidebar-edit-done")).toBeTruthy();
    await rerender({ activeView: "graph", collapsed: true });
    expect(queryByTestId("sidebar-edit-done")).toBeNull();
  });
});
