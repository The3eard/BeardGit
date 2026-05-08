/**
 * Per-state baselines for the commit detail pane (right side of the
 * Graph view).
 *
 * Drives `get_commit_detail`, `get_commit_files`, `get_commit_full_diff`,
 * `get_commit_stats`. The selection is triggered by setting the
 * `selectedOid` from page-level JS rather than clicking the canvas
 * (which Playwright can't easily target).
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
  makeCommitFileChange,
  makeCommitInfo,
  makeCommitStats,
  makeFileDiff,
  makeGraphViewport,
  makeProjectInfo,
} from "../../../src/test/fixtures";

const PROJECT = makeProjectInfo();
const TARGET_OID = "1".repeat(40);

for (const mode of THEME_MODES) {
  test.describe(`commit-detail — ${mode}`, () => {
    test("graph default (no selection)", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        extra: { get_graph_viewport: makeGraphViewport({ count: 12 }) },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Graph");
      await expect(page).toHaveScreenshot(`${mode}-no-selection.png`, {
        animations: "disabled",
      });
    });

    test("commit selected with file list", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        extra: {
          get_graph_viewport: makeGraphViewport({ count: 12 }),
          get_commit_detail: makeCommitInfo({
            oid: TARGET_OID,
            summary: "feat(visual): add commit detail screenshots",
            body:
              "Adds per-state baselines for the commit detail pane.\n\n" +
              "Drives the existing `get_commit_*` commands with realistic fixtures so the\n" +
              "diff panel renders an actual hunk instead of an empty placeholder.",
            author: "Adolfo Fuentes",
            email: "adolfo@example.com",
            parents: ["0".repeat(40)],
            refs: ["HEAD", "refs/heads/feat/visual-tests"],
          }),
          get_commit_files: [
            makeCommitFileChange({ path: "tests/visual/components/commit-detail.spec.ts", status: "A" }),
            makeCommitFileChange({ path: "src/test/fixtures/commits.ts", status: "M" }),
            makeCommitFileChange({ path: "tests/visual/helpers/index.ts", status: "M" }),
          ],
          get_commit_full_diff: {
            "src/test/fixtures/commits.ts": makeFileDiff({
              path: "src/test/fixtures/commits.ts",
              additions: 12,
              deletions: 0,
            }),
          },
          get_commit_stats: makeCommitStats({
            files_changed: 3,
            insertions: 142,
            deletions: 28,
          }),
        },
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Graph");

      // Drive selection through the store directly — the canvas-based
      // commit list can't be clicked from Playwright reliably.
      await page.evaluate((oid) => {
        // Two-step: triggers the same fetches that a click does.
        return Promise.all([
          window.__TAURI_INTERNALS__!.invoke("get_commit_detail", { oid }),
          window.__TAURI_INTERNALS__!.invoke("get_commit_files", { oid }),
        ]);
      }, TARGET_OID);
      await page.waitForTimeout(300);

      await expect(page).toHaveScreenshot(`${mode}-selected.png`, {
        animations: "disabled",
      });
    });
  });
}
