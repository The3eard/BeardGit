/**
 * Per-state baselines for the Changes view.
 *
 * Each scenario varies `get_file_statuses` + `get_status_summary` so the
 * StagingArea panel exercises a distinct visual state. Most keep the diff
 * panel empty — that's covered in `commit-detail.spec.ts` — except
 * `populated-diff`, which exists specifically to put added/removed rows on
 * screen.
 *
 * That last one is the end-to-end guard for the theme-token class of bug.
 * `--diff-added-bg` / `--diff-removed-bg` were inert for months because
 * the Rust side serialized `added-bg` while the TypeScript mirror read
 * `added_bg`, so light themes rendered dark green and red diff rows. No
 * unit test could see it — the token reaches the DOM through `var()`, and
 * both test suites agreed with the types by construction. A rendered
 * light-theme diff is the only thing that catches it.
 */

import { expect, test } from "@playwright/test";

import {
  applyTheme,
  clickNav,
  installBootstrapMocks,
  THEME_MODES,
  waitForAppReady,
  type IpcResponses,
} from "../helpers";
import {
  makeFileDiff,
  makeFileDiffStat,
  makeFileStatus,
  makeFileStatusList,
  makeProjectInfo,
  makeStatusSummary,
} from "../../../src/test/fixtures";
import type {
  FileDiff,
  FileStatus,
  StatusSummary,
} from "../../../src/lib/types";

const PROJECT = makeProjectInfo({
  name: "sample",
  head_branch: "feat/example",
});

interface Scenario {
  files: FileStatus[];
  summary: StatusSummary;
  /** Diff rows to render, served from `get_diff_file`. */
  diff?: FileDiff;
  /** File to click so the diff panel opens. */
  select?: string;
}

const SCENARIOS: Record<string, Scenario> = {
  empty: {
    files: [],
    summary: makeStatusSummary(),
  },
  "only-staged": {
    files: [
      makeFileStatus({ path: "src/lib/feature.ts", status: "modified", is_staged: true }),
      makeFileStatus({ path: "src/lib/types/index.ts", status: "modified", is_staged: true }),
      makeFileStatus({ path: "src/lib/utils/format.ts", status: "new", is_staged: true }),
    ],
    summary: makeStatusSummary({ staged: 3 }),
  },
  "only-unstaged": {
    files: [
      makeFileStatus({ path: "src/routes/+page.svelte", status: "modified", is_staged: false }),
      makeFileStatus({ path: "src/lib/components/ui/Button.svelte", status: "modified", is_staged: false }),
      makeFileStatus({ path: "src/lib/legacy/old-helper.ts", status: "deleted", is_staged: false }),
    ],
    summary: makeStatusSummary({ unstaged: 3 }),
  },
  "mixed-populated": {
    files: makeFileStatusList(),
    summary: makeStatusSummary({ staged: 3, unstaged: 3, untracked: 2 }),
  },
  "populated-diff": {
    files: [makeFileStatus({ path: "src/a.ts", status: "modified", is_staged: false })],
    summary: makeStatusSummary({ unstaged: 1 }),
    diff: makeFileDiff({ path: "src/a.ts" }),
    select: "src/a.ts",
  },
  "many-untracked": {
    files: Array.from({ length: 10 }, (_, i) =>
      makeFileStatus({
        path: `tests/visual/scratch/untracked-${i + 1}.ts`,
        // `"untracked"` exists, but on the *diff* channel. `get_file_statuses`
        // is the staging one, where untracked arrives as `"new"` with
        // `is_staged: false` (`git-engine/src/staging.rs`). With the wrong
        // word this baseline recorded ten blue `U` badges where the app
        // renders ten green `A`.
        status: "new",
        is_staged: false,
      }),
    ),
    summary: makeStatusSummary({ untracked: 10 }),
  },
};

function fixtureFor(scenario: Scenario): IpcResponses {
  return {
    get_file_statuses: scenario.files,
    get_status_summary: scenario.summary,
    get_diff_workdir: [],
    get_diff_index: [],
    // The lists are driven by the lightweight per-file stats; the selected
    // file's hunks come from `get_diff_file`, fetched lazily.
    get_diff_stats_workdir: scenario.select
      ? [makeFileDiffStat({ path: scenario.select })]
      : [],
    get_diff_stats_index: [],
    get_diff_file: scenario.diff ?? null,
  };
}

for (const mode of THEME_MODES) {
  test.describe(`changes — ${mode}`, () => {
    for (const [name, scenario] of Object.entries(SCENARIOS)) {
      test(name, async ({ page }) => {
        await installBootstrapMocks(page, {
          mode,
          activeProject: PROJECT,
          extra: fixtureFor(scenario),
        });
        await page.goto("/");
        await applyTheme(page, mode);
        await waitForAppReady(page);
        await clickNav(page, "Changes");
        if (scenario.select) {
          // Open the diff panel so the added/removed row backgrounds are
          // actually in the screenshot.
          const testId = `file-row-${scenario.select.replace(/\//g, "-")}`;
          await page.getByTestId(testId).locator(".file-btn").click();
          await expect(page.locator(".staging-diff-editor")).toBeVisible();
        }
        await expect(page).toHaveScreenshot(`${mode}-${name}.png`, {
          animations: "disabled",
        });
      });
    }
  });
}
