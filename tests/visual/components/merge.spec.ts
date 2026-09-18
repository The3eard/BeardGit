/**
 * Per-state baselines for the merge conflict editor.
 *
 * The editor is a fixed overlay opened from the conflict banner, so the
 * scenarios drive `get_conflict_status` + `get_conflict_file_contents`
 * and click through the banner's file list. Three states matter:
 *
 * - `pending`: both conflicts untouched. Side headers, center widgets and
 *   connectors all visible.
 * - `half-resolved`: theirs accepted on the first conflict from its side
 *   header. The accepted lines sit above the placeholder in the result,
 *   the theirs header shows its badge and the center widget keeps only
 *   the ours buttons. This is the state the per-side decision model adds,
 *   and the one a regression to "accept replaces the placeholder" would
 *   silently lose.
 * - `sticky`: the incoming side scrolled into the middle of a long block
 *   while the result fits on screen and cannot scroll at all — the shape
 *   that used to leave the block unreachable. The incoming header has left
 *   the viewport, so the pinned bar at the panel's top carries the actions.
 */

import { expect, test } from "@playwright/test";

import {
  installBootstrapMocks,
  THEME_MODES,
  waitForAppReady,
} from "../helpers";
import { byArg } from "../helpers/mock-ipc";
import { makeConflictStatus, makeProjectInfo } from "../../../src/test/fixtures";

const PROJECT = makeProjectInfo({ name: "sample", head_branch: "main" });

const lines = (n: number, f: (i: number) => string) =>
  Array.from({ length: n }, (_, i) => f(i + 1)).join("\n");

/** Two conflicts: a rewritten function and a disputed constant. */
const CONFIG_TS = {
  base: [
    "export const retries = 3;",
    "",
    "export function backoff(attempt: number): number {",
    "  return attempt * 100;",
    "}",
    "",
    "export const timeout = 1000;",
  ].join("\n"),
  theirs: [
    "export const retries = 3;",
    "",
    "export function backoff(attempt: number): number {",
    "  const base = 2 ** attempt * 100;",
    "  return Math.min(base, 5_000);",
    "}",
    "",
    "export const timeout = 2000;",
  ].join("\n"),
  ours: [
    "export const retries = 3;",
    "",
    "export function backoff(attempt: number): number {",
    "  return attempt * 250;",
    "}",
    "",
    "export const timeout = 1500;",
  ].join("\n"),
};

/** One conflict where the incoming side replaces a line with a 120-line block. */
const BLOCK_TXT = {
  base: lines(12, (i) => `setting ${i} = ${i}`),
  theirs: lines(12, (i) => (i === 6 ? lines(120, (k) => `incoming line ${k}`) : `setting ${i} = ${i}`)),
  ours: lines(12, (i) => (i === 6 ? "setting 6 = local" : `setting ${i} = ${i}`)),
};

const FILES = { "src/config.ts": CONFIG_TS, "block.txt": BLOCK_TXT };

async function openMergeEditor(page: import("@playwright/test").Page, mode: (typeof THEME_MODES)[number], file: keyof typeof FILES) {
  await installBootstrapMocks(page, {
    mode,
    activeProject: PROJECT,
    extra: {
      get_conflict_status: makeConflictStatus({
        state: "merging",
        conflicted_files: Object.keys(FILES),
      }),
      get_conflict_file_contents: byArg("path", FILES, null),
    },
  });
  await page.goto("/");
  await waitForAppReady(page);
  await page.locator(".conflict-files-btn").click();
  await page.locator(".conflict-file-item", { hasText: file }).click();
  await page.locator(".merge-editor-wrapper").waitFor();
  // Connectors and side headers render one frame after the editors mount.
  await page.locator(".cm-side-conflict").first().waitFor();
  await page.locator(".connector-svg path, .connector-svg line").first().waitFor({ state: "attached" });
}

for (const mode of THEME_MODES) {
  test.describe(`merge — ${mode}`, () => {
    test("pending", async ({ page }) => {
      await openMergeEditor(page, mode, "src/config.ts");
      await expect(page.locator(".conflict-counter")).toHaveText(/0\/2/);
      await expect(page).toHaveScreenshot(`${mode}-pending.png`, { animations: "disabled" });
    });

    test("half-resolved", async ({ page }) => {
      await openMergeEditor(page, mode, "src/config.ts");
      await page.locator(".cm-side-conflict-theirs .cm-cw-accept").first().click();
      await expect(page.locator(".cm-conflict-widget .cm-cw-state")).toHaveCount(1);
      await expect(page.locator(".conflict-counter")).toHaveText(/0\/2/);
      await expect(page).toHaveScreenshot(`${mode}-half-resolved.png`, { animations: "disabled" });
    });

    test("sticky", async ({ page }) => {
      await openMergeEditor(page, mode, "block.txt");
      const theirs = page.locator(".panel-editor").first();
      await theirs.hover();
      await page.mouse.wheel(0, 900);
      await theirs.locator(".sticky-conflict").waitFor();
      // The result is 13 lines here and cannot scroll, which is the point of
      // the scenario: the incoming pane still can. Wait for its scroll to
      // have landed before capturing.
      await page.waitForFunction(() => {
        const scroller = document.querySelector(".panel-editor .cm-scroller");
        return !!scroller && scroller.scrollTop >= 800;
      });
      await expect(page).toHaveScreenshot(`${mode}-sticky.png`, { animations: "disabled" });
    });
  });
}
