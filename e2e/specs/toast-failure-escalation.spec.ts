/**
 * Phase 11.3 — Failure escalation via the toast "See details" action.
 *
 * When a mutation wrapped in `runMutation` rejects, the façade fires a
 * sticky (non-auto-dismiss) error toast with a single "See details"
 * action. Clicking that action must open the Tasks popover *already
 * scrolled to the failed task's detail view* — not the generic list —
 * so the user lands on the error output in one click.
 *
 * Setup: the canonical `simple-repo` fixture has no `origin`. We point
 * `origin` at an unreachable URL via `git remote add` (spawned from
 * the spec process — no frontend command to add remotes exists), then
 * drive the Toolbar's push button through the UI. `push_remote` on the
 * Rust side rejects, `runMutation` escalates, and the popover pops up
 * on the detail pane of the ad-hoc error task it synthesised.
 */
import { expect } from "@wdio/globals";
import { execSync } from "node:child_process";
import {
  openAnyFixtureRepo,
  fixtureRepoPath,
} from "../helpers/fixtures";

describe("toast failure escalation", () => {
  before(async () => {
    await openAnyFixtureRepo();
    const repoPath = await fixtureRepoPath();
    // Idempotent: clear any origin left over from a previous spec run,
    // then re-add pointing at a guaranteed-unreachable URL. RFC 6761
    // reserves `.invalid` so this never leaks onto the network.
    try {
      execSync(`git -C '${repoPath}' remote remove origin`, {
        stdio: "ignore",
      });
    } catch {
      // remote may not exist yet — safe to ignore
    }
    execSync(
      `git -C '${repoPath}' remote add origin https://example.invalid/x.git`,
      { stdio: "ignore" },
    );
    // Give the UI a beat to notice the new remote (watcher refresh).
    await browser.pause(500);
  });

  after(async () => {
    // Leave the fixture as we found it so later specs aren't poisoned.
    const repoPath = await fixtureRepoPath();
    try {
      execSync(`git -C '${repoPath}' remote remove origin`, {
        stdio: "ignore",
      });
    } catch {
      // already gone — fine
    }
  });

  it("push failure → sticky error toast → See details → tasks detail", async () => {
    const push = await $('[data-testid="push-button"]');
    await push.waitForDisplayed({ timeout: 5000 });
    await push.waitForEnabled({ timeout: 5000 });
    await push.click();

    // Push reaches DNS resolution then fails — 6 s is enough on CI.
    const toast = await $('[data-testid="toast"][data-type="error"]');
    await toast.waitForDisplayed({ timeout: 10000 });

    const seeDetails = await $('[data-testid="toast-action-see-details"]');
    await seeDetails.waitForDisplayed({ timeout: 2000 });
    await seeDetails.click();

    const popover = await $('[data-testid="tasks-popover"]');
    await popover.waitForDisplayed({ timeout: 2000 });
    // The popover opens directly on a detail view — header testid is
    // only present in detail mode, not list mode.
    const detailHeader = await $(
      '[data-testid="tasks-popover-detail-header"]',
    );
    await detailHeader.waitForDisplayed({ timeout: 2000 });

    // The detail panel carries the failure output.
    const errorNode = await $('[data-testid="task-detail-error"]');
    await expect(errorNode).toBeDisplayed();
  });
});
