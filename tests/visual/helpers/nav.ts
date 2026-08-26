/**
 * Sidebar navigation for the visual suite — click, then wait for the view
 * to actually be there.
 *
 * ## Why the wait lives here
 *
 * Half the views in `+page.svelte` are wrapped in `LazyComponent`, which
 * dynamic-imports the panel's chunk. Until the import resolves it renders
 * `.lazy-placeholder` — a spinner on an otherwise empty main area. Click
 * and screenshot, and that spinner is what you get.
 *
 * That would be a loud failure if it only ever broke a comparison. It is
 * not: `toHaveScreenshot` polls against the baseline on disk, so a
 * baseline *recorded* mid-load matches on an early poll and passes
 * forever, while a correctly recorded one fails visibly. Getting it wrong
 * is silent and permanent; getting it right is noisy. Three baselines
 * (`dark-pipelines`, `dark-settings`, `dark-ai-sessions`) were recorded
 * that way and Settings — one of the largest surfaces in the app — had no
 * coverage that could ever fail.
 *
 * So the wait belongs on the shared route helper rather than in whichever
 * spec happened to notice, and it has to be a *positive* signal first:
 * waiting only for the placeholder to detach passes instantly when it has
 * not been attached yet, which is the same bug one tick earlier.
 */

import { expect, type Page } from "@playwright/test";

function navItem(page: Page, label: string) {
  return page.locator(`button.nav-item:has(.nav-label:text-is("${label}"))`).first();
}

/**
 * Click a sidebar nav item by visible label and wait for its view.
 *
 * `Settings` is rendered outside `<nav>` in `Sidebar.svelte` but uses the
 * same `.nav-item` class, so a label-based locator hits every sidebar
 * entry uniformly without needing to know which container the item lives
 * in.
 */
export async function clickNav(page: Page, label: string): Promise<void> {
  await navItem(page, label).click();

  // 1. Positive edge: the click's reactive pass has landed. `LazyComponent`
  //    renders its placeholder in that same pass, so from here on "no
  //    placeholder" means "loaded", not "not started yet".
  await expect(navItem(page, label)).toHaveClass(/\bactive\b/, { timeout: 10_000 });

  // 2. The chunk has arrived. Views that are not lazy never attach one and
  //    fall through immediately.
  await page
    .locator(".lazy-placeholder")
    .waitFor({ state: "detached", timeout: 15_000 });

  // 3. A failed chunk renders "Failed to load panel." — which is a
  //    perfectly stable thing to screenshot, and exactly the kind of wrong
  //    baseline this helper exists to prevent.
  await expect(page.locator(".lazy-error")).toHaveCount(0);
}
