/**
 * Spec 4 / Phase 9.1 — Integrations layout.
 *
 * Locks in the post-Phase-7/8 Integrations page shape:
 *
 *   1. A compact "How to connect" control sits at the very top of the
 *      category body — rendered by `<ConnectionHowTo>` with
 *      `data-testid="integrations-howto"`.
 *   2. Below the howto lives exactly ONE `.bg-card`: the unified
 *      "Connections" card (`<ConnectionRow>` ×4). The old split of a
 *      separate PAT card + CLI card is gone.
 *   3. Inside that single card, all four expected rows render:
 *      github, gitlab, gh, glab — each exposing
 *      `data-testid="integrations-row-<kind>"` from the shared
 *      `<ConnectionRow>` primitive.
 */
import { expect } from "@wdio/globals";
import sidebar from "../../pages/sidebar.page";

describe("Regression: integrations page layout", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
    await sidebar.navigateTo("settings");
    await $('[data-testid="settings-page"]').waitForDisplayed({
      timeout: 5000,
    });

    const integrationsBtn = await $('[data-testid="bg-cat-nav-integrations"]');
    await integrationsBtn.waitForClickable({ timeout: 3000 });
    await integrationsBtn.click();
    // Give onMount (provider status + CLI check) a moment; the card
    // mounts synchronously but row subscriptions kick in async.
    await browser.pause(400);
  });

  it("renders the howto dropdown above the connections card", async () => {
    const content = await $('[data-testid="settings-content"]');
    const howto = await content.$('[data-testid="integrations-howto"]');
    await howto.waitForExist({ timeout: 3000 });

    const firstCard = await content.$(".bg-card");
    await firstCard.waitForExist({ timeout: 3000 });

    // Positional assertion: the howto must appear before the first
    // `.bg-card` in document order. WebDriver lacks a "document
    // position" API so we compare bounding rects — the howto's top
    // must sit above (lower y) the card's top.
    const howtoTop = (await howto.getLocation("y")) as number;
    const cardTop = (await firstCard.getLocation("y")) as number;
    expect(howtoTop).toBeLessThan(cardTop);
  });

  it("renders exactly one Connections card under the howto", async () => {
    const content = await $('[data-testid="settings-content"]');
    const cardCount = await content.$$(".bg-card").length;
    expect(cardCount).toBe(1);
  });

  it("renders all four connection rows inside the card", async () => {
    const content = await $('[data-testid="settings-content"]');
    const card = await content.$(".bg-card");

    for (const kind of ["github", "gitlab", "gh", "glab"]) {
      const row = await card.$(`[data-testid="integrations-row-${kind}"]`);
      await row.waitForExist({ timeout: 3000 });
      expect(await row.isExisting()).toBe(true);
    }
  });
});
