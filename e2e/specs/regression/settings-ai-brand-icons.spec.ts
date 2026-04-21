/**
 * Spec 4 / Phase 9.1 — AI settings brand icons.
 *
 * The provider-row in AI Settings must show the vendor's brand glyph
 * (Anthropic / OpenAI / OpenCode SVGs) via the shared `<ProviderIcon>`
 * component — never the old `<span class="provider-icon nf">` nerd-font
 * fallback. Spec 2 landed the `<ProviderIcon>` component; Spec 4
 * Phase 5 re-verifies it and adds a testid prefix (`provider-icon-*`)
 * so this end-to-end check can assert without relying on the
 * implementation-specific `img.provider-icon` class selector.
 *
 * Assertions:
 *   1. Exactly 3 `[data-testid^="provider-icon-"]` elements render —
 *      one per row in the ALL_KINDS triple (claude_code / codex /
 *      open_code).
 *   2. No legacy `span.provider-icon.nf` nerd-font span survives —
 *      the old regression-prone markup must stay buried.
 */
import { expect } from "@wdio/globals";
import sidebar from "../../pages/sidebar.page";

describe("Regression: AI settings brand icons", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
    await sidebar.navigateTo("settings");
    await $('[data-testid="settings-page"]').waitForDisplayed({
      timeout: 5000,
    });

    const aiBtn = await $('[data-testid="bg-cat-nav-ai"]');
    await aiBtn.waitForClickable({ timeout: 3000 });
    await aiBtn.click();
    // Provider detection runs in onMount and populates the rows;
    // the component still renders all 3 ALL_KINDS rows even when no
    // providers are detected, so the icons should appear immediately.
    await browser.pause(400);
  });

  it("renders three ProviderIcon elements via the shared component", async () => {
    const content = await $('[data-testid="settings-content"]');
    const icons = await content.$$('[data-testid^="provider-icon-"]');
    const count = await icons.length;
    expect(count).toBe(3);

    // Sanity check: the three expected provider kinds are represented.
    const kinds: string[] = [];
    for (let i = 0; i < count; i++) {
      const tid = (await icons[i].getAttribute("data-testid")) ?? "";
      kinds.push(tid.replace("provider-icon-", ""));
    }
    expect(new Set(kinds)).toEqual(
      new Set(["claude_code", "codex", "open_code"]),
    );
  });

  it("never resurrects the legacy `.provider-icon.nf` nerd-font span", async () => {
    const content = await $('[data-testid="settings-content"]');
    const legacyCount = await content.$$("span.provider-icon.nf").length;
    expect(legacyCount).toBe(0);
  });
});
