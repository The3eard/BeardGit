/**
 * Forge — empty release detail copy regression.
 *
 * Spec 3 (Forge Data Fixes), Phase 8.2.
 *
 * A release with no body and no assets used to paint a single em-dash
 * in the detail pane — users couldn't tell whether the fetch had
 * failed, the release was genuinely blank, or the UI had silently
 * dropped the payload. Phase 7.2 swaps the em-dash for a localized
 * "No release notes or assets published for {tag}." string whenever
 * `body` is blank and `assets.length === 0`. This spec locks in that
 * contract end-to-end.
 *
 * Preconditions (future CI): `e2e/fixtures/setup.sh` (or a companion
 * script) must create a release tagged `v-empty` with `body: ""` and
 * no uploaded assets. Today the fixture repos are bare git checkouts
 * — there's no wired-up forge that a test run can publish to — so
 * the spec is gated behind `describe.skip` until the mock-forge
 * harness lands. The selectors and assertions are complete so
 * flipping the gate is the only diff.
 */

import { expect } from "@wdio/globals";
import { openFixtureProject } from "../../helpers/project";

const EMPTY_TAG = "v-empty";

// eslint-disable-next-line @typescript-eslint/no-unused-expressions
describe.skip("Regression: Forge release empty detail", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10_000 });
    await openFixtureProject("simple-repo");
    const nav = await $('[data-testid="nav-releases"]');
    await nav.waitForDisplayed({ timeout: 5_000 });
    await nav.click();
  });

  it("renders the empty-state copy when body and assets are both blank", async () => {
    // The release list row uses a per-tag test-id. Click the one the
    // fixture script created with `--notes ""` and no uploaded assets.
    const row = await $(`[data-testid="release-list-row-${EMPTY_TAG}"]`);
    await row.waitForDisplayed({ timeout: 5_000 });
    await row.click();

    // Wait for the detail pane to stop loading.
    const spinner = await $('[data-testid="forge-detail-loading"]');
    await spinner.waitForExist({ reverse: true, timeout: 10_000 });

    // The empty-state copy must be visible verbatim, with the tag
    // interpolated. We accept either language output — the
    // en-US copy is asserted here since the harness defaults to en.
    const body = await $("section.body");
    await body.waitForDisplayed({ timeout: 5_000 });
    const text = await body.getText();
    expect(text).toContain(
      `No release notes or assets published for ${EMPTY_TAG}.`,
    );
  });

  it("keeps the assets table and upload zone visible for empty releases", async () => {
    // Users should still be able to seed a blank release with files,
    // so the asset table header and upload button stay mounted even
    // when the body is empty.
    const uploadBtn = await $$("button").filter(async (b) =>
      /Upload asset/i.test(await b.getText()),
    );
    expect(uploadBtn.length).toBeGreaterThan(0);
  });
});
