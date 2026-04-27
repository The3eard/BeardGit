import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { get } from "svelte/store";

// Mock the Tauri API before importing the store so the mocks are hot.
vi.mock("$lib/api/tauri", () => ({
  getSidebarNavLayout: vi.fn(async () => ({
    order: ["changes", "graph"],
    hidden: ["bisect"],
  })),
  setSidebarNavLayout: vi.fn(async () => {}),
}));

import {
  sidebarLayout,
  loadSidebarLayout,
  updateLayout,
  DEBOUNCE_MS,
} from "./sidebarLayout";
import * as api from "$lib/api/tauri";

beforeEach(() => {
  vi.useFakeTimers();
  sidebarLayout.set({ order: [], hidden: [] });
  vi.mocked(api.setSidebarNavLayout).mockClear();
  vi.mocked(api.getSidebarNavLayout).mockClear();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("sidebarLayout store", () => {
  it("hydrates from the backend via loadSidebarLayout", async () => {
    await loadSidebarLayout();
    expect(get(sidebarLayout)).toEqual({
      order: ["changes", "graph"],
      hidden: ["bisect"],
    });
    expect(api.getSidebarNavLayout).toHaveBeenCalledTimes(1);
  });

  it("updateLayout patches the store synchronously", () => {
    updateLayout({ order: ["graph"] });
    expect(get(sidebarLayout).order).toEqual(["graph"]);
    expect(get(sidebarLayout).hidden).toEqual([]);
  });

  it("updateLayout debounces the backend save by DEBOUNCE_MS", () => {
    updateLayout({ order: ["graph"] });
    updateLayout({ hidden: ["bisect"] });
    updateLayout({ order: ["changes", "graph"] });
    expect(api.setSidebarNavLayout).not.toHaveBeenCalled();

    vi.advanceTimersByTime(DEBOUNCE_MS);
    expect(api.setSidebarNavLayout).toHaveBeenCalledTimes(1);
    expect(api.setSidebarNavLayout).toHaveBeenLastCalledWith({
      order: ["changes", "graph"],
      hidden: ["bisect"],
    });
  });
});
