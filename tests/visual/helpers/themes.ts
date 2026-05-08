/**
 * Force a fixed theme mode for visual tests.
 *
 * The app reads `data-forced-theme-mode` on `<html>` to override OS
 * theme detection. We block on `--overlay-accent-blue` being non-empty
 * because it's written in the same `applyTheme()` pass that sets every
 * other token — it's a reliable "tokens are now on the document" gate
 * and avoids screenshotting an unstyled flash on slow runs.
 */

import type { Page } from "@playwright/test";

export type ThemeMode = "dark" | "light";
export const THEME_MODES: readonly ThemeMode[] = ["dark", "light"] as const;

export async function applyTheme(page: Page, mode: ThemeMode): Promise<void> {
  await page.evaluate((m) => {
    document.documentElement.setAttribute("data-forced-theme-mode", m);
  }, mode);
  await page.waitForFunction(
    () =>
      !!getComputedStyle(document.documentElement)
        .getPropertyValue("--overlay-accent-blue")
        .trim(),
  );
}
