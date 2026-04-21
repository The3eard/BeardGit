/**
 * Phase 11.4 — Graph cold-start paints from cache.
 *
 * After the Phase-8 slice landed, opening a project whose
 * `ProjectSnapshot.graph_viewport_cache` is fresh should paint the
 * graph synchronously — no skeleton, no flash of empty canvas. The
 * background refresh reconciles against the cached `top_oid` and
 * only repaints if HEAD has advanced.
 *
 * Tauri-driver doesn't support reboot mid-session, so this spec
 * simulates "cold start" by closing every open project tab and
 * re-opening the fixture. That's good enough to exercise the
 * cache-rehydration path — the in-memory `viewport` store is `null`
 * at tab-open, and the disk snapshot populates it synchronously
 * before the canvas mounts.
 *
 * Assertions:
 *  1. At least one row exists immediately after re-open — the cache
 *     hydrated into the mirror before the fresh viewport fetch
 *     returned.
 *  2. The skeleton overlay (`data-testid="graph-skeleton"`) never
 *     appears while the viewport is hydrated from cache; it only
 *     renders when `$viewport === null`.
 */
import { expect } from "@wdio/globals";
import { openAnyFixtureRepo, restartApp } from "../helpers/fixtures";

describe("graph cold start cache", () => {
  it("paints from cache after close+reopen (no skeleton)", async () => {
    // First open: populates the in-memory + on-disk viewport cache.
    await openAnyFixtureRepo();
    // Wait long enough for the fresh viewport fetch + snapshot save.
    await browser.pause(1500);
    const firstCount = await $$('[data-testid="graph-row"]').length;
    expect(firstCount).toBeGreaterThan(0);

    // Tear everything down, then reopen the same fixture — simulates
    // the project tab lifecycle that rehydrates from the persisted
    // cache.
    await restartApp();
    await openAnyFixtureRepo();

    // Rows must be visible *immediately* (within one tick) — the
    // cache path is synchronous. We check on the very next frame
    // rather than waiting a full second, so a regression that makes
    // rehydration async would surface as a failure.
    await browser.pause(50);
    const rowCount = await $$('[data-testid="graph-row"]').length;
    expect(rowCount).toBeGreaterThan(0);

    // The skeleton only renders when `$viewport === null`. A warm
    // cache means it should never have been displayed after reopen.
    const skeleton = await $('[data-testid="graph-skeleton"]');
    const skeletonExists = await skeleton.isExisting();
    if (skeletonExists) {
      const visible = await skeleton.isDisplayed();
      expect(visible).toBe(false);
    }
  });
});
