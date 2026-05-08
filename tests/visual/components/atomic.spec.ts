/**
 * Atomic-component baselines that aren't naturally exposed by any
 * single view's flow.
 *
 * Approach: use the Vite dev-server's ES-module endpoints (e.g.
 * `/src/lib/stores/toast.ts`) to dispatch into the live app's stores
 * from `page.evaluate`. This avoids spinning up an isolated harness
 * route while still letting us drive states that real interactions
 * don't trigger directly.
 *
 * EmptyState and SearchBar are already covered by `mr-pr.spec.ts`,
 * `issues.spec.ts`, and `branches.spec.ts` (empty / populated rows
 * and the search-bar row above the list), so they aren't repeated
 * here — those specs already exercise their visible states.
 */

import { expect, test } from "@playwright/test";

import {
  applyTheme,
  installBootstrapMocks,
  THEME_MODES,
  waitForAppReady,
} from "../helpers";
import { makeProjectInfo } from "../../../src/test/fixtures";

const PROJECT = makeProjectInfo();

interface ToastSpec {
  type: "success" | "error" | "warning" | "info";
  message: string;
  duration: number | null;
}

async function addToasts(page: import("@playwright/test").Page, specs: ToastSpec[]): Promise<void> {
  await page.evaluate(async (toasts) => {
    // The dev server serves source files as ES modules; resolving the
    // absolute path bypasses TypeScript's module-resolution check (the
    // suppressed comment is the cleanest way to let `tsc --noEmit` pass).
    // @ts-expect-error — Vite-served URL, not resolved by tsc
    const mod = await import("/src/lib/stores/toast.ts");
    for (const t of toasts) (mod as { addToast: (s: unknown) => void }).addToast(t);
  }, specs);
}

for (const mode of THEME_MODES) {
  test.describe(`atomic — ${mode}`, () => {
    test("toast: success", async ({ page }) => {
      await installBootstrapMocks(page, { mode, activeProject: PROJECT });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);

      await addToasts(page, [
        { type: "success", message: "Pushed 3 commits to origin/feat/visual-tests", duration: null },
      ]);
      await page.waitForTimeout(150);
      await expect(page).toHaveScreenshot(`${mode}-toast-success.png`, {
        animations: "disabled",
      });
    });

    test("toast: error sticky", async ({ page }) => {
      await installBootstrapMocks(page, { mode, activeProject: PROJECT });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);

      await addToasts(page, [
        {
          type: "error",
          message: "Push failed — remote rejected non-fast-forward update on refs/heads/main",
          duration: null,
        },
      ]);
      await page.waitForTimeout(150);
      await expect(page).toHaveScreenshot(`${mode}-toast-error.png`, {
        animations: "disabled",
      });
    });

    test("toast: stack of three", async ({ page }) => {
      await installBootstrapMocks(page, { mode, activeProject: PROJECT });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);

      await addToasts(page, [
        { type: "success", message: "Staged 4 files", duration: null },
        { type: "warning", message: "Local changes ahead of upstream by 3 commits", duration: null },
        { type: "info", message: "Background fetch completed", duration: null },
      ]);
      await page.waitForTimeout(150);
      await expect(page).toHaveScreenshot(`${mode}-toast-stack.png`, {
        animations: "disabled",
      });
    });
  });
}
