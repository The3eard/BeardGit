/**
 * Factories for commit-graph fixtures: CommitInfo, LayoutNode,
 * LaneSegment, MergeCurve, GraphViewport, CommitFileChange,
 * CommitStats.
 *
 * `makeGraphViewport({ count })` is the workhorse — it generates a
 * linear chain of `count` commits with realistic refs/timestamps so
 * the graph view has something to render. Use overrides on individual
 * nodes when a test needs a specific shape (merges, multi-lane).
 */

import type {
  CommitFileChange,
  CommitInfo,
  CommitStats,
  GraphViewport,
  LaneSegment,
  LayoutNode,
  MergeCurve,
} from "../../lib/types";

const BASE_TIMESTAMP = 1715000000; // 2024-05-06 — stable so screenshots are deterministic.

function oid(seed: number): string {
  return seed.toString(16).padStart(40, "0");
}

export function makeCommitInfo(
  overrides: Partial<CommitInfo> = {},
): CommitInfo {
  return {
    oid: oid(1),
    summary: "feat: add example feature",
    body: "",
    author: "Adolfo Fuentes",
    email: "adolfo@example.com",
    timestamp: BASE_TIMESTAMP,
    parents: [oid(2)],
    refs: [],
    ...overrides,
  };
}

export function makeLayoutNode(
  overrides: Partial<LayoutNode> = {},
): LayoutNode {
  return {
    oid: oid(1),
    lane: 0,
    row: 0,
    refs: [],
    summary: "feat: add example feature",
    author: "Adolfo Fuentes",
    email: "adolfo@example.com",
    timestamp: BASE_TIMESTAMP,
    is_merge: false,
    is_root: false,
    segment_group: 0,
    ...overrides,
  };
}

export function makeLaneSegment(
  overrides: Partial<LaneSegment> = {},
): LaneSegment {
  return {
    lane: 0,
    start_row: 0,
    end_row: 10,
    color_index: 0,
    recycled: false,
    sync_state: "Synced",
    group_id: 0,
    ...overrides,
  };
}

export function makeMergeCurve(
  overrides: Partial<MergeCurve> = {},
): MergeCurve {
  return {
    from_lane: 1,
    from_row: 1,
    to_lane: 0,
    to_row: 0,
    color_index: 1,
    group_id: 1,
    ...overrides,
  };
}

export function makeCommitStats(
  overrides: Partial<CommitStats> = {},
): CommitStats {
  return {
    files_changed: 3,
    insertions: 42,
    deletions: 12,
    ...overrides,
  };
}

export function makeCommitFileChange(
  overrides: Partial<CommitFileChange> = {},
): CommitFileChange {
  return {
    path: "src/lib/feature.ts",
    status: "M",
    ...overrides,
  };
}

export interface GraphViewportOpts {
  /** Number of commits to generate. Default 25. */
  count?: number;
  /**
   * Index (0 = newest) of the commit that should carry HEAD/branch
   * refs. Default 0.
   */
  headIndex?: number;
  /** Branch name placed on the head commit. Default "feat/example". */
  headBranch?: string;
  /** Apply per-node overrides. Receives the auto-generated node. */
  decorate?: (node: LayoutNode, index: number) => Partial<LayoutNode>;
}

export function makeGraphViewport(
  opts: GraphViewportOpts = {},
  overrides: Partial<GraphViewport> = {},
): GraphViewport {
  const count = opts.count ?? 25;
  const headIndex = opts.headIndex ?? 0;
  const headBranch = opts.headBranch ?? "feat/example";

  const summaries = [
    "feat(graph): add lane recycling for long-lived branches",
    "fix(commit): handle empty diff payloads gracefully",
    "chore: bump dependencies",
    "refactor(stores): collapse refresh listeners",
    "docs: update CONTRIBUTING with branch policy",
    "test: cover the rebase abort path",
    "perf(graph): memoise lane assignment",
    "feat(ui): keyboard shortcut for command palette",
  ];

  const nodes: LayoutNode[] = Array.from({ length: count }, (_, i) => {
    const base = makeLayoutNode({
      oid: oid(i + 1),
      row: i,
      lane: 0,
      summary: summaries[i % summaries.length] ?? "chore: misc",
      timestamp: BASE_TIMESTAMP - i * 3600,
      refs:
        i === headIndex
          ? ["HEAD", `refs/heads/${headBranch}`, `refs/remotes/origin/${headBranch}`]
          : [],
    });
    return { ...base, ...(opts.decorate?.(base, i) ?? {}) };
  });

  const lane_segments: LaneSegment[] = [
    makeLaneSegment({ lane: 0, start_row: 0, end_row: count - 1 }),
  ];

  return {
    nodes,
    lane_segments,
    merge_curves: [],
    total_count: count,
    offset: 0,
    visible_lane_count: 1,
    total_lane_count: 1,
    head_lane: 0,
    has_more: false,
    ...overrides,
  };
}
