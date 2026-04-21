/**
 * Spec 4 / Phase 9.1 — Theme lives in General.
 *
 * Before MT-5 the theme picker had its own "Appearance" category. The
 * IA overhaul folded it into General so the five-entry sidebar stays
 * reduced. This spec asserts:
 *
 *   1. There is no `bg-cat-nav-appearance` entry to click. (Phase 1
 *      eliminated it; a regression here would mean the category list
 *      was silently re-added.)
 *   2. Opening General exposes the `theme` setting anchor in the
 *      content body — i.e. `<LookAndFeelSection>` rendered its
 *      `data-setting-anchor="theme"` wrapper inside the General
 *      content pane, not in a side panel or popover.
 */
import { expect } from "@wdio/globals";
import sidebar from "../../pages/sidebar.page";

describe("Regression: theme picker lives in General", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
    await sidebar.navigateTo("settings");
    await $('[data-testid="settings-page"]').waitForDisplayed({
      timeout: 5000,
    });
  });

  it("does not expose an 'appearance' sidebar entry", async () => {
    const appearance = await $('[data-testid="bg-cat-nav-appearance"]');
    await appearance.waitForExist({ reverse: true, timeout: 1000 });
  });

  it("renders the theme anchor inside the General content body", async () => {
    // Click the General entry defensively — the page boots with it
    // selected, but other specs in the run may have left a different
    // category active.
    const generalBtn = await $('[data-testid="bg-cat-nav-general"]');
    await generalBtn.waitForClickable({ timeout: 3000 });
    await generalBtn.click();
    await browser.pause(200);

    // The theme wrapper is inside LookAndFeelSection, which General
    // embeds. We scope the query to `settings-content` so a stray
    // anchor elsewhere (e.g. in the search dropdown's preview code)
    // can't pass the assertion.
    const content = await $('[data-testid="settings-content"]');
    const themeAnchor = await content.$(
      '[data-setting-anchor="theme"]',
    );
    await themeAnchor.waitForExist({ timeout: 3000 });
    expect(await themeAnchor.isExisting()).toBe(true);
  });
});
