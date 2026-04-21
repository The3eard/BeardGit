/**
 * Phase 11.2 — Mutation refresh on the external-CLI path.
 *
 * Proves the Rust-side file watcher (`crates/watcher`) still catches
 * mutations that didn't originate in the app — e.g. the user runs
 * `git commit` in a shell tab while BeardGit is open — and forwards
 * them via the same `project-mutated` event the UI mutations use.
 *
 * Contract:
 *  1. Graph row count increases within the watcher debounce window
 *     (~500 ms). The spec waits 900 ms to absorb OS fs-event latency.
 *  2. *No toast fires* — external mutations are silent refreshes.
 *     Toasts are UX-initiated feedback; watcher-driven refreshes are
 *     backend housekeeping and must not spam the toast rail.
 */
import { expect } from "@wdio/globals";
import { execSync } from "node:child_process";
import {
  openAnyFixtureRepo,
  fixtureRepoPath,
} from "../helpers/fixtures";

describe("mutation refresh — external git CLI", () => {
  before(async () => {
    await openAnyFixtureRepo();
  });

  it("watcher-driven refresh adds a row and stays silent", async () => {
    const repoPath = await fixtureRepoPath();
    const before = await $$('[data-testid="graph-row"]').length;
    expect(before).toBeGreaterThan(0);

    // `--allow-empty` so we don't depend on worktree state.
    execSync(
      `git -C '${repoPath}' commit --allow-empty -m 'external-cli-smoke'`,
      { stdio: "ignore" },
    );

    // 500 ms watcher debounce + rAF. Extra headroom for slow CI disks.
    await browser.pause(1500);

    const after = await $$('[data-testid="graph-row"]').length;
    expect(after).toBeGreaterThan(before);

    // External mutations never raise a toast — that's the whole point
    // of the silent-refresh policy.
    const toastCount = await $$('[data-testid="toast"]').length;
    expect(toastCount).toBe(0);
  });
});
