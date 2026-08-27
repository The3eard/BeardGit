/**
 * The commit-detail file list windows its rows above 500 files.
 *
 * A commit's file list has no cap — an initial commit or a wide merge carries
 * thousands of paths. Same two assertions as the changes list: below the
 * threshold nothing changes (so the baselines hold), above it the DOM stays
 * small while the scroll height and the last row stay correct.
 *
 * Assertion-based — no screenshots.
 */

import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import {
  clickNav,
  installBootstrapMocks,
  waitForAppReady,
  waitForGraphPainted,
  type IpcResponses,
} from "./helpers";
import {
  makeCommitFileChange,
  makeCommitInfo,
  makeCommitStats,
  makeGraphViewport,
  makeProjectInfo,
} from "../../src/test/fixtures";

const PROJECT = makeProjectInfo();
const TARGET_OID = "1".repeat(40);
/** Node count of the mocked viewport, for the paint wait. */
const GRAPH_COMMITS = 12;

/** Select a commit whose detail carries `n` changed files. */
async function selectCommitWithFiles(page: Page, n: number) {
  const files = Array.from({ length: n }, (_, i) =>
    makeCommitFileChange({ path: `src/dir${i % 20}/file${i}.ts`, status: "modified" }),
  );
  const extra: IpcResponses = {
    get_graph_viewport: makeGraphViewport({ count: GRAPH_COMMITS }),
    get_commit_detail: makeCommitInfo({
      oid: TARGET_OID,
      summary: "a commit with a lot of files",
      parents: ["0".repeat(40)],
    }),
    get_commit_files: files,
    get_commit_stats: makeCommitStats({ files_changed: n, insertions: n, deletions: 0 }),
  };
  await installBootstrapMocks(page, { activeProject: PROJECT, extra });
  await page.goto("/");
  await waitForAppReady(page);
  await clickNav(page, "Graph");
  await waitForGraphPainted(page, GRAPH_COMMITS);
  // Row 0 of the canvas — the same deterministic hit `commit-detail.spec` uses.
  await page.locator("canvas").first().click({ position: { x: 300, y: 14 } });
  await page.locator(".graph-detail-sidebar").waitFor({ timeout: 10_000 });
  await page.waitForSelector(".file-list .file-item", { timeout: 10_000 });
}

const probe = () => {
  const rows = Array.from(document.querySelectorAll<HTMLElement>(".file-list .file-item"));
  return {
    rows: rows.length,
    sizers: document.querySelectorAll(".file-list .virt-sizer").length,
    lastLabel: rows.length ? (rows[rows.length - 1].textContent ?? "").trim() : null,
  };
};

test("below the threshold every file row is mounted", async ({ page }) => {
  await selectCommitWithFiles(page, 400);
  const info = await page.evaluate(probe);
  expect(info.sizers, "no sizer on the plain path").toBe(0);
  expect(info.rows).toBe(400);
});

test("above the threshold only a window is mounted", async ({ page }) => {
  await selectCommitWithFiles(page, 5_000);

  const top = await page.evaluate(probe);
  expect(top.sizers).toBe(1);
  expect(top.rows, "a viewport's worth plus overscan, not 5k").toBeLessThan(150);
  expect(top.rows).toBeGreaterThan(5);

  // Scroll the detail pane to the bottom and check the real last file is there.
  await page.evaluate(() => {
    const scroller = document.querySelector<HTMLElement>(".commit-detail");
    if (scroller) scroller.scrollTop = scroller.scrollHeight;
  });
  await page.waitForTimeout(200);

  const bottom = await page.evaluate(probe);
  expect(bottom.rows).toBeLessThan(150);
  expect(bottom.lastLabel).toContain("file4999.ts");
});
