/**
 * Visual baselines — one screenshot per sidebar view, dark + light.
 *
 * Each test installs the bootstrap mocks (theme, sidebar layout, projects,
 * etc.) and a per-view fixture set so the screenshot reflects a populated
 * state instead of an empty post-bootstrap shell.
 *
 * To regenerate baselines after intentional UI changes:
 *   npm run test:visual:update
 */

import { expect, test } from "@playwright/test";

import {
  applyTheme,
  installBootstrapMocks,
  THEME_MODES,
  waitForAppReady,
  type IpcResponses,
} from "./helpers";
import {
  makeBranchList,
  makeCiRunList,
  makeFileStatusList,
  makeGraphViewport,
  makeIssueList,
  makeMrPrList,
  makeProjectInfo,
  makeRepoInfo,
  makeStatusSummary,
} from "../../src/test/fixtures";

const ACTIVE_PROJECT = makeProjectInfo({
  path: "/Users/test/projects/sample",
  name: "sample",
  head_branch: "feat/example",
  change_count: 4,
});

/**
 * Per-view fixture overrides. Keys are command names exactly as
 * passed to `invoke()`. Empty arrays are deliberate — they trigger
 * the empty-state baseline rather than an unknown-shape default.
 */
function fixtureBundle(): IpcResponses {
  return {
    // Repo
    open_repo: makeRepoInfo({ path: ACTIVE_PROJECT.path, head_branch: "feat/example" }),
    get_status_summary: makeStatusSummary({ ahead: 2, staged: 2, unstaged: 2, untracked: 0 }),
    get_branches: makeBranchList(),
    get_remotes: [
      { name: "origin", url: "git@github.com:adolfofuentes/sample.git" },
    ],

    // Graph
    get_graph_viewport: makeGraphViewport({ count: 25 }),
    refresh_graph_layout: undefined,

    // Changes
    get_file_statuses: makeFileStatusList(),
    get_diff_workdir: [],
    get_diff_index: [],

    // MRs / PRs
    list_mr_prs: makeMrPrList(),

    // Issues
    list_issues: makeIssueList(),
    list_milestones: [],
    list_labels: [],

    // Pipelines / CI
    list_ci_runs: makeCiRunList(),

    // Lists that should render empty-state for now
    list_tags: [],
    list_stashes: [],
    list_worktrees: [],
    get_reflog: [],
    list_submodules: [],
    list_releases: [],
    list_workflows: [],
    list_ai_sessions: [],

    // Repo config / blame / file editor
    get_repo_config_labels: [],
    get_branch_protections: [],
    get_remote_repo_config: null,
    list_workdir_tree: [],
  };
}

const VIEWS: Array<[id: string, sidebarLabel: string]> = [
  ["graph", "Graph"],
  ["changes", "Changes"],
  ["branches", "Branches"],
  ["tags", "Tags"],
  ["stashes", "Stashes"],
  ["worktrees", "Worktrees"],
  ["reflog", "Reflog"],
  ["bisect", "Bisect"],
  ["submodules", "Submodules"],
  ["pipelines", "Pipelines"],
  ["issues", "Issues"],
  ["merge-requests", "Pull Requests"],
  ["releases", "Releases"],
  ["ai-sessions", "AI Sessions"],
  ["repo-config", "Repo settings"],
  ["settings", "Settings"],
];

for (const mode of THEME_MODES) {
  test.describe(`visual baseline — ${mode}`, () => {
    test.beforeEach(async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        activeProject: ACTIVE_PROJECT,
        recentRepos: [{ path: ACTIVE_PROJECT.path, name: ACTIVE_PROJECT.name }],
        extra: fixtureBundle(),
      });
      await page.goto("/");
      await applyTheme(page, mode);
      await waitForAppReady(page);
    });

    for (const [id, label] of VIEWS) {
      test(`${id}`, async ({ page }) => {
        // `Settings` lives outside the <nav> in Sidebar.svelte but uses
        // the same `.nav-item` class — we target by label match on the
        // nav-label span so a single selector hits every sidebar entry.
        await page
          .locator(
            `button.nav-item:has(.nav-label:text-is("${label}"))`,
          )
          .first()
          .click();
        await expect(page).toHaveScreenshot(`${mode}-${id}.png`, {
          fullPage: false,
          animations: "disabled",
        });
      });
    }
  });
}
