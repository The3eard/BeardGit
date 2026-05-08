/**
 * Click a sidebar nav item by visible label.
 *
 * `Settings` is rendered outside `<nav>` in `Sidebar.svelte` but uses
 * the same `.nav-item` class, so a label-based locator hits every
 * sidebar entry uniformly without needing to know which container the
 * item lives in.
 */

import type { Page } from "@playwright/test";

export async function clickNav(page: Page, label: string): Promise<void> {
  await page
    .locator(`button.nav-item:has(.nav-label:text-is("${label}"))`)
    .first()
    .click();
}
