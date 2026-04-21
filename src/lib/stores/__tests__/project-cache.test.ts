import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

// api/tauri must be mocked before importing modules that chain into it
// (project-cache → graph → api/tauri at import time).
vi.mock("../../api/tauri", () => ({
  getProjectSnapshot: vi.fn(),
  saveProjectSnapshot: vi.fn(),
  getStatusSummary: vi.fn(),
  getGraphViewport: vi.fn(),
  getCommitDetail: vi.fn(),
  getCommitFiles: vi.fn(),
  getDiffBetweenCommits: vi.fn(),
  getCommitFileDiff: vi.fn(),
  getUserIdentities: vi.fn(),
  getCommitRow: vi.fn(),
  getFileAtCommit: vi.fn(),
  refreshGraphLayout: vi.fn(),
}));

import {
  isCacheFresh,
  GRAPH_CACHE_TTL_MS,
  hydrateSnapshotCache,
  _clearSnapshotCacheForTests,
  getCachedSnapshot,
  restorePersistedViewport,
} from "../project-cache";
import { viewport, graphOffset } from "../graph";
import type { ProjectSnapshot, LayoutNode } from "$lib/types";

function makeSnap(overrides: Partial<ProjectSnapshot> = {}): ProjectSnapshot {
  return {
    path: "/Users/test/project",
    head_branch: "main",
    ahead: 0,
    behind: 0,
    staged: 0,
    unstaged: 0,
    untracked: 0,
    conflicted: 0,
    stash_count: 0,
    change_count: 0,
    graph_viewport_cache: null,
    ...overrides,
  };
}

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

describe("project-cache graph slice", () => {
  it("rejects caches older than 7 days", () => {
    const stale = Date.now() - GRAPH_CACHE_TTL_MS - 1;
    expect(isCacheFresh(stale)).toBe(false);
  });

  it("accepts recent caches", () => {
    expect(isCacheFresh(Date.now())).toBe(true);
  });

  it("accepts caches exactly at the TTL boundary", () => {
    const boundary = Date.now() - GRAPH_CACHE_TTL_MS;
    expect(isCacheFresh(boundary)).toBe(true);
  });

  it("exposes the TTL as 7 days in ms", () => {
    expect(GRAPH_CACHE_TTL_MS).toBe(7 * 24 * 60 * 60 * 1000);
  });
});

describe("restorePersistedViewport", () => {
  beforeEach(() => {
    _clearSnapshotCacheForTests();
    viewport.set(null);
    graphOffset.set(0);
  });

  it("returns false when no snapshot is primed", () => {
    expect(restorePersistedViewport("/missing/project")).toBe(false);
    expect(get(viewport)).toBeNull();
  });

  it("returns false when the snapshot lacks a graph viewport slice", () => {
    hydrateSnapshotCache("/p", makeSnap({ graph_viewport_cache: null }));
    expect(restorePersistedViewport("/p")).toBe(false);
    expect(get(viewport)).toBeNull();
  });

  it("returns false when the cached slice is stale", () => {
    hydrateSnapshotCache(
      "/p",
      makeSnap({
        graph_viewport_cache: {
          nodes: [makeNode("abc")],
          total_count: 1,
          head_oid: "abc",
          top_oid: "abc",
          offset: 0,
          cached_at: Date.now() - GRAPH_CACHE_TTL_MS - 60_000,
        },
      }),
    );
    expect(restorePersistedViewport("/p")).toBe(false);
    expect(get(viewport)).toBeNull();
  });

  it("installs the cached viewport + offset on a fresh hit", () => {
    const nodes = [makeNode("abc"), makeNode("def")];
    hydrateSnapshotCache(
      "/p",
      makeSnap({
        graph_viewport_cache: {
          nodes,
          total_count: 42,
          head_oid: "abc",
          top_oid: "abc",
          offset: 15,
          cached_at: Date.now(),
        },
      }),
    );
    expect(restorePersistedViewport("/p")).toBe(true);
    const vp = get(viewport);
    expect(vp).not.toBeNull();
    expect(vp?.nodes).toEqual(nodes);
    expect(vp?.total_count).toBe(42);
    expect(vp?.offset).toBe(15);
    // Lane geometry is zeroed — the fresh refresh repopulates it.
    expect(vp?.lane_segments).toEqual([]);
    expect(vp?.merge_curves).toEqual([]);
    expect(get(graphOffset)).toBe(15);
  });

  it("exposes the in-memory cache via getCachedSnapshot", () => {
    const snap = makeSnap();
    hydrateSnapshotCache("/p", snap);
    expect(getCachedSnapshot("/p")).toBe(snap);
    expect(getCachedSnapshot("/other")).toBeNull();
  });
});
