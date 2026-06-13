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

const COMMIT_OID = "5e8ec3ea4b29f07c3d8e6021f9a4c7b8d0e1f234";
const GOOD_OID = "80beb31c7a14e9d2f6b305a8c1e0742b9d63f5a0";

const OUT_DIR = "docs/assets/screenshots/_new";

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
    }),
    get_commit_files: [
      makeCommitFileChange({ path: "src/store.rs", status: "M" }),
      makeCommitFileChange({ path: "src/recurrence.rs", status: "A" }),
      makeCommitFileChange({ path: "tests/recurrence.rs", status: "M" }),
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
    get_diff_workdir: [makeFileDiff({ path: "src/cli.rs", status: "M" })],
    get_diff_index: [makeFileDiff({ path: "src/store.rs", status: "M" })],

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
    list_workflows: [],

    // Releases / tags
    list_releases: releaseList(),
    list_tags: tagList(),
    list_tags_paginated: tagList(),

    // Worktrees / reflog / stashes
    list_worktrees: worktreeList(),
    get_reflog: reflogList(),
    stash_entries: stashList(),

    // AI
    ai_list_conversations: aiConversationList(),
    ai_list_background_runs: [],
    list_ai_sessions: [],

    // Editor
    list_workdir_tree: workdirTree(),
    read_workdir_file: readFileResult(),
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
  /** Press a chord (e.g. command palette) after navigating. */
  press?: string;
}

const GH_VIEWS: ViewSpec[] = [
  { label: "Graph", slug: "graph" },
  { label: "Graph", slug: "command-palette", press: "Meta+Shift+P" },
  { label: "Changes", slug: "changes" },
  { label: "Editor", slug: "editor", select: ["src", "store.rs"] },
  { label: "Branches", slug: "branches" },
  { label: "Tags", slug: "tags" },
  { label: "Stashes", slug: "stashes" },
  { label: "Worktrees", slug: "worktrees" },
  { label: "Reflog", slug: "reflog" },
  { label: "Bisect", slug: "bisect" },
  { label: "Pipelines", slug: "pipelines", select: "feat/recurring-tasks" },
  { label: "Issues", slug: "issues", select: "export tasks to Markdown" },
  { label: "Pull Requests", slug: "pull-requests", select: "feat(recurrence)" },
  { label: "Releases", slug: "releases" },
  { label: "AI Sessions", slug: "ai-sessions" },
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
          if (view.select) await trySelect(page, view.select);
          if (view.press) {
            await page.keyboard.press(view.press);
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
