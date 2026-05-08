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
        // Give the canvas one frame to paint the new viewport.
        await page.waitForTimeout(150);
        await expect(page).toHaveScreenshot(`${mode}-${name}.png`, {
          animations: "disabled",
        });
      });
    }
  });
}
