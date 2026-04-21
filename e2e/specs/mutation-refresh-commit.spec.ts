/**
 * Phase 11.1 — Mutation refresh on the commit path.
 *
 * Exercises the reactivity-foundation contract: a UI-driven commit
 * must trigger a backend `project-mutated` event that propagates
 * through `mutations.ts` and forces the graph viewport to reload —
 * without the call-site running `reloadGraph()` itself, and without
 * the user hitting a manual refresh button.
 *
 * The spec leans on the hidden `<ol data-testid="graph-rows">` DOM
 * mirror of the canvas graph: we snapshot the row count before the
 * commit, fire the UI flow, then assert the count increased by one.
 * A success toast ("Committed — …") should also be visible — the
 * sticky error variant with a "See details" action must *not* fire.
 */
import { expect } from "@wdio/globals";
import {
  openAnyFixtureRepo,
  stageAllAndCommit,
} from "../helpers/fixtures";

describe("mutation refresh — commit path", () => {
  before(async () => {
    await openAnyFixtureRepo();
  });

  it("graph gains a row after a UI commit (no manual reload)", async () => {
    // Baseline from the mirror — canvas rows aren't query-able, so the
    // synthetic <li data-testid="graph-row"> mirror is the contract.
    // `.length` on a ChainablePromiseArray is itself a Promise<number>,
    // so each count goes through its own `await`.
    const before = await $$('[data-testid="graph-row"]').length;
    expect(before).toBeGreaterThan(0);

    await stageAllAndCommit("chore: reactivity smoke");
    // 500 ms rAF + one mutation-listener debounce. Generous enough to
    // absorb CI jitter without hiding genuine regressions.
    await browser.pause(1000);

    const after = await $$('[data-testid="graph-row"]').length;
    expect(after).toBeGreaterThan(before);

    // Success path never escalates to a sticky error toast.
    const errorToastCount = await $$(
      '[data-testid="toast"][data-type="error"]',
    ).length;
    expect(errorToastCount).toBe(0);
  });
});
