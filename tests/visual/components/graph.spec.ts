/**
 * Per-state baselines for the commit Graph view.
 *
 * The graph is canvas-rendered, so we can't easily click individual
 * nodes from Playwright; the variation here comes from different
 * `GraphViewport` shapes (empty, single-lane chain, multi-lane with
 * merge curves). Hover / selection states will need a separate
 * approach (likely synthetic store sets) — out of scope for v1.
 */

import { expect, test } from "@playwright/test";

import {
  applyTheme,
  clickNav,
  installBootstrapMocks,
  THEME_MODES,
  waitForAppReady,
  waitForGraphPainted,
  GRAPH_CANVAS_PIXEL_BUDGET,
} from "../helpers";
import {
  makeGraphViewport,
  makeLaneSegment,
  makeMergeCurve,
  makeProjectInfo,
} from "../../../src/test/fixtures";
import type { GraphViewport } from "../../../src/lib/types";

const PROJECT = makeProjectInfo();

const SCENARIOS: Record<string, () => GraphViewport> = {
  empty: () =>
    makeGraphViewport(
      { count: 0 },
      {
        nodes: [],
        lane_segments: [],
        merge_curves: [],
        total_count: 0,
        visible_lane_count: 0,
        total_lane_count: 0,
        head_lane: null,
      },
    ),
  "single-lane": () => makeGraphViewport({ count: 12 }),
  "long-chain": () => makeGraphViewport({ count: 50 }),
  "multi-lane-merges": () => {
    const viewport = makeGraphViewport({
      count: 18,
      decorate: (node, i) => {
        // Alternate between two lanes and mark every 4th as a merge.
        const lane = i % 4 === 0 ? 0 : i % 2 === 0 ? 1 : 0;
        return {
          lane,
          is_merge: i % 5 === 0 && i > 0,
          parents: i % 5 === 0 && i > 0 ? [
            (i + 1).toString(16).padStart(40, "0"),
            (i + 2).toString(16).padStart(40, "0"),
          ] : undefined,
        };
      },
    });
    viewport.lane_segments = [
      makeLaneSegment({ lane: 0, start_row: 0, end_row: 17, color_index: 0 }),
      makeLaneSegment({ lane: 1, start_row: 1, end_row: 16, color_index: 1 }),
    ];
    viewport.merge_curves = [
      makeMergeCurve({ from_lane: 1, from_row: 5, to_lane: 0, to_row: 5, color_index: 1 }),
      makeMergeCurve({ from_lane: 1, from_row: 10, to_lane: 0, to_row: 10, color_index: 1 }),
    ];
    viewport.visible_lane_count = 2;
    viewport.total_lane_count = 2;
    return viewport;
  },
  /**
   * The exact layout `graph-builder` emits for a merged branch followed by a
   * tip that rejoins the mainline — both curve shapes the renderer draws:
   *
   *   row 0  m   lane 0  merge(b1, b2)   → curve (0,0)→(1,2) bends at the top
   *   row 1  b1  lane 0
   *   row 2  b2  lane 1                  lane 1 opens at row 0, closes here
   *   row 3  base lane 0                 ← curve (1,2)→(0,3) bends at the bottom
   *   row 4  t   lane 1  tip → u         lane 1 reopens, closes at row 5
   *   row 5  f1  lane 0
   *   row 6  u   lane 0                  ← curve (1,4)→(0,6)
   *   row 7  v   lane 0
   *
   * Lane 1 used to run to the last row after each merge (a ghost line), and
   * the segment clip under the top bend disagreed with the curve geometry.
   */
  "merge-and-rejoin": () => {
    const viewport = makeGraphViewport({
      count: 8,
      decorate: (_node, i) => ({
        lane: i === 2 || i === 4 ? 1 : 0,
        segment_group: i === 2 ? 1 : i === 4 ? 2 : 0,
        is_merge: i === 0,
      }),
    });
    viewport.lane_segments = [
      makeLaneSegment({ lane: 0, start_row: 0, end_row: 7, color_index: 0, group_id: 0 }),
      makeLaneSegment({ lane: 1, start_row: 0, end_row: 2, color_index: 1, group_id: 1 }),
      makeLaneSegment({ lane: 1, start_row: 4, end_row: 5, color_index: 1, group_id: 2 }),
    ];
    viewport.merge_curves = [
      makeMergeCurve({ from_lane: 0, from_row: 0, to_lane: 1, to_row: 2, color_index: 0, group_id: 0, opens_lane: true }),
      makeMergeCurve({ from_lane: 1, from_row: 2, to_lane: 0, to_row: 3, color_index: 1, group_id: 1 }),
      makeMergeCurve({ from_lane: 1, from_row: 4, to_lane: 0, to_row: 6, color_index: 1, group_id: 2 }),
    ];
    viewport.visible_lane_count = 2;
    viewport.total_lane_count = 2;
    return viewport;
  },
};

for (const mode of THEME_MODES) {
  test.describe(`graph — ${mode}`, () => {
    for (const [name, factory] of Object.entries(SCENARIOS)) {
      test(name, async ({ page }) => {
        const viewport = factory();
        await installBootstrapMocks(page, {
          mode,
          activeProject: PROJECT,
          extra: {
            get_graph_viewport: viewport,
            get_branches: [],
          },
        });
        await page.goto("/");
        await applyTheme(page, mode);
        await waitForAppReady(page);
        await clickNav(page, "Graph");
        await waitForGraphPainted(page, viewport.nodes.length);
        await expect(page).toHaveScreenshot(`${mode}-${name}.png`, {
          animations: "disabled",
          // The drawing is the subject here, so it cannot be masked; see
          // `GRAPH_CANVAS_PIXEL_BUDGET` for what that costs.
          maxDiffPixels: GRAPH_CANVAS_PIXEL_BUDGET,
        });
      });
    }
  });
}
