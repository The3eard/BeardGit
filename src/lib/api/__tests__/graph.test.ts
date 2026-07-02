/**
 * Graph viewport wrappers — verify each TS function maps to the expected
 * Tauri command name and payload shape, including the optional
 * GraphViewOptions parameters added for the graph view modes.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

import { getGraphViewport, loadGraphChunk } from "../tauri";

beforeEach(() => {
  mocks.invoke.mockReset();
});

describe("graph wrappers", () => {
  it("getGraphViewport defaults the mode params to null", async () => {
    mocks.invoke.mockResolvedValue({ nodes: [] });
    await getGraphViewport(0, 50);
    expect(mocks.invoke).toHaveBeenCalledWith("get_graph_viewport", {
      offset: 0,
      limit: 50,
      firstParent: null,
      branch: null,
      maxLanes: null,
    });
  });

  it("getGraphViewport forwards firstParent, branch and maxLanes", async () => {
    mocks.invoke.mockResolvedValue({ nodes: [] });
    await getGraphViewport(10, 20, { firstParent: true, branch: "main", maxLanes: 12 });
    expect(mocks.invoke).toHaveBeenCalledWith("get_graph_viewport", {
      offset: 10,
      limit: 20,
      firstParent: true,
      branch: "main",
      maxLanes: 12,
    });
  });

  it("loadGraphChunk invokes 'load_graph_chunk' with mode params", async () => {
    mocks.invoke.mockResolvedValue({ nodes: [], has_more: true });
    const out = await loadGraphChunk(100, 50, { firstParent: true, branch: "origin/dev" });
    expect(mocks.invoke).toHaveBeenCalledWith("load_graph_chunk", {
      offset: 100,
      limit: 50,
      firstParent: true,
      branch: "origin/dev",
      maxLanes: null,
      anchor: null,
    });
    expect(out).toEqual({ nodes: [], has_more: true });
  });

  it("loadGraphChunk forwards the anchor OID for sequential scrolls", async () => {
    mocks.invoke.mockResolvedValue({ nodes: [], has_more: true });
    await loadGraphChunk(300, 50, { firstParent: true }, "deadbeef");
    expect(mocks.invoke).toHaveBeenCalledWith("load_graph_chunk", {
      offset: 300,
      limit: 50,
      firstParent: true,
      branch: null,
      maxLanes: null,
      anchor: "deadbeef",
    });
  });
});
