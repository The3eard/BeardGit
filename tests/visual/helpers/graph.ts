/**
 * Wait for the commit graph to have painted a specific viewport.
 *
 * The graph draws to a `<canvas>`, so nothing about its contents is
 * visible to a normal locator and the specs used to bridge the gap with
 * `waitForTimeout(150)`. That is a race, and it lost often enough to
 * matter: the canvas region differed by ~10,800px between runs of the
 * same test, which `maxDiffPixelRatio: 0.01` (12,960px) silently absorbed.
 *
 * Two signals, both needed:
 *
 * 1. `GitGraph.svelte` renders one `li[data-testid="graph-row"]` per node
 *    alongside the canvas, for accessibility. That count reaching the
 *    expected value means the viewport data is in and Svelte has flushed.
 * 2. The canvas's own pixels, sampled until two consecutive frames come
 *    back identical. A fixed number of animation frames is not enough:
 *    the graph repaints more than once as fonts resolve and the layout
 *    settles, and which repaint a screenshot lands between varied run to
 *    run — thousands of differing pixels along the glyph edges of every
 *    row. Reading the bitmap asks the only question that matters, which
 *    is whether it has stopped changing.
 */

import { expect, type Page } from "@playwright/test";

/**
 * Pixel budget for a screenshot containing the commit graph.
 *
 * The canvas does not settle to the same bitmap twice. `waitForGraphPainted`
 * polls until three consecutive reads of `toDataURL()` agree, so this is not
 * a race that a longer wait fixes — the *stable* state differs run to run,
 * by up to ~7,600px along the antialiased edges of the row text. Every
 * differing pixel measured has been a glyph edge; the lanes, nodes and
 * merge curves are stable.
 *
 * This is real blindness and it is worth naming: a change to the graph
 * smaller than this will not be caught here. It buys the rest of the suite
 * a 300px budget instead of the 12,960 a global ratio would need.
 */
export const GRAPH_CANVAS_PIXEL_BUDGET = 9_000;

export async function waitForGraphPainted(page: Page, expectedRows: number): Promise<void> {
  await expect(page.locator('li[data-testid="graph-row"]')).toHaveCount(expectedRows, {
    timeout: 10_000,
  });

  await page.waitForFunction(
    () => {
      const canvas = document.querySelector("canvas");
      if (!canvas) return false;
      const w = window as unknown as { __graphPaint?: string; __graphStable?: number };
      const shot = canvas.toDataURL();
      if (w.__graphPaint === shot) {
        w.__graphStable = (w.__graphStable ?? 0) + 1;
      } else {
        w.__graphPaint = shot;
        w.__graphStable = 0;
      }
      return (w.__graphStable ?? 0) >= 3;
    },
    undefined,
    { timeout: 10_000, polling: 100 },
  );
}
