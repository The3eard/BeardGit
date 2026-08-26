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
 * Pixel budget for a screenshot whose *subject* is the commit graph.
 *
 * The canvas sometimes paints in the fallback typeface and stays there.
 * `GitGraph` now asks for its face and redraws when it lands, and this
 * helper waits on `fonts.ready`, on `fonts.check`, and on the bitmap
 * holding still — which took the failure rate from most runs to roughly
 * one full run in ten, but not to zero. The residual is the same shape
 * every time: ~280 differing pixels per rendered row, all of them glyph
 * edges. Lanes, nodes and merge curves never differ.
 *
 * 15,000 covers the 50-row scenario (measured 14,212 on a forced font
 * flip). It is a lot, and it is only spent where the canvas is what the
 * test is looking at — the four `graph.spec` scenarios, whose whole
 * subject is the drawing. Screenshots where the graph is merely *behind*
 * the thing under test mask it instead and keep the suite's 300.
 *
 * This is the largest hole left in the visual suite and it is not
 * closed: a change to the graph's text colours could hide under it. The
 * follow-up is to assert the canvas's palette by sampling pixels, which
 * is deterministic in a way image comparison of antialiased text is not.
 */
export const GRAPH_CANVAS_PIXEL_BUDGET = 15_000;

export async function waitForGraphPainted(page: Page, expectedRows: number): Promise<void> {
  await expect(page.locator('li[data-testid="graph-row"]')).toHaveCount(expectedRows, {
    timeout: 10_000,
  });

  // Before sampling for stability, not after: `GitGraph` asks for its canvas
  // face on mount and redraws when it lands, so a canvas that has held still
  // for 300ms in the fallback typeface is "stable" right up until it isn't.
  // This is a second await of `fonts.ready` — the one in `waitForAppReady`
  // resolved before the graph existed to request anything.
  await page.evaluate(() => document.fonts.ready.then(() => undefined));

  await page.waitForFunction(
    () => {
      const canvas = document.querySelector("canvas");
      if (!canvas) return false;
      // Never accept a canvas that is stable in the *fallback* typeface.
      // `fonts.ready` above resolves against whatever had been requested
      // when it was read, and `GitGraph` requests its face on mount — so
      // asking directly is the only reading that cannot be stale.
      if (!document.fonts.check('12px "Fira Code"')) return false;
      const w = window as unknown as { __graphPaint?: string; __graphStable?: number };
      const shot = canvas.toDataURL();
      if (w.__graphPaint === shot) {
        w.__graphStable = (w.__graphStable ?? 0) + 1;
      } else {
        w.__graphPaint = shot;
        w.__graphStable = 0;
      }
      return (w.__graphStable ?? 0) >= 5;
    },
    undefined,
    { timeout: 15_000, polling: 150 },
  );
}
