/**
 * The changes list windows its rows above 500 files.
 *
 * `file_statuses` recurses untracked directories, so a `node_modules` that
 * isn't ignored arrives file by file — tens of thousands of rows, each
 * mounting a Checkbox, a badge and an IconButton. This asserts the two
 * things that matter: below the threshold nothing changes (which is why the
 * visual baselines still hold), and above it the DOM stays small while the
 * scrollbar and the last row stay correct.
 *
 * Assertion-based — no screenshots.
 */

import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import {
  installBootstrapMocks,
  waitForAppReady,
  clickNav,
  type IpcResponses,
} from "./helpers";
import {
  makeFileStatus,
  makeFileDiffStat,
  makeProjectInfo,
  makeStatusSummary,
} from "../../src/test/fixtures";

const PROJECT = makeProjectInfo({ name: "sample", head_branch: "main" });

/** Boot the Changes view with `n` unstaged files. */
async function mountChanges(page: Page, n: number) {
  const files = Array.from({ length: n }, (_, i) =>
    makeFileStatus({
      path: `src/dir${i % 20}/file${i}.ts`,
      status: "modified",
      is_staged: false,
    }),
  );
  const extra: IpcResponses = {
    get_file_statuses: files,
    get_status_summary: makeStatusSummary({ staged: 0, unstaged: n }),
    get_diff_stats_workdir: files.map((f) => makeFileDiffStat({ path: f.path })),
    get_diff_stats_index: [],
  };
  await installBootstrapMocks(page, { activeProject: PROJECT, extra });
  await page.goto("/");
  await waitForAppReady(page);
  await clickNav(page, "Changes");
  await page.waitForSelector(".file-item", { timeout: 15_000 });
}

/** Row count, sizer presence, scroll height, and the last mounted index. */
const probe = () => ({
  rows: document.querySelectorAll(".file-item").length,
  sizers: document.querySelectorAll(".virt-sizer").length,
  scrollerHeight: document.querySelector(".file-lists")?.scrollHeight ?? 0,
  lastIndex: (() => {
    const rows = Array.from(
      document.querySelectorAll<HTMLElement>(".file-item"),
    );
    return rows.length ? rows[rows.length - 1].dataset.rowIndex : null;
  })(),
});

test("below the threshold every row is mounted, unchanged", async ({ page }) => {
  await mountChanges(page, 400);
  const info = await page.evaluate(probe);
  expect(info.sizers, "no sizer on the plain path").toBe(0);
  expect(info.rows).toBe(400);
});

test("above the threshold only a window is mounted", async ({ page }) => {
  await mountChanges(page, 30_000);

  const top = await page.evaluate(probe);
  expect(top.sizers).toBe(1);
  expect(top.rows, "a viewport's worth plus overscan, not 30k").toBeLessThan(150);
  expect(top.rows).toBeGreaterThan(10);
  // The sizer keeps the scrollbar honest: 30k * 28px.
  expect(top.scrollerHeight).toBeGreaterThan(800_000);

  // Scrolling to the bottom reaches the real last row rather than blank space.
  await page.evaluate(() => {
    const scroller = document.querySelector<HTMLElement>(".file-lists")!;
    scroller.scrollTop = scroller.scrollHeight;
  });
  await page.waitForTimeout(200);

  const bottom = await page.evaluate(probe);
  expect(bottom.rows).toBeLessThan(150);
  expect(bottom.lastIndex).toBe("29999");
});
