/**
 * Forge — PR detail fetch retry path regression.
 *
 * Spec 3 (Forge Data Fixes), Phase 8.3.
 *
 * Complements the timeout spec (`forge-pr-timeout.spec.ts`) by
 * exercising a *deterministic* error path: the forge provider is
 * configured to fail the first `get_mr_pr` call and succeed on the
 * second. The UI must:
 *
 *   1. Flip to the error banner (ForgeDetailShell's `error` branch).
 *   2. Show a retry button wired to `loadMrPrDetail`.
 *   3. Re-render the content pane after the retry succeeds.
 *
 * Preconditions (future CI): the `MockProvider` in
 * `crates/forge-provider/src/mock.rs` needs a test-only hook — the
 * plan suggests `#[cfg(any(test, feature = "e2e"))]` around a
 * `set_failure_count(n)` method and a companion
 * `#[tauri::command]` wrapper (e.g. `e2e_set_mock_failure_count`)
 * guarded by the same feature flag. Until that shim exists the spec
 * is `describe.skip`-gated so enabling E2E in CI doesn't trip on a
 * missing command.
 *
 * The assertions below model the final behavior so wiring up the
 * harness is the only diff required to run this green.
 */

import { expect } from "@wdio/globals";
import { openFixtureProject } from "../../helpers/project";

/**
 * Invoke the test-only Tauri command that primes the mock provider
 * to fail its next N `get_mr_pr` calls.
 *
 * The command itself is expected to be compiled only when
 * `cargo build --features e2e` is set (see the module-level doc
 * block) — on a production bundle it's absent and `invoke` will
 * reject. This helper swallows that rejection so the suite doesn't
 * dereference an undefined return value; the assertions below then
 * fail loudly if the harness is missing.
 */
async function primeMockFailureCount(n: number): Promise<void> {
  await browser.executeAsync<void, [number]>(
    (count, done) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const internals = (window as any).__TAURI_INTERNALS__;
      if (!internals || typeof internals.invoke !== "function") {
        done();
        return;
      }
      internals
        .invoke("e2e_set_mock_failure_count", { count })
        .then(() => done())
        .catch(() => done());
    },
    n,
  );
}

// eslint-disable-next-line @typescript-eslint/no-unused-expressions
describe.skip("Regression: Forge PR detail retry path", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10_000 });
    await openFixtureProject("simple-repo");
    const nav = await $('[data-testid="nav-merge-requests"]');
    await nav.waitForDisplayed({ timeout: 5_000 });
    await nav.click();
  });

  it("surfaces an error banner on the first failed fetch and retries successfully", async () => {
    // Prime the mock to fail the next get_mr_pr call exactly once.
    await primeMockFailureCount(1);

    // Click a PR — first fetch will throw, so the banner must show.
    const firstRow = await $('[data-testid^="mrpr-list-row-"]').getElement();
    await firstRow.waitForDisplayed({ timeout: 5_000 });
    await firstRow.click();

    const banner = await $('[data-testid="forge-detail-error"]');
    await banner.waitForDisplayed({ timeout: 5_000 });
    await expect(banner).toBeDisplayed();

    // The Retry affordance must be present…
    const retry = await $('[data-testid="forge-detail-retry"]');
    await expect(retry).toBeDisplayed();

    // …clicking it triggers a second fetch, which (with
    // `failure_count` already at 0) succeeds — banner disappears and
    // the detail pane mounts.
    await retry.click();

    await banner.waitForExist({ reverse: true, timeout: 5_000 });

    // Content pane is identified by the `.detail-title` header that
    // `MrPrDetail.svelte` emits for a loaded PR. If the content
    // snippet ever gets wrapped in a dedicated test-id switch this
    // selector should follow.
    const loaded = await $("h3.detail-title");
    await loaded.waitForDisplayed({ timeout: 5_000 });
    await expect(loaded).toBeDisplayed();
  });
});
