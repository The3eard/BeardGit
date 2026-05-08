/**
 * Per-state baselines for the Branches view.
 *
 * Drives `get_branches` for the list states. The detail pane (commits
 * inside a branch) is fed by `get_branch_commits`.
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
  makeBranchInfo,
  makeBranchList,
  makeCommitInfo,
  makeProjectInfo,
} from "../../../src/test/fixtures";

const PROJECT = makeProjectInfo();

for (const mode of THEME_MODES) {
  test.describe(`branches — ${mode}`, () => {
    test("empty", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        extra: { get_branches: [] },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Branches");
      await expect(page).toHaveScreenshot(`${mode}-empty.png`, {
        animations: "disabled",
      });
    });

    test("populated (HEAD + ahead/behind + remote)", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        extra: { get_branches: makeBranchList() },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Branches");
      await expect(page).toHaveScreenshot(`${mode}-populated.png`, {
        animations: "disabled",
      });
    });

    test("many branches (grouped by prefix)", async ({ page }) => {
      const branches = [
        makeBranchInfo({ name: "main", is_head: false, ahead: 0, behind: 0, upstream: "origin/main" }),
        makeBranchInfo({ name: "feat/visual-tests", is_head: true, ahead: 3, behind: 0, upstream: "origin/feat/visual-tests" }),
        makeBranchInfo({ name: "feat/sidebar-search" }),
        makeBranchInfo({ name: "feat/keyboard-shortcuts" }),
        makeBranchInfo({ name: "fix/graph-flicker" }),
        makeBranchInfo({ name: "fix/diff-large-files" }),
        makeBranchInfo({ name: "chore/bump-deps" }),
        makeBranchInfo({ name: "docs/contributing" }),
        makeBranchInfo({ name: "origin/main", is_remote: true, upstream: null }),
        makeBranchInfo({ name: "origin/feat/legacy", is_remote: true, upstream: null }),
      ];
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        extra: { get_branches: branches },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Branches");
      await expect(page).toHaveScreenshot(`${mode}-many.png`, {
        animations: "disabled",
      });
    });

    test("branch selected (commit list expanded)", async ({ page }) => {
      const branches = makeBranchList();
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        extra: {
          get_branches: branches,
          get_branch_commits: [
            makeCommitInfo({ oid: "0".repeat(40), summary: "feat(graph): add lane recycling" }),
            makeCommitInfo({ oid: "1".repeat(40), summary: "fix: handle empty diff payloads" }),
            makeCommitInfo({ oid: "2".repeat(40), summary: "chore: bump dependencies" }),
          ],
        },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Branches");
      // Click the first branch leaf (not a folder/group). The tree
      // renders branches as `.tree-leaf` inside `BranchTreeNode.svelte`;
      // folders are `.tree-folder` and would just toggle expansion.
      await page.locator(".tree-leaf").first().click();
      await page.waitForTimeout(300);
      await expect(page).toHaveScreenshot(`${mode}-selected.png`, {
        animations: "disabled",
      });
    });
  });
}
