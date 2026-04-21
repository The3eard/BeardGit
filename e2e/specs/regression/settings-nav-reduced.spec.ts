/**
 * Spec 4 / Phase 9.1 — Reduced settings nav.
 *
 * Locks in the post-MT-5 Settings IA: the sidebar on the Settings page
 * exposes exactly five categories — General, Git, AI, Integrations,
 * Advanced — and never resurrects the retired "Appearance" or "Editor"
 * (aka Editor / Diff) entries that pre-Spec-4 builds showed.
 *
 * Each category button is rendered by the shared `<CategoryNav>`
 * primitive with `data-testid="bg-cat-nav-<id>"`. We assert against
 * that stable attribute plus the rendered label text (read via
 * `aria-label`-equivalent child span content) so a reshuffled icon
 * map or tooltip copy never silently reintroduces the old entries.
 */
import { expect } from "@wdio/globals";
import sidebar from "../../pages/sidebar.page";

describe("Regression: settings nav reduced IA", () => {
  before(async () => {
    await $("aside.sidebar").waitForExist({ timeout: 10000 });
    await sidebar.navigateTo("settings");
    await $('[data-testid="settings-page"]').waitForDisplayed({
      timeout: 5000,
    });
  });

  it("renders exactly the five post-IA category entries", async () => {
    const items = await $$("[data-testid^='bg-cat-nav-']");
    const count = await items.length;
    expect(count).toBe(5);

    const ids: string[] = [];
    for (let i = 0; i < count; i++) {
      const tid = (await items[i].getAttribute("data-testid")) ?? "";
      ids.push(tid.replace("bg-cat-nav-", ""));
    }
    expect(new Set(ids)).toEqual(
      new Set(["general", "git", "ai", "integrations", "advanced"]),
    );
  });

  it("never shows an 'Appearance' or 'Editor' category entry", async () => {
    const appearance = await $('[data-testid="bg-cat-nav-appearance"]');
    await appearance.waitForExist({ reverse: true, timeout: 1000 });

    const editor = await $('[data-testid="bg-cat-nav-editor"]');
    await editor.waitForExist({ reverse: true, timeout: 1000 });

    // Paranoia-level double check: the visible labels never contain the
    // retired copy either, even under a future i18n swap that would
    // change the slug.
    const labels = await $$(
      "[data-testid^='bg-cat-nav-'] .bg-cat-nav__label",
    );
    const labelCount = await labels.length;
    for (let i = 0; i < labelCount; i++) {
      const text = (await labels[i].getText()).toLowerCase();
      expect(text).not.toContain("appearance");
      expect(text).not.toContain("editor");
      expect(text).not.toContain("diff");
    }
  });
});
