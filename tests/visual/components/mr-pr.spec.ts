/**
 * Per-state baselines for the Pull / Merge Requests view.
 *
 * The list scenarios drive `list_mr_prs`; the detail scenario also
 * drives `get_mr_pr_detail` (requested when a row is clicked) and
 * `get_mr_pr_diff` for the file list.
 */

import { expect, test } from "@playwright/test";

import {
  applyTheme,
  clickNav,
  installBootstrapMocks,
  THEME_MODES,
  waitForAppReady,
} from "../helpers";
import {
  makeMrPr,
  makeMrPrDetail,
  makeMrPrDiffFile,
  makeMrPrList,
  makeProjectInfo,
} from "../../../src/test/fixtures";

const PROJECT = makeProjectInfo();

for (const mode of THEME_MODES) {
  test.describe(`mr-pr — ${mode}`, () => {
    test("empty list", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        forge: "github",
        extra: { list_mr_prs: [] },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Pull Requests");
      await expect(page).toHaveScreenshot(`${mode}-empty.png`, {
        animations: "disabled",
      });
    });

    test("populated list (open + draft + merged + closed)", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        forge: "github",
        extra: { list_mr_prs: makeMrPrList() },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Pull Requests");
      await expect(page).toHaveScreenshot(`${mode}-populated.png`, {
        animations: "disabled",
      });
    });

    test("detail with comments", async ({ page }) => {
      const list = makeMrPrList();
      const targetNumber = list[0].number;

      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        forge: "github",
        extra: {
          list_mr_prs: list,
          get_mr_pr_detail: makeMrPrDetail({
            summary: makeMrPr({ number: targetNumber, title: list[0].title }),
          }),
          get_mr_pr_diff: [
            makeMrPrDiffFile({
              path: "tests/visual/helpers/mock-ipc.ts",
              status: "A",
              additions: 142,
              deletions: 0,
            }),
            makeMrPrDiffFile({
              path: "src/test/fixtures/index.ts",
              status: "M",
              additions: 8,
              deletions: 0,
            }),
            makeMrPrDiffFile({
              path: "tests/visual/routes.spec.ts",
              status: "M",
              additions: 56,
              deletions: 28,
            }),
          ],
        },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Pull Requests");

      // Click the first row in the generic List component to open the
      // detail pane. `.list-row` is the wrapper from common/List.svelte.
      await page.locator(".list-row").first().click();
      // Brief wait for the detail fetch + paint.
      await page.waitForTimeout(300);
      await expect(page).toHaveScreenshot(`${mode}-detail.png`, {
        animations: "disabled",
      });
    });
  });
}
