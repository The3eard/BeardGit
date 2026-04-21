/**
 * Forge — PR detail timeout banner regression.
 *
 * Spec 3 (Forge Data Fixes), Phase 8.1.
 *
 * Exercises the "infinite spinner on slow `gh api pulls/N/files
 * --paginate`" bug that motivated Phase 1 of the plan. The TypeScript
 * side wraps the detail+diff fetch in `withTimeout(DETAIL_TIMEOUT_MS)`
 * (see `src/lib/stores/mr-pr.ts`, ~line 150). When the timeout fires
 * the store flips `mrPrDetailError` to a localized timeout string and
 * `ForgeDetailShell` renders the error banner + retry button instead
 * of leaving the spinner up forever.
 *
 * Preconditions (future CI): the E2E image needs to expose a mocked
 * forge provider whose `get_mr_pr_diff` can be configured to never
 * settle. The existing `MockProvider` in `crates/forge-provider`
 * returns immediately — a slow variant or a `#[cfg(feature = "e2e")]`
 * "stall" hook has to be added before this spec can run unattended.
 *
 * Until that harness exists this suite is `describe.skip`-gated so
 * `npm run e2e` stays green when it's accidentally re-enabled. The
 * assertions are still written in full so flipping the gate and
 * adding the harness is the entire diff.
 */

import { expect } from "@wdio/globals";
import { openFixtureProject } from "../../helpers/project";

// The shared `DETAIL_TIMEOUT_MS` in `src/lib/stores/mr-pr.ts` is 15 s;
// allow a 3 s grace window for slow CI before failing the assertion.
const TIMEOUT_MS = 15_000;
const SLACK_MS = 3_000;

// eslint-disable-next-line @typescript-eslint/no-unused-expressions
describe.skip("Regression: Forge PR detail timeout", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10_000 });
    await openFixtureProject("simple-repo");
    // Navigate to the MR/PR list. The label switches between "Pull
    // requests" and "Merge requests" but the route id is stable
    // (see src/lib/components/layout/Sidebar.svelte).
    const nav = await $('[data-testid="nav-merge-requests"]');
    await nav.waitForDisplayed({ timeout: 5_000 });
    await nav.click();
  });

  it("shows the error banner after DETAIL_TIMEOUT_MS when the diff never resolves", async () => {
    // Click the first PR entry — the list is fed by the mock provider
    // and the mock is configured (out of band, see header comment) to
    // stall `get_mr_pr_diff` indefinitely.
    const firstRow = await $('[data-testid^="mrpr-list-row-"]').getElement();
    await firstRow.waitForDisplayed({ timeout: 5_000 });
    await firstRow.click();

    // Loading spinner must appear first…
    const spinner = await $('[data-testid="forge-detail-loading"]');
    await spinner.waitForDisplayed({ timeout: 2_000 });

    // …and then be replaced by the error banner no later than the
    // timeout + slack window.
    const banner = await $('[data-testid="forge-detail-error"]');
    await banner.waitForDisplayed({ timeout: TIMEOUT_MS + SLACK_MS });
    await expect(banner).toBeDisplayed();

    // Retry button is expected to exist so the user can recover.
    const retry = await $('[data-testid="forge-detail-retry"]');
    await expect(retry).toBeDisplayed();
  });

  it("clicking Retry re-invokes the fetch", async () => {
    // The mock provider is reconfigured to succeed on the second call,
    // so clicking Retry should trigger a new loading spinner and then
    // the content pane (not the banner).
    const retry = await $('[data-testid="forge-detail-retry"]');
    await retry.click();

    const spinner = await $('[data-testid="forge-detail-loading"]');
    await spinner.waitForDisplayed({ timeout: 2_000 });

    // After the second (successful) fetch the banner must be gone.
    const banner = await $('[data-testid="forge-detail-error"]');
    await banner.waitForExist({ reverse: true, timeout: 5_000 });
  });
});
