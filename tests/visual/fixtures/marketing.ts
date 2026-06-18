/**
 * Marketing-screenshot fixtures — real data lifted from the two
 * prepared test repos so the landing-page captures show authentic
 * content instead of synthetic placeholders:
 *
 *   GitHub → The3eard/beardgit_gh_tests   (~/Projects/beardgit_gh_tests)
 *   GitLab → The3eard/beardgit_glab_tests (~/Projects/beardgit_glab_tests)
 *
 * Both repos are the same "tasklog" Rust CLI project (mirrored), so the
 * commit history, branches, PR/MR titles, issues and releases below are
 * the real ones pulled with `gh` / `glab` / `git` on 2026-06-13.
 *
 * These feed `installBootstrapMocks(page, { extra })` in
 * `marketing.spec.ts`; the shapes mirror the factories in
 * `src/test/fixtures/` exactly.
 */

import type {
  AiConversation,
  BranchInfo,
  CiRun,
  CiRunDetail,
  FileStatus,
  GraphViewport,
  Issue,
  IssueDetail,
  Label,
  LaneSegment,
  LayoutNode,
  MergeCurve,
  MrPr,
  MrPrDetail,
  MrPrDiffFile,
  ProjectInfo,
  ReadWorkdirFileResult,
  ReflogEntry,
  Release,
  StashEntry,
  TagInfo,
  WorkdirTreeEntry,
  WorktreeInfo,
} from "../../../src/lib/types";
import {
  makeCiRunDetail,
  makeForgeComment,
  makeIssueDetail,
  makeMrPrDetail,
  makeProjectInfo,
} from "../../../src/test/fixtures";

const oid = (s: number): string => s.toString(16).padStart(40, "0");

const GH_PATH = "/Users/adolfo/Projects/beardgit_gh_tests";
const GL_PATH = "/Users/adolfo/Projects/beardgit_glab_tests";

export const GH_PROJECT: ProjectInfo = makeProjectInfo({
  path: GH_PATH,
  name: "beardgit_gh_tests",
  head_branch: "main",
  change_count: 0,
});

export const GL_PROJECT: ProjectInfo = makeProjectInfo({
  path: GL_PATH,
  name: "beardgit_glab_tests",
  head_branch: "main",
  change_count: 0,
});

function label(name: string, color: string, description: string | null = null): Label {
  return { name, color, description };
}

/** Real branches (both repos share the same ref set). */
export function branchList(): BranchInfo[] {
  return [
    { name: "main", is_head: true, is_remote: false, oid: oid(0xdde29e2), upstream: "origin/main", ahead: 0, behind: 0 },
    { name: "feat/tui-dashboard", is_head: false, is_remote: false, oid: oid(0x8636791), upstream: "origin/feat/tui-dashboard", ahead: 3, behind: 1 },
    { name: "feat/recurring-tasks", is_head: false, is_remote: false, oid: oid(0x5e8ec3e), upstream: "origin/feat/recurring-tasks", ahead: 4, behind: 1 },
    { name: "fix/parser-edge-case", is_head: false, is_remote: false, oid: oid(0xdde29e2), upstream: "origin/fix/parser-edge-case", ahead: 1, behind: 0 },
    { name: "chore/clippy-cleanup", is_head: false, is_remote: false, oid: oid(0xd7319fb), upstream: null, ahead: 1, behind: 2 },
    { name: "origin/main", is_head: false, is_remote: true, oid: oid(0xdde29e2), upstream: null, ahead: 0, behind: 0 },
  ];
}

/** Real working-tree changes — a believable mid-edit state for the tasklog crate. */
export function fileStatusList(): FileStatus[] {
  return [
    { path: "src/store.rs", status: "M", is_staged: true },
    { path: "src/recurrence.rs", status: "A", is_staged: true },
    { path: "src/cli.rs", status: "M", is_staged: false },
    { path: "src/main.rs", status: "M", is_staged: false },
    { path: "tests/recurrence.rs", status: "A", is_staged: false },
    { path: "README.md", status: "M", is_staged: false },
  ];
}

/** Real GitHub PRs (`gh pr list --state all`). */
export function prList(): MrPr[] {
  const base = {
    target_branch: "main",
    head_repo_url: null,
    base_sha: "0".repeat(40),
    head_sha: "1".repeat(40),
  };
  return [
    {
      ...base,
      number: 9,
      title: "feat(recurrence): repeating tasks via --repeat daily|weekly|monthly",
      state: "open",
      author: "adolfofuentes",
      source_branch: "feat/recurring-tasks",
      url: "https://github.com/The3eard/beardgit_gh_tests/pull/9",
      draft: false,
      labels: [label("enhancement", "#a2eeef")],
      reviewers: ["octocat"],
      created_at: "2026-04-27T19:26:28Z",
      updated_at: "2026-04-27T19:27:00Z",
      additions: 214,
      deletions: 9,
      changed_files: 5,
    },
    {
      ...base,
      number: 10,
      title: "fix(store): reject empty/whitespace-only titles in 'tasklog add'",
      state: "open",
      author: "adolfofuentes",
      source_branch: "fix/parser-edge-case",
      url: "https://github.com/The3eard/beardgit_gh_tests/pull/10",
      draft: true,
      labels: [label("bug", "#d73a4a")],
      reviewers: [],
      created_at: "2026-04-27T19:27:34Z",
      updated_at: "2026-04-27T19:28:00Z",
      additions: 38,
      deletions: 4,
      changed_files: 2,
    },
    {
      ...base,
      number: 11,
      title: "chore(lints): codify CI lint rules in Cargo.toml [lints] tables",
      state: "merged",
      author: "adolfofuentes",
      source_branch: "chore/clippy-cleanup",
      url: "https://github.com/The3eard/beardgit_gh_tests/pull/11",
      draft: false,
      labels: [label("chore", "#fbca04")],
      reviewers: ["octocat"],
      created_at: "2026-04-27T19:27:52Z",
      updated_at: "2026-04-27T19:29:00Z",
      additions: 12,
      deletions: 0,
      changed_files: 1,
    },
  ];
}

/** Real GitLab MRs (`glab mr list --all`) — same project, MR flavour. */
export function mrList(): MrPr[] {
  return prList().map((pr) => ({
    ...pr,
    url: pr.url.replace("github.com", "gitlab.com").replace("/pull/", "/-/merge_requests/"),
  }));
}

/** Real GitHub issues (`gh issue list --state all`). */
export function issueList(): Issue[] {
  const mk = (o: Partial<Issue>): Issue => ({
    number: 0,
    title: "",
    state: "open",
    author: "adolfofuentes",
    labels: [],
    assignees: [],
    milestone: null,
    comments_count: 0,
    created_at: "2026-04-27T19:24:00Z",
    updated_at: "2026-04-27T19:24:00Z",
    url: "https://github.com/The3eard/beardgit_gh_tests/issues/1",
    ...o,
  });
  return [
    mk({ number: 1, title: "list output misaligns when titles exceed terminal width", labels: [label("bug", "#d73a4a")], comments_count: 2 }),
    mk({ number: 2, title: "feat: archive completed tasks instead of deleting them", labels: [label("enhancement", "#a2eeef")], comments_count: 1 }),
    mk({ number: 3, title: "docs: README is missing a --repeat example once feat/recurring-tasks lands", labels: [label("documentation", "#0075ca")] }),
    mk({ number: 4, title: "feat: export tasks to Markdown for weekly review", labels: [label("enhancement", "#a2eeef")], comments_count: 4 }),
    mk({ number: 5, title: "good first issue: --no-color flag to disable ANSI escapes", labels: [label("enhancement", "#a2eeef"), label("good first issue", "#7057ff")] }),
    mk({ number: 6, title: "bug: 0.3.0 storage migration leaves a stale ~/.tasklog.json behind", labels: [label("bug", "#d73a4a")], comments_count: 3 }),
    mk({ number: 7, title: "done now panics when given an unknown task id", state: "closed", labels: [label("bug", "#d73a4a")], comments_count: 5, assignees: ["adolfofuentes"] }),
    mk({ number: 8, title: "Move tasks.json off ~/.tasklog.json onto platform-correct paths", state: "closed", labels: [label("enhancement", "#a2eeef")], comments_count: 6, assignees: ["adolfofuentes"] }),
  ];
}

/**
 * CI runs — GitHub Actions had no recorded runs and the GitLab token
 * lacks pipeline scope, so these are modelled on the real pipeline
 * defined in each repo's `.gitlab-ci.yml` / `.github/workflows`
 * (fmt → clippy → test → build stages over the real branches).
 */
export function ciRunList(host: "github" | "gitlab"): CiRun[] {
  const isGh = host === "github";
  const url = (id: number) =>
    isGh
      ? `https://github.com/The3eard/beardgit_gh_tests/actions/runs/${id}`
      : `https://gitlab.com/The3eard/beardgit_glab_tests/-/pipelines/${id}`;
  return [
    { id: 5021, display_id: 5021, status: "success", ref_name: "main", sha: oid(0xdde29e2), source: "push", name: "CI", actor: "adolfofuentes", created_at: "2026-04-27T19:29:00Z", updated_at: "2026-04-27T19:33:40Z", web_url: url(5021) },
    { id: 5020, display_id: 5020, status: "running", ref_name: "feat/recurring-tasks", sha: oid(0x5e8ec3e), source: "push", name: "CI", actor: "adolfofuentes", created_at: "2026-04-27T19:26:30Z", updated_at: "2026-04-27T19:28:10Z", web_url: url(5020) },
    { id: 5019, display_id: 5019, status: "failed", ref_name: "fix/parser-edge-case", sha: oid(0xdde29e2), source: "push", name: "CI", actor: "adolfofuentes", created_at: "2026-04-27T19:27:30Z", updated_at: "2026-04-27T19:31:00Z", web_url: url(5019) },
    { id: 5018, display_id: 5018, status: "success", ref_name: "feat/tui-dashboard", sha: oid(0x8636791), source: "push", name: "CI", actor: "adolfofuentes", created_at: "2026-04-27T19:20:00Z", updated_at: "2026-04-27T19:24:20Z", web_url: url(5018) },
    { id: 5017, display_id: 5017, status: "success", ref_name: "chore/clippy-cleanup", sha: oid(0xd7319fb), source: "push", name: "CI", actor: "adolfofuentes", created_at: "2026-04-27T19:15:00Z", updated_at: "2026-04-27T19:19:10Z", web_url: url(5017) },
  ];
}

/** Detail for the most recent CI run (real branch/sha, fmt→clippy→test→build stages). */
export function ciRunDetail(host: "github" | "gitlab"): CiRunDetail {
  return makeCiRunDetail({ run: ciRunList(host)[0] });
}

/** Detail for PR/MR #9 — the recurring-tasks feature, with a real-looking review thread. */
export function prDetail(): MrPrDetail {
  return makeMrPrDetail({
    summary: prList()[0],
    body:
      "## Summary\n\nAdds `--repeat daily|weekly|monthly` to `tasklog add`. When a repeating task is marked done, the next occurrence rolls forward automatically.\n\n## Changes\n\n- New `Recurrence` enum (daily / weekly / monthly)\n- `mark_done` rolls repeating tasks forward instead of closing them\n- Covered monthly rollover at month boundaries\n\n## Test plan\n\n- [x] `cargo test recurrence`\n- [x] Manual: `tasklog add \"standup\" --repeat daily`",
    comments: [
      makeForgeComment({ id: 1, author: "octocat", body: "Nice — does this handle the Jan 31 → Feb 28 monthly rollover?" }),
      makeForgeComment({ id: 2, author: "adolfofuentes", body: "Yep, covered in `test(recurrence): cover monthly rollover via mark_done`.", created_at: "2026-04-27T19:27:40Z" }),
      makeForgeComment({ id: 3, author: "octocat", body: "Clamp the day to the last valid day of the target month here.", path: "src/recurrence.rs", line: 48, is_review: true, created_at: "2026-04-27T19:28:10Z" }),
    ],
    review_status: "commented",
    mergeable: true,
  });
}

export function prDiff(): MrPrDiffFile[] {
  return [
    { path: "src/recurrence.rs", old_path: null, status: "A", additions: 96, deletions: 0, patch: null },
    { path: "src/store.rs", old_path: null, status: "M", additions: 31, deletions: 9, patch: null },
    { path: "src/cli.rs", old_path: null, status: "M", additions: 12, deletions: 0, patch: null },
    { path: "tests/recurrence.rs", old_path: null, status: "A", additions: 75, deletions: 0, patch: null },
  ];
}

/** Detail for issue #4 — export to Markdown. */
export function issueDetail(): IssueDetail {
  return makeIssueDetail({
    summary: issueList()[3],
    body:
      "It'd be handy to dump open tasks to a Markdown checklist for a weekly review.\n\n```\ntasklog export --format md > week.md\n```\n\n## Acceptance\n\n- [ ] `--format md` flag\n- [ ] groups by tag\n- [ ] open tasks as `- [ ]`, done as `- [x]`",
    comments: [
      makeForgeComment({ id: 1, author: "octocat", body: "+1 — I'd pipe this straight into my notes." }),
      makeForgeComment({ id: 2, author: "adolfofuentes", body: "Leaning toward reusing the `list --tag` grouping for this.", created_at: "2026-04-27T19:25:00Z" }),
    ],
  });
}

/**
 * Hand-authored commit graph for the tasklog repo: a linear release
 * mainline (lane 0) with two feature branches (lanes 1–2) that fan out
 * and merge back, plus a long-lived chore branch. Summaries are the
 * real ones from `git log --all`.
 */
export function graphViewport(): GraphViewport {
  const n = (i: number, lane: number, summary: string, refs: string[] = [], is_merge = false): LayoutNode => ({
    oid: oid(0x1000 + i),
    lane,
    row: i,
    refs,
    summary,
    author: "Adolfo Fuentes",
    email: "adolfo@example.com",
    timestamp: 1745780000 - i * 5400,
    is_merge,
    is_root: false,
    segment_group: lane,
  });

  const nodes: LayoutNode[] = [
    n(0, 0, "Merge branch 'feat/recurring-tasks'", ["HEAD", "refs/heads/main", "refs/remotes/origin/main"], true),
    n(1, 1, "fix(store): derive Debug on MarkDoneOutcome"),
    n(2, 1, "test(recurrence): cover monthly rollover via mark_done"),
    n(3, 1, "feat(recurrence): roll repeating tasks forward when marked done"),
    n(4, 1, "feat(recurrence): introduce Recurrence enum (daily/weekly/monthly)", ["refs/heads/feat/recurring-tasks"]),
    n(5, 2, "feat(tui): scaffold ratatui-based 'tasklog tui' dashboard", ["refs/heads/feat/tui-dashboard"]),
    n(6, 0, "ci(gitlab): mirror the GitHub Actions pipeline as .gitlab-ci.yml"),
    n(7, 0, "chore: appease clippy::field_reassign_with_default"),
    n(8, 0, "release: 0.3.0 — search, tag filter, platform-correct storage", ["refs/tags/v0.3.0"]),
    n(9, 0, "feat(cli): list --tag <name> to narrow output to a single context"),
    n(10, 0, "feat(cli): add 'tasklog search' for case-insensitive title+tag lookup"),
    n(11, 0, "feat(store): move tasks.json to platform-correct paths via directories"),
    n(12, 0, "release: 0.2.0 — tagging, due dates, friendlier errors", ["refs/tags/v0.2.0"]),
    n(13, 0, "feat(task): add optional tag and due date with chrono timestamps"),
    n(14, 0, "release: 0.1.0", ["refs/tags/v0.1.0"]),
  ];

  const lane_segments: LaneSegment[] = [
    { lane: 0, start_row: 0, end_row: 14, color_index: 0, recycled: false, sync_state: "Synced", group_id: 0 },
    { lane: 1, start_row: 0, end_row: 4, color_index: 1, recycled: false, sync_state: "LocalOnly", group_id: 1 },
    { lane: 2, start_row: 5, end_row: 8, color_index: 2, recycled: false, sync_state: "LocalOnly", group_id: 2 },
  ];

  const merge_curves: MergeCurve[] = [
    { from_lane: 1, from_row: 1, to_lane: 0, to_row: 0, color_index: 1, group_id: 1 },
    { from_lane: 0, from_row: 8, to_lane: 2, to_row: 5, color_index: 2, group_id: 2 },
  ];

  return {
    nodes,
    lane_segments,
    merge_curves,
    total_count: nodes.length,
    offset: 0,
    visible_lane_count: 3,
    total_lane_count: 3,
    head_lane: 0,
    has_more: false,
  };
}

/** Real releases (`gh release list`). */
export function releaseList(): Release[] {
  const url = (t: string) => `https://github.com/The3eard/beardgit_gh_tests/releases/tag/${t}`;
  return [
    { tag: "v0.3.0", name: "tasklog 0.3.0", state: "published", author: "adolfofuentes", created_at: "2026-04-27T19:29:35Z", published_at: "2026-04-27T19:29:35Z", asset_count: 3, url: url("v0.3.0") },
    { tag: "v0.2.0", name: "tasklog 0.2.0", state: "published", author: "adolfofuentes", created_at: "2026-04-27T19:29:33Z", published_at: "2026-04-27T19:29:33Z", asset_count: 3, url: url("v0.2.0") },
    { tag: "v0.1.0", name: "tasklog 0.1.0", state: "published", author: "adolfofuentes", created_at: "2026-04-27T19:29:33Z", published_at: "2026-04-27T19:29:33Z", asset_count: 2, url: url("v0.1.0") },
  ];
}

/** Tags matching the real releases. */
export function tagList(): TagInfo[] {
  const mk = (name: string, seed: number, msg: string, date: string): TagInfo => ({
    name,
    object_oid: oid(seed),
    commit_oid: oid(seed),
    annotated: true,
    message: msg,
    tagger_name: "Adolfo Fuentes",
    tagger_email: "adolfo@example.com",
    date,
  });
  return [
    mk("v0.3.0", 0x4213da0, "tasklog 0.3.0 — search, tag filter, platform-correct storage", "2026-04-27T19:29:35Z"),
    mk("v0.2.0", 0x80beb31, "tasklog 0.2.0 — tagging, due dates, friendlier errors", "2026-04-20T11:00:00Z"),
    mk("v0.1.0", 0x0fc526c, "tasklog 0.1.0 — first cut", "2026-04-12T09:00:00Z"),
  ];
}

export function worktreeList(): WorktreeInfo[] {
  return [
    { path: "/Users/adolfo/Projects/beardgit_gh_tests", branch: "main", head_oid: oid(0xdde29e2), is_main: true, is_locked: false },
    { path: "/Users/adolfo/Projects/beardgit_gh_tests/.worktrees/ai-claude_code-recurring", branch: "ai/claude_code/recurring-tasks", head_oid: oid(0x5e8ec3e), is_main: false, is_locked: false },
    { path: "/Users/adolfo/Projects/beardgit_gh_tests/.worktrees/tui", branch: "feat/tui-dashboard", head_oid: oid(0x8636791), is_main: false, is_locked: false },
  ];
}

export function reflogList(): ReflogEntry[] {
  const e = (i: number, action: string, summary: string): ReflogEntry => ({
    oid: oid(0x2000 + i),
    prev_oid: oid(0x2000 + i + 1),
    action,
    summary,
    author: "Adolfo Fuentes",
    email: "adolfo@example.com",
    timestamp: 1745780000 - i * 3600,
  });
  return [
    e(0, "merge feat/recurring-tasks", "Merge branch 'feat/recurring-tasks'"),
    e(1, "commit", "fix(store): derive Debug on MarkDoneOutcome"),
    e(2, "checkout", "moving from main to feat/recurring-tasks"),
    e(3, "commit", "feat(recurrence): roll repeating tasks forward when marked done"),
    e(4, "pull", "Fast-forward"),
    e(5, "reset", "moving to HEAD~1"),
    e(6, "commit", "release: 0.3.0 — search, tag filter, platform-correct storage"),
  ];
}

export function stashList(): StashEntry[] {
  return [
    { index: 0, message: "WIP on main: experiment with --json output", branch: "main", timestamp: 1745779000, oid: oid(0x3001) },
    { index: 1, message: "On feat/tui-dashboard: scratch keybinding map", branch: "feat/tui-dashboard", timestamp: 1745700000, oid: oid(0x3002) },
  ];
}

export function aiConversationList(): AiConversation[] {
  return [
    { id: "a1b2c3d4", provider: "claude_code", cwd: "/Users/adolfo/Projects/beardgit_gh_tests", created_at: 1745778000000, last_activity_at: 1745779500000, title: "Add --repeat flag and roll recurring tasks forward on done" },
    { id: "e5f6a7b8", provider: "codex", cwd: "/Users/adolfo/Projects/beardgit_gh_tests", created_at: 1745700000000, last_activity_at: 1745705000000, title: "Scaffold a ratatui dashboard for tasklog tui" },
    { id: "c9d0e1f2", provider: "claude_code", cwd: "/Users/adolfo/Projects/beardgit_gh_tests", created_at: 1745600000000, last_activity_at: 1745602000000, title: "Move tasks.json onto platform-correct paths via directories crate" },
  ];
}

/** A small, believable file tree for the editor view. */
export function workdirTree(): WorkdirTreeEntry[] {
  const f = (path: string, size: number): WorkdirTreeEntry => ({ path, name: path.split("/").pop()!, is_directory: false, size });
  const d = (path: string): WorkdirTreeEntry => ({ path, name: path.split("/").pop()!, is_directory: true, size: null });
  return [
    d("src"),
    f("src/main.rs", 1840),
    f("src/cli.rs", 4210),
    f("src/store.rs", 6320),
    f("src/recurrence.rs", 2980),
    f("src/task.rs", 3110),
    d("tests"),
    f("tests/recurrence.rs", 2440),
    f("Cargo.toml", 612),
    f("README.md", 3870),
    f("CHANGELOG.md", 2150),
  ];
}

export function readFileResult(): ReadWorkdirFileResult {
  const data = `use crate::recurrence::Recurrence;
use crate::task::Task;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// On-disk task store, persisted as JSON under the platform data dir.
pub struct Store {
    path: PathBuf,
    tasks: Vec<Task>,
}

impl Store {
    /// Add a task, rejecting empty/whitespace-only titles.
    pub fn add(&mut self, title: &str, repeat: Option<Recurrence>) -> Result<u32> {
        let title = title.trim();
        if title.is_empty() {
            anyhow::bail!("task title must not be empty");
        }
        let id = self.next_id();
        self.tasks.push(Task::new(id, title, repeat));
        self.flush().context("failed to persist task store")?;
        Ok(id)
    }

    /// Mark a task done; roll repeating tasks forward to the next due date.
    pub fn mark_done(&mut self, id: u32) -> Result<MarkDoneOutcome> {
        let task = self.get_mut(id)?;
        match task.repeat {
            Some(rec) => {
                task.due = rec.next(task.due);
                Ok(MarkDoneOutcome::Rolled(task.due))
            }
            None => {
                task.done = true;
                Ok(MarkDoneOutcome::Closed)
            }
        }
    }
}
`;
  return { kind: "text", data, size: data.length };
}
