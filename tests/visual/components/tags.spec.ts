/**
 * Per-state baselines for the Tags view detail pane.
 *
 * Exercises the tag-selection flow (`list_tags_paginated` +
 * `get_commit_detail` / `get_commit_stats` / `get_commit_files`) and the
 * embedded parent CommitDetail panel that opens when a parent OID chip
 * is clicked — the panel regressed once by rendering as an unframed
 * graph-sidebar fragment inside the card-based body.
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
  makeProjectInfo,
} from "../../../src/test/fixtures";

const PROJECT = makeProjectInfo();

const TAG = {
  name: "v1.2.3",
  object_oid: "aaaa111122223333aaaa111122223333aaaa1111",
  commit_oid: "bbbb111122223333aaaa111122223333aaaa1111",
  annotated: true,
  message: "Release v1.2.3 — bug fixes and polish",
  tagger_name: "Adolfo Fuentes",
  tagger_email: "adolfo@example.com",
  date: "2026-06-10T10:00:00Z",
};

const FIXTURES = {
  list_tags_paginated: [TAG],
  get_commit_detail: makeCommitInfo({
    oid: TAG.commit_oid,
    summary: "feat(core): the tagged commit",
    body: "Longer body of the tagged commit.",
    parents: ["cccc111122223333aaaa111122223333aaaa1111"],
    refs: ["refs/tags/v1.2.3", "refs/heads/main"],
  }),
  get_commit_stats: makeCommitStats({
    files_changed: 3,
    insertions: 42,
    deletions: 7,
  }),
  get_commit_files: [
    makeCommitFileChange({ path: "src/lib/a.ts", status: "M" }),
    makeCommitFileChange({ path: "src/lib/b.ts", status: "A" }),
    makeCommitFileChange({ path: "README.md", status: "M" }),
  ],
};

for (const mode of THEME_MODES) {
  test.describe(`tags — ${mode}`, () => {
    test("parent commit detail panel", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        forge: "github",
        extra: FIXTURES,
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await clickNav(page, "Tags");

      // Select the tag, then open the embedded parent CommitDetail.
      await page.locator(".list-row").first().click();
      await page.locator(".parent-oid.clickable").first().click();
      await page.locator(".parent-detail-panel").waitFor();

      // Scroll the detail body so the parent panel is fully in frame.
      await page
        .locator(".tag-detail > .detail-body")
        .first()
        .evaluate((el) => {
          el.scrollTop = el.scrollHeight;
        });

      await expect(page).toHaveScreenshot(`${mode}-parent-detail.png`, {
        animations: "disabled",
      });
    });
  });
}
