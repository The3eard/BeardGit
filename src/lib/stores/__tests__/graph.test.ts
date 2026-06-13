import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

vi.mock("../../api/tauri", () => ({
  getGraphViewport: vi.fn(async (offset: number, limit: number) => ({
    nodes: [],
    total_count: 0,
    offset,
    limit,
  })),
  refreshGraphLayout: vi.fn(),
  // stubs for other imports the module pulls in
  getCommitDetail: vi.fn(),
  getCommitFiles: vi.fn(),
  getDiffBetweenCommits: vi.fn(),
  getCommitFileDiff: vi.fn(),
  getUserIdentities: vi.fn(),
  getCommitRow: vi.fn(),
  getFileAtCommit: vi.fn(),
  getFileAtCommitText: vi.fn(),
}));

import {
  reloadGraph,
  graphOffset,
  viewport,
  reconcileViewport,
  refreshAndReloadGraph,
} from "../graph";
import type { GraphViewport, LayoutNode } from "$lib/types";
import { getGraphViewport, refreshGraphLayout } from "../../api/tauri";

function makeNode(oid: string): LayoutNode {
  return {
    oid,
    lane: 0,
    row: 0,
    refs: [],
    summary: "",
    author: "",
    email: "",
    timestamp: 0,
    is_merge: false,
    is_root: false,
    segment_group: 0,
  };
}

function makeViewport(nodes: LayoutNode[], offset = 0): GraphViewport {
  return {
    nodes,
    lane_segments: [],
    merge_curves: [],
    total_count: nodes.length,
    offset,
    visible_lane_count: 0,
    total_lane_count: 0,
    head_lane: null,
    has_more: false,
  };
}

describe("reloadGraph", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("re-fetches the viewport at the current offset", async () => {
    graphOffset.set(300);
    await reloadGraph();
    expect(getGraphViewport).toHaveBeenCalledWith(
      300,
      expect.any(Number),
      expect.any(Object),
    );
  });

  it("defaults to offset 0 when none has been set", async () => {
    graphOffset.set(0);
    await reloadGraph();
    expect(getGraphViewport).toHaveBeenCalledWith(
      0,
      expect.any(Number),
      expect.any(Object),
    );
  });
});

describe("reconcileViewport", () => {
  beforeEach(() => {
    viewport.set(null);
    graphOffset.set(0);
  });

  it("silent replaces when top_oid matches (no offset change)", () => {
    viewport.set(makeViewport([makeNode("a"), makeNode("b")]));
    graphOffset.set(7);
    const fresh = makeViewport([makeNode("a"), makeNode("b"), makeNode("c")]);
    reconcileViewport(fresh);
    expect(get(viewport)).toBe(fresh);
    expect(get(graphOffset)).toBe(7);
  });

  it("preserves scroll anchor when new commits land above the old top", () => {
    viewport.set(makeViewport([makeNode("old-top"), makeNode("b")]));
    graphOffset.set(10);
    // Fresh data has two new commits above the old top
    const fresh = makeViewport([
      makeNode("new-1"),
      makeNode("new-2"),
      makeNode("old-top"),
      makeNode("b"),
    ]);
    reconcileViewport(fresh);
    expect(get(viewport)).toBe(fresh);
    // Old top is now at index 2 in the fresh window → bump by 2
    expect(get(graphOffset)).toBe(12);
  });

  it("snaps to the top when the old top_oid is not in the fresh window", () => {
    // The anchor fell outside the fresh window entirely (e.g. a pull
    // rewrote history above it). Keeping the stale offset would scroll
    // to an unrelated row, so reconcile snaps back to the newest commits.
    viewport.set(makeViewport([makeNode("gone")]));
    graphOffset.set(5);
    const fresh = makeViewport([makeNode("new-1"), makeNode("new-2")]);
    reconcileViewport(fresh);
    expect(get(viewport)).toBe(fresh);
    expect(get(graphOffset)).toBe(0);
  });

  it("atomic swaps when no cached viewport exists (cold start)", () => {
    viewport.set(null);
    graphOffset.set(0);
    const fresh = makeViewport([makeNode("first"), makeNode("second")]);
    reconcileViewport(fresh);
    expect(get(viewport)).toBe(fresh);
    expect(get(graphOffset)).toBe(0);
  });
});

describe("refreshAndReloadGraph", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    viewport.set(null);
    graphOffset.set(0);
  });

  it("rebuilds layout then reconciles via the reconciler", async () => {
    const fresh = makeViewport([makeNode("a")]);
    vi.mocked(getGraphViewport).mockResolvedValueOnce(fresh);
    await refreshAndReloadGraph();
    expect(refreshGraphLayout).toHaveBeenCalledTimes(1);
    expect(getGraphViewport).toHaveBeenCalledWith(
      0,
      expect.any(Number),
      expect.any(Object),
    );
    expect(get(viewport)).toBe(fresh);
  });

  it("still reconciles when refresh_graph_layout errors (best-effort)", async () => {
    vi.mocked(refreshGraphLayout).mockRejectedValueOnce(new Error("no repo"));
    const fresh = makeViewport([makeNode("x")]);
    vi.mocked(getGraphViewport).mockResolvedValueOnce(fresh);
    await refreshAndReloadGraph();
    expect(get(viewport)).toBe(fresh);
  });

  it("preserves the scroll anchor across a refresh that grows the head", async () => {
    viewport.set(makeViewport([makeNode("anchor"), makeNode("b")]));
    graphOffset.set(20);
    const fresh = makeViewport([
      makeNode("new"),
      makeNode("anchor"),
      makeNode("b"),
    ]);
    vi.mocked(getGraphViewport).mockResolvedValueOnce(fresh);
    await refreshAndReloadGraph();
    expect(get(graphOffset)).toBe(21);
    expect(get(viewport)).toBe(fresh);
  });
});
