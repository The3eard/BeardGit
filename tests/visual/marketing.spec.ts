/**
 * Marketing screenshots — NOT a baseline suite.
 *
 * Renders the real UI (Svelte under `npm run dev` + mock IPC) populated
 * with authentic data from the two prepared test repos, at 2× device
 * scale, and writes paired light/dark PNGs to
 * `docs/assets/screenshots/_new/`. The landing page swaps the matching
 * image when its theme toggle flips, so each view is captured in both
 * modes with nothing else changed.
 *
 * Run via `npm run build:screenshots` (which also produces webp/avif),
 * or directly:  npx playwright test marketing.spec.ts
 *
 * Output under `_new/` is git-ignored until the captures are approved
 * and promoted over the live `assets/screenshots/*`.
 */

import { mkdir } from "node:fs/promises";

import { test, type Page } from "@playwright/test";

import {
  applyTheme,
  clickNav,
  FIXED_NOW,
  installBootstrapMocks,
  THEME_MODES,
  waitForAppReady,
  type IpcResponses,
  type ThemeMode,
} from "./helpers";
import {
  aiConversationList,
  branchList,
  ciRunDetail,
  ciRunList,
  fileStatusList,
  GH_PROJECT,
  GL_PROJECT,
  graphViewport,
  issueDetail,
  issueList,
  mrList,
  prDetail,
  prDiff,
  prList,
  readFileResult,
  reflogList,
  releaseList,
  stashList,
  submoduleList,
  tagList,
  worktreeList,
  workdirTree,
} from "./fixtures/marketing";
import {
  makeCommitFileChange,
  makeCommitInfo,
  makeFileDiff,
  makeStatusSummary,
} from "../../src/test/fixtures";
import { SHOWCASE_THEMES } from "./fixtures/theme-data";

const COMMIT_OID = "5e8ec3ea4b29f07c3d8e6021f9a4c7b8d0e1f234";
const GOOD_OID = "80beb31c7a14e9d2f6b305a8c1e0742b9d63f5a0";

const OUT_DIR = "docs/assets/screenshots/_new";

/**
 * Unix seconds, `days` before the harness's frozen clock. The shared commit
 * factory has a fixed 2024 timestamp, so without this every capture shows
 * its commits as "2 years ago" beside a 2026 version badge.
 */
function daysAgo(days: number, hours = 0): number {
  return Math.floor((FIXED_NOW.getTime() - (days * 86400 + hours * 3600) * 1000) / 1000);
}


// 2× scale over the baseline viewport → ~2880×1800 marketing PNGs.
test.use({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });

/** Every command any captured view might call, with real-repo data. */
function commonFixtures(host: "github" | "gitlab"): IpcResponses {
  return {
    get_status_summary: makeStatusSummary({ ahead: 0, behind: 0, staged: 2, unstaged: 4, untracked: 0 }),
    get_branches: branchList(),
    get_remotes: [
      {
        name: "origin",
        url:
          host === "github"
            ? "git@github.com:The3eard/beardgit_gh_tests.git"
            : "git@gitlab.com:The3eard/beardgit_glab_tests.git",
      },
    ],

    // Graph + commit detail
    get_graph_viewport: graphViewport(),
    refresh_graph_layout: undefined,
    get_commit_detail: makeCommitInfo({
      oid: COMMIT_OID,
      summary: "feat(recurrence): roll repeating tasks forward when marked done",
      body: "When a repeating task is marked done, advance its due date to the\nnext occurrence instead of closing it.",
      author: "Adolfo Fuentes",
      email: "adolfo@beardgit.dev",
      timestamp: daysAgo(1, 3),
      // The factory default is a padded placeholder, which the detail pane
      // renders as a row of zeros next to a real SHA.
      parents: ["3b22f0e1d4c8a7069e2f5b3a8c17d0e94f6b2a58"],
    }),
    get_commit_files: [
      makeCommitFileChange({ path: "src/store.rs", status: "modified" }),
      makeCommitFileChange({ path: "src/recurrence.rs", status: "added" }),
      makeCommitFileChange({ path: "tests/recurrence.rs", status: "modified" }),
    ],

    // Bisect — an in-progress session (more telling than the idle start screen)
    bisect_get_state: {
      active: true,
      current_commit: COMMIT_OID,
      steps_remaining: 3,
      good_commits: [GOOD_OID],
      bad_commits: [COMMIT_OID],
    },
    bisect_get_log: "git bisect start\ngit bisect bad HEAD\ngit bisect good v0.2.0\nBisecting: 6 revisions left to test after this (roughly 3 steps)",

    // Changes
    get_file_statuses: fileStatusList(),
    get_diff_workdir: [makeFileDiff({ path: "src/cli.rs", status: "modified" })],
    get_diff_index: [makeFileDiff({ path: "src/store.rs", status: "modified" })],

    // Forge — PRs/MRs + detail
    list_mr_prs: host === "github" ? prList() : mrList(),
    get_mr_pr_detail: prDetail(),
    get_mr_pr_diff: prDiff(),

    // Issues + detail
    list_issues: issueList(),
    get_issue: issueDetail(),
    list_milestones: [],
    list_labels: [],

    // CI + detail
    list_ci_runs: ciRunList(host),
    get_ci_run_detail: ciRunDetail(host),
    list_ci_workflows: [],

    // Releases / tags
    list_releases: releaseList(),
    list_tags: tagList(),
    list_tags_paginated: tagList(),

    // Worktrees / reflog / stashes / submodules
    list_worktrees: worktreeList(),
    get_reflog: reflogList(),
    stash_entries: stashList(),
    list_submodules: submoduleList(),

    // Starred branches — they hoist to the top of their own level, which
    // is only visible in a capture if some of them are starred.
    get_favorite_branches: ["main", "feat/recurring-tasks"],

    // The branch detail pane, which otherwise sits in its skeleton state.
    get_branch_commits: [
      makeCommitInfo({
        oid: COMMIT_OID,
        summary: "feat(recurrence): roll repeating tasks forward when marked done",
        author: "Adolfo Fuentes",
        timestamp: daysAgo(1, 3),
      }),
      makeCommitInfo({
        oid: "3b22f0e1d4c8a7069e2f5b3a8c17d0e94f6b2a58",
        summary: "feat(recurrence): parse --repeat daily|weekly|monthly",
        author: "Adolfo Fuentes",
        timestamp: daysAgo(2),
      }),
      makeCommitInfo({
        oid: "8636791a2f0c5d8e41b7930a6c2d5f180e4b93a7",
        summary: "feat(cli): add --due and --tag filters to `tasklog list`",
        author: "Adolfo Fuentes",
        timestamp: daysAgo(4),
      }),
      makeCommitInfo({
        oid: "d7319fb2c085e1a9f34b7d6082e5c1a09b83f27d",
        summary: "test(recurrence): cover monthly rollover at month boundaries",
        author: "Adolfo Fuentes",
        timestamp: daysAgo(5),
      }),
    ],

    // Compare: a branch measured against the default branch
    get_merge_base: GOOD_OID,
    get_commits_between: [
      makeCommitInfo({
        oid: COMMIT_OID,
        summary: "feat(recurrence): roll repeating tasks forward when marked done",
        author: "Adolfo Fuentes",
        timestamp: daysAgo(1, 3),
      }),
      makeCommitInfo({
        oid: "3b22f0e1d4c8a7069e2f5b3a8c17d0e94f6b2a58",
        summary: "feat(recurrence): parse --repeat daily|weekly|monthly",
        author: "Adolfo Fuentes",
        timestamp: daysAgo(2),
      }),
      makeCommitInfo({
        oid: "d7319fb2c085e1a9f34b7d6082e5c1a09b83f27d",
        summary: "test(recurrence): cover monthly rollover at month boundaries",
        author: "Adolfo Fuentes",
        timestamp: daysAgo(5),
      }),
    ],
    get_diff_between_commits: [
      makeCommitFileChange({ path: "src/recurrence.rs", status: "added" }),
      makeCommitFileChange({ path: "src/store.rs", status: "modified" }),
      makeCommitFileChange({ path: "src/cli.rs", status: "modified" }),
      makeCommitFileChange({ path: "tests/recurrence.rs", status: "modified" }),
    ],

    // Both CLIs ship inside the installer, so a Settings capture that says
    // "not installed" would contradict the page it illustrates.
    cli_check_auth_status: [
      { tool: "gh", installed: true, authenticated: true, username: "adolfofuentes", error: null },
      { tool: "glab", installed: true, authenticated: true, username: "adolfofuentes", error: null },
    ],
    is_cli_authenticated: true,

    // AI
    ai_list_conversations: aiConversationList(),
    ai_list_background_runs: [],

    // Editor
    list_workdir_tree: workdirTree(),
    read_workdir_file: readFileResult(),

    // Requests (.http) workspace
    requests_list_project: [
      { kind: "file", name: "create-task.http", rel_path: "create-task.http", children: [] },
      { kind: "file", name: "list-tasks.http", rel_path: "list-tasks.http", children: [] },
      { kind: "file", name: "complete-task.http", rel_path: "complete-task.http", children: [] },
      { kind: "file", name: "health.http", rel_path: "health.http", children: [] },
    ],
    requests_list_global: [],
    requests_get_envs: [
      { name: "local", active: true, var_count: 3 },
      { name: "prod", active: false, var_count: 3 },
    ],
    requests_history: [],
    requests_load: [
      {
        name: "Create task",
        method: "POST",
        url: "{{base_url}}/api/tasks",
        headers: [
          ["Content-Type", "application/json"],
          ["Authorization", "Bearer {{token}}"],
        ],
        body: '{\n  "title": "Ship the recurring-tasks feature",\n  "tag": "work",\n  "repeat": "weekly",\n  "due": "2026-05-04"\n}',
      },
    ],
  };
}

interface ViewSpec {
  label: string;
  slug: string;
  /**
   * Click list/tree item(s) by visible text to populate a detail pane.
   * An array clicks in sequence (e.g. expand a dir, then open a file).
   */
  select?: string | string[];
  /** Press a chord, or a sequence of them, after navigating. */
  press?: string | string[];
  /**
   * Click at viewport coordinates. The graph paints its rows on a canvas,
   * so there is no text node for `select` to find and no keyboard route
   * into the commit detail pane — a row has to be clicked where it is.
   */
  clickAt?: { x: number; y: number };
}

const GH_VIEWS: ViewSpec[] = [
  // The graph rows are painted on a canvas, so there is no text node to
  // click: "j" moves the selection and opens the commit detail pane.
  { label: "Graph", slug: "graph", clickAt: { x: 576, y: 174 } },
  { label: "Graph", slug: "command-palette", press: "Meta+Shift+P" },
  { label: "Changes", slug: "changes" },
  { label: "Editor", slug: "editor", select: ["src", "store.rs"] },
  { label: "Branches", slug: "branches", select: "recurring-tasks" },
  { label: "Tags", slug: "tags" },
  { label: "Stashes", slug: "stashes" },
  { label: "Worktrees", slug: "worktrees" },
  { label: "Reflog", slug: "reflog" },
  { label: "Bisect", slug: "bisect" },
  { label: "Submodules", slug: "submodules" },
  { label: "Pipelines", slug: "pipelines", select: "feat/recurring-tasks" },
  { label: "Issues", slug: "issues", select: "export tasks to Markdown" },
  { label: "Pull Requests", slug: "pull-requests", select: "feat(recurrence)" },
  { label: "Releases", slug: "releases" },
  { label: "AI Sessions", slug: "ai-sessions" },
  { label: "Requests", slug: "requests", select: "create-task.http" },
  { label: "Settings", slug: "themes" },
];

const GL_VIEWS: ViewSpec[] = [
  { label: "Merge Requests", slug: "merge-requests", select: "feat(recurrence)" },
  { label: "Pipelines", slug: "pipelines-gitlab", select: "feat/recurring-tasks" },
];

async function settle(page: Page, ms = 600): Promise<void> {
  await page.waitForTimeout(ms);
}

async function trySelect(page: Page, select: string | string[]): Promise<void> {
  for (const text of Array.isArray(select) ? select : [select]) {
    try {
      await page.getByText(text, { exact: false }).first().click({ timeout: 2500 });
      await settle(page, 450);
    } catch {
      /* leave the empty-state pane if the row isn't found */
    }
  }
}

function runViews(host: "github" | "gitlab", project: typeof GH_PROJECT, views: ViewSpec[]): void {
  for (const mode of THEME_MODES) {
    test.describe(`marketing — ${host} — ${mode}`, () => {
      test.beforeEach(async ({ page }) => {
        await installBootstrapMocks(page, {
          mode,
          forge: host,
          activeProject: project,
          recentRepos: [{ path: project.path, name: project.name }],
          extra: commonFixtures(host),
        });
        await page.goto("/");
        await waitForAppReady(page);
        await applyTheme(page, mode);
      });

      for (const view of views) {
        test(`${view.slug}`, async ({ page }) => {
          await clickNav(page, view.label);
          await settle(page);
          if (view.clickAt) {
            await page.mouse.click(view.clickAt.x, view.clickAt.y);
            await settle(page, 500);
          }
          if (view.select) await trySelect(page, view.select);
          for (const chord of view.press
            ? Array.isArray(view.press)
              ? view.press
              : [view.press]
            : []) {
            await page.keyboard.press(chord);
            await settle(page, 500);
          }
          await page.screenshot({ path: `${OUT_DIR}/${view.slug}-${mode}.png` });
        });
      }
    });
  }
}

test.beforeAll(async () => {
  await mkdir(OUT_DIR, { recursive: true });
});

runViews("github", GH_PROJECT, GH_VIEWS);
runViews("gitlab", GL_PROJECT, GL_VIEWS);

/**
 * The views the sidebar cannot reach, and the one that only exists with no
 * project open. Each is its own test rather than a `ViewSpec` because the
 * way in is different: a palette entry, a settings category, a keyboard
 * chord plus a pushed event, and a bootstrap with no active project.
 */
for (const mode of THEME_MODES) {
  test.describe(`marketing — special views — ${mode}`, () => {
    test(`compare-${mode}`, async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        forge: "github",
        activeProject: GH_PROJECT,
        recentRepos: [{ path: GH_PROJECT.path, name: GH_PROJECT.name }],
        extra: commonFixtures("github"),
      });
      await page.goto("/");
      await waitForAppReady(page);

      // Compare has no sidebar entry; the command palette is the way in.
      await page.keyboard.press("Meta+Shift+P");
      await settle(page, 400);
      await page.keyboard.type("compare");
      await settle(page, 300);
      await page.keyboard.press("Enter");
      await settle(page);

      // Both endpoints have to be picked for the view to hold anything.
      for (const [label, ref] of [
        ["Base", "main"],
        ["Compare", "feat/recurring-tasks"],
      ] as const) {
        // getByLabel would also match the listbox, which carries the same
        // aria-label as the input it belongs to.
        const input = page.getByRole("textbox", { name: label, exact: true });
        await input.click();
        await input.fill(ref);
        await settle(page, 300);
        await page.getByRole("listbox", { name: label }).getByText(ref, { exact: true }).first().click();
        await settle(page, 400);
      }
      await settle(page);
      await page.screenshot({ path: `${OUT_DIR}/compare-${mode}.png` });
    });

    test(`settings-integrations-${mode}`, async ({ page }) => {
      await installBootstrapMocks(page, {
        mode,
        forge: "github",
        activeProject: GH_PROJECT,
        recentRepos: [{ path: GH_PROJECT.path, name: GH_PROJECT.name }],
        extra: commonFixtures("github"),
      });
      await page.goto("/");
      await waitForAppReady(page);
      await clickNav(page, "Settings");
      await settle(page);
      await trySelect(page, "Integrations");
      await settle(page);
      await page.screenshot({ path: `${OUT_DIR}/settings-integrations-${mode}.png` });
    });

    test(`welcome-${mode}`, async ({ page }) => {
      // No activeProject: this is the screen a first launch lands on.
      await installBootstrapMocks(page, {
        mode,
        forge: "github",
        recentRepos: [
          { path: "/Users/adolfo/Projects/beardgit_gh_tests", name: "beardgit_gh_tests" },
          { path: "/Users/adolfo/Projects/beardgit_glab_tests", name: "beardgit_glab_tests" },
          { path: "/Users/adolfo/Projects/tasklog", name: "tasklog" },
        ],
      });
      await page.goto("/");
      await waitForAppReady(page);
      await settle(page);
      await page.screenshot({ path: `${OUT_DIR}/welcome-${mode}.png` });
    });
  });
}

/**
 * Theme previews — the in-app editor rendered under each native theme
 * family, light + dark, using the real derived ThemeData. These back
 * the clickable theme chips in the landing's Themes section. The editor
 * view is chosen because its syntax palette varies most across themes.
 */
const THEME_BASES = ["beardgit", "fjord", "nebula"];
for (const mode of THEME_MODES) {
  test.describe(`marketing — theme previews — ${mode}`, () => {
    for (const base of THEME_BASES) {
      test(`theme-${base}-${mode}`, async ({ page }) => {
        const data = SHOWCASE_THEMES[`${base}-${mode}`];
        await installBootstrapMocks(page, {
          mode,
          forge: "github",
          activeProject: GH_PROJECT,
          recentRepos: [{ path: GH_PROJECT.path, name: GH_PROJECT.name }],
          extra: { ...commonFixtures("github"), resolve_startup_theme: data, get_theme: data },
        });
        await page.goto("/");
        await waitForAppReady(page);
        await applyTheme(page, mode);
        await clickNav(page, "Editor");
        await settle(page);
        await trySelect(page, ["src", "store.rs"]);
        await page.screenshot({ path: `${OUT_DIR}/theme-${base}-${mode}.png` });
      });
    }
  });
}
