/**
 * Per-state baselines for the Issues view (provider-gated).
 *
 * Drives `list_issues` for the list states and `get_issue` for the
 * detail pane.
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
  makeIssue,
  makeIssueDetail,
  makeIssueList,
  makeProjectInfo,
} from "../../../src/test/fixtures";

const PROJECT = makeProjectInfo();

for (const mode of THEME_MODES) {
  test.describe(`issues — ${mode}`, () => {
    test("empty list", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        forge: "github",
        extra: { list_issues: [] },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Issues");
      await expect(page).toHaveScreenshot(`${mode}-empty.png`, {
        animations: "disabled",
      });
    });

    test("populated list (open + closed)", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        forge: "github",
        extra: { list_issues: makeIssueList() },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Issues");
      await expect(page).toHaveScreenshot(`${mode}-populated.png`, {
        animations: "disabled",
      });
    });

    test("detail with comments and labels", async ({ page }) => {
      const list = makeIssueList();
      const target = list[0];

      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        forge: "github",
        extra: {
          list_issues: list,
          get_issue: makeIssueDetail({
            summary: makeIssue({ number: target.number, title: target.title }),
          }),
        },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Issues");
      await page.locator(".list-row").first().click();
      await page.waitForTimeout(300);
      await expect(page).toHaveScreenshot(`${mode}-detail.png`, {
        animations: "disabled",
      });
    });
  });
}
