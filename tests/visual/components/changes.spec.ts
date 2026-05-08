/**
 * Per-state baselines for the Changes view.
 *
 * Each scenario varies `get_file_statuses` + `get_status_summary` so
 * the StagingArea panel exercises a distinct visual state. Diff panels
 * are kept empty here — that's covered separately in
 * `commit-detail.spec.ts`.
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
  makeFileStatus,
  makeFileStatusList,
  makeProjectInfo,
  makeStatusSummary,
} from "../../../src/test/fixtures";
import type {
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
}

const SCENARIOS: Record<string, Scenario> = {
  empty: {
    files: [],
    summary: makeStatusSummary(),
  },
  "only-staged": {
    files: [
      makeFileStatus({ path: "src/lib/feature.ts", status: "M", is_staged: true }),
      makeFileStatus({ path: "src/lib/types/index.ts", status: "M", is_staged: true }),
      makeFileStatus({ path: "src/lib/utils/format.ts", status: "A", is_staged: true }),
    ],
    summary: makeStatusSummary({ staged: 3 }),
  },
  "only-unstaged": {
    files: [
      makeFileStatus({ path: "src/routes/+page.svelte", status: "M", is_staged: false }),
      makeFileStatus({ path: "src/lib/components/ui/Button.svelte", status: "M", is_staged: false }),
      makeFileStatus({ path: "src/lib/legacy/old-helper.ts", status: "D", is_staged: false }),
    ],
    summary: makeStatusSummary({ unstaged: 3 }),
  },
  "mixed-populated": {
    files: makeFileStatusList(),
    summary: makeStatusSummary({ staged: 3, unstaged: 3, untracked: 2 }),
  },
  "many-untracked": {
    files: Array.from({ length: 10 }, (_, i) =>
      makeFileStatus({
        path: `tests/visual/scratch/untracked-${i + 1}.ts`,
        status: "?",
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
        await expect(page).toHaveScreenshot(`${mode}-${name}.png`, {
          animations: "disabled",
        });
      });
    }
  });
}
