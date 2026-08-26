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
  waitForGraphPainted,
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
/** Node count of the viewport this test mocks, for the paint wait. */
const GRAPH_COMMITS = 12;

for (const mode of THEME_MODES) {
  test.describe(`commit-detail — ${mode}`, () => {
    // No "graph default (no selection)" case here. It rendered the same
    // 12-commit viewport as `graph — single-lane` with the same mocks and
    // produced a byte-identical baseline, so it added no coverage — and
    // being the heaviest canvas in a spec that runs late in the schedule,
    // it was the one screenshot in the suite that still disagreed with
    // itself (~4,655px, roughly one run in ten) once everything else had
    // been made deterministic. The unselected graph stays covered by
    // `graph.spec.ts`; what this spec is actually for is the detail pane.

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
            makeCommitFileChange({ path: "tests/visual/components/commit-detail.spec.ts", status: "added" }),
            makeCommitFileChange({ path: "src/test/fixtures/commits.ts", status: "modified" }),
            makeCommitFileChange({ path: "tests/visual/helpers/index.ts", status: "modified" }),
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
      await waitForGraphPainted(page, GRAPH_COMMITS);

      // Click the first row of the canvas. `graphHitTest` resolves a click
      // to `Math.floor(y / ROW_HEIGHT) + offset` and treats a hit anywhere
      // in the text area as a node hit, so row 0 is (any x past the lanes,
      // ROW_HEIGHT / 2) — deterministic, and it runs the real
      // `selectCommit` path.
      //
      // What it replaced called `invoke("get_commit_detail")` straight
      // through the mock, which resolves a canned value to nobody and left
      // the app in exactly the unselected state the sibling test captures.
      // Both baselines came out byte-identical, so this one could never
      // fail for the reason it exists.
      await page.locator("canvas").first().click({ position: { x: 300, y: 14 } });
      await page.locator(".graph-detail-sidebar").waitFor({ timeout: 10_000 });
      await expect(page.getByText("feat(visual): add commit detail screenshots")).toBeVisible();

      await expect(page).toHaveScreenshot(`${mode}-selected.png`, {
        animations: "disabled",
        // The detail pane is the subject; the graph is behind it. Masked
        // for the same reason as the route baselines.
        mask: [page.locator("canvas")],
      });
    });
  });
}
