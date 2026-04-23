import { test, expect } from "@playwright/test";

const VIEWS: Array<[id: string, sidebarLabel: string]> = [
  ["graph", "Graph"],
  ["changes", "Changes"],
  ["branches", "Branches"],
  ["tags", "Tags"],
  ["stashes", "Stashes"],
  ["worktrees", "Worktrees"],
  ["reflog", "Reflog"],
  ["bisect", "Bisect"],
  ["submodules", "Submodules"],
  ["pipelines", "Pipelines"],
  ["issues", "Issues"],
  ["mr-pr", "Merge/Pull Requests"],
  ["releases", "Releases"],
  ["ai-sessions", "AI Sessions"],
  ["repo-config", "Repo settings"],
  ["settings", "Settings"],
];

for (const mode of ["dark", "light"] as const) {
  test.describe(`visual baseline — ${mode}`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto("/");
      await page.evaluate((m) => {
        document.documentElement.setAttribute("data-forced-theme-mode", m);
      }, mode);
      // Wait for applyTheme to have written tokens.
      await page.waitForFunction(
        () => !!getComputedStyle(document.documentElement)
          .getPropertyValue("--overlay-accent-blue").trim(),
      );
    });

    for (const [id, label] of VIEWS) {
      test(`${id}`, async ({ page }) => {
        await page.getByRole("navigation").getByLabel(label).click();
        await expect(page).toHaveScreenshot(`${mode}-${id}.png`, {
          fullPage: false,
          animations: "disabled",
        });
      });
    }
  });
}
