/**
 * Per-state baselines for the Compare view (CompareView + RefPicker).
 *
 * Compare has no sidebar entry — it's reached from the command palette
 * (`nav.compare`) and the graph/branches context menus. The spec opens
 * it via the palette, then drives the two header RefPickers to set the
 * base/compare refs. Committing side B (with side A already set) kicks
 * off `runCompare`, which resolves against the mocked `get_merge_base` /
 * `get_commits_between` / `get_diff_between_commits` responses.
 */

import { expect, test, type Page } from "@playwright/test";

import {
  applyTheme,
  installBootstrapMocks,
  THEME_MODES,
  waitForAppReady,
  type IpcResponses,
} from "../helpers";
import {
  makeBranchList,
  makeCommitFileChange,
  makeCommitInfo,
  makeProjectInfo,
} from "../../../src/test/fixtures";

const PROJECT = makeProjectInfo({
  name: "sample",
  head_branch: "feat/example",
});

// A short range for the populated state. `get_commits_between` is called
// for both directions (ahead A..B and behind B..A); the mock returns this
// same list for each, so the summary shows equal ahead/behind counts —
// fine for a static baseline.
const COMMITS = [
  makeCommitInfo({
    oid: "a".repeat(40),
    summary: "feat(compare): ref picker autocomplete over branches + tags",
    author: "Adolfo Fuentes",
  }),
  makeCommitInfo({
    oid: "b".repeat(40),
    summary: "fix(compare): fall back to A for unrelated histories",
    author: "Sam Rivera",
  }),
  makeCommitInfo({
    oid: "c".repeat(40),
    summary: "test(compare): cover two-dot vs three-dot ranges",
    author: "Adolfo Fuentes",
  }),
];

// The diff path (`get_diff_between_commits`) emits the diff vocabulary
// ("modified" / "added" / "deleted" / …), not the single-letter form.
const FILES = [
  makeCommitFileChange({ path: "src/lib/stores/compare.ts", status: "modified" }),
  makeCommitFileChange({ path: "src/lib/components/compare/CompareView.svelte", status: "added" }),
  makeCommitFileChange({ path: "src/lib/components/compare/RefPicker.svelte", status: "added" }),
  makeCommitFileChange({ path: "src/lib/legacy/old-diff.ts", status: "deleted" }),
];

function compareFixture(): IpcResponses {
  return {
    // Populate the pickers' autocomplete + the compare backend calls.
    get_branches: makeBranchList(),
    list_tags: [],
    get_merge_base: "f".repeat(40),
    get_commits_between: COMMITS,
    get_diff_between_commits: FILES,
  };
}

/** Open the palette (Cmd/Ctrl+Shift+P) and run "Compare refs…". */
async function openComparePalette(page: Page): Promise<void> {
  const input = page.locator("input.cp-input");
  // The palette is opened by a global keyboard shortcut; the first
  // keypress can land before the window keydown listener is settled, so
  // retry the press until the input appears. `openCommandPalette` just
  // sets the open flag, so repeated presses are idempotent.
  await expect(async () => {
    await page.keyboard.press("ControlOrMeta+Shift+P");
    await expect(input).toBeVisible({ timeout: 1000 });
  }).toPass({ timeout: 10_000 });
  await input.fill("Compare refs");
  await page.keyboard.press("Enter");
  // Wait for the lazy-loaded CompareView to mount.
  await page.locator(".compare-view").waitFor({ state: "visible" });
}

for (const mode of THEME_MODES) {
  test.describe(`compare — ${mode}`, () => {
    test("empty (both refs unset)", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        extra: compareFixture(),
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await openComparePalette(page);
      await expect(page).toHaveScreenshot(`${mode}-empty.png`, {
        animations: "disabled",
      });
    });

    test("three-dot with results", async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: PROJECT,
        extra: compareFixture(),
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
      await openComparePalette(page);

      // Set both sides through the header pickers; committing side B (with
      // side A already set) triggers runCompare().
      const base = page.locator('input.rp-input[aria-label="Base"]');
      await base.fill("main");
      await base.press("Enter");
      const compare = page.locator('input.rp-input[aria-label="Compare"]');
      await compare.fill("feat/example");
      await compare.press("Enter");

      // Wait for the ahead summary chip to render (compare resolved).
      await page.locator(".summary .chip--ahead").waitFor({ state: "visible" });
      await page.waitForTimeout(150);
      await expect(page).toHaveScreenshot(`${mode}-three-dot-results.png`, {
        animations: "disabled",
      });
    });
  });
}
