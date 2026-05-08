/**
 * Factories for Issues fixtures: Milestone, Issue, IssueDetail.
 *
 * `makeIssueList()` returns an open/closed mix with varied label counts
 * and assignees so a single screenshot exercises the full row layout.
 */

import type { Issue, IssueDetail, Milestone } from "../../lib/types";
import { makeForgeComment, makeLabel } from "./mrs";

export function makeMilestone(
  overrides: Partial<Milestone> = {},
): Milestone {
  return {
    id: 7,
    title: "v0.2.0 — Visual test infra",
    state: "open",
    due_on: "2026-06-15",
    ...overrides,
  };
}

export function makeIssue(overrides: Partial<Issue> = {}): Issue {
  return {
    number: 128,
    title: "Graph: lanes flicker when resizing the right pane",
    state: "open",
    author: "octocat",
    labels: [makeLabel({ name: "bug", color: "#d73a4a", description: null })],
    assignees: ["adolfofuentes"],
    milestone: makeMilestone(),
    comments_count: 3,
    created_at: "2026-04-30T08:00:00Z",
    updated_at: "2026-05-04T14:30:00Z",
    url: "https://github.com/adolfofuentes/beardgit/issues/128",
    ...overrides,
  };
}

export function makeIssueList(): Issue[] {
  return [
    makeIssue({
      number: 128,
      title: "Graph: lanes flicker when resizing the right pane",
      state: "open",
      labels: [makeLabel({ name: "bug", color: "#d73a4a", description: null })],
      comments_count: 3,
    }),
    makeIssue({
      number: 127,
      title: "Add dark-mode toggle to the welcome screen",
      state: "open",
      author: "adolfofuentes",
      assignees: [],
      labels: [
        makeLabel({ name: "enhancement", color: "#a2eeef", description: null }),
        makeLabel({ name: "ux", color: "#bfdadc", description: null }),
      ],
      milestone: null,
      comments_count: 0,
    }),
    makeIssue({
      number: 126,
      title: "Settings page can't render on systems without Fira Code",
      state: "open",
      author: "user42",
      assignees: ["adolfofuentes", "reviewer1"],
      labels: [
        makeLabel({ name: "bug", color: "#d73a4a", description: null }),
        makeLabel({ name: "needs-triage", color: "#cccccc", description: null }),
      ],
      milestone: null,
      comments_count: 7,
    }),
    makeIssue({
      number: 119,
      title: "Add support for SSH agent on Linux",
      state: "closed",
      author: "octocat",
      assignees: ["adolfofuentes"],
      labels: [makeLabel({ name: "enhancement", color: "#a2eeef", description: null })],
      milestone: null,
      comments_count: 12,
      created_at: "2026-04-10T10:00:00Z",
      updated_at: "2026-04-20T18:00:00Z",
    }),
  ];
}

export function makeIssueDetail(
  overrides: Partial<IssueDetail> = {},
): IssueDetail {
  return {
    summary: makeIssue(),
    body:
      "## Steps to reproduce\n\n1. Open a repo with > 200 commits.\n2. Drag the right pane divider.\n3. Observe lanes flickering between renders.\n\n## Expected\n\nLane positions stable across the resize gesture.",
    comments: [
      makeForgeComment({ id: 1, author: "octocat", body: "I can repro on macOS 25.4." }),
      makeForgeComment({
        id: 2,
        author: "adolfofuentes",
        body: "Looking at it — likely the lane assignment is re-running on every resize tick.",
        created_at: "2026-05-02T15:30:00Z",
      }),
      makeForgeComment({
        id: 3,
        author: "adolfofuentes",
        body: "Fixed in #135.",
        created_at: "2026-05-04T09:00:00Z",
      }),
    ],
    ...overrides,
  };
}
