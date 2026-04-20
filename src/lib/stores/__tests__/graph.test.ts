import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

vi.mock("../../api/tauri", () => ({
  getGraphViewport: vi.fn(async (offset: number, limit: number) => ({
    nodes: [],
    total_count: 0,
    offset,
    limit,
  })),
  // stubs for other imports the module pulls in
  getCommitDetail: vi.fn(),
  getCommitFiles: vi.fn(),
  getDiffBetweenCommits: vi.fn(),
  getCommitFileDiff: vi.fn(),
  getUserIdentities: vi.fn(),
  getCommitRow: vi.fn(),
  getFileAtCommit: vi.fn(),
}));

import { reloadGraph, graphOffset } from "../graph";
import { getGraphViewport } from "../../api/tauri";

describe("reloadGraph", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("re-fetches the viewport at the current offset", async () => {
    graphOffset.set(300);
    await reloadGraph();
    expect(getGraphViewport).toHaveBeenCalledWith(300, expect.any(Number));
  });

  it("defaults to offset 0 when none has been set", async () => {
    graphOffset.set(0);
    await reloadGraph();
    expect(getGraphViewport).toHaveBeenCalledWith(0, expect.any(Number));
  });
});
