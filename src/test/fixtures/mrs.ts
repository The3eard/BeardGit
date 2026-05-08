/**
 * Factories for Merge Request / Pull Request fixtures: Label, MrPr,
 * MrPrDetail, MrPrDiffFile, ForgeComment.
 *
 * `makeMrPrList()` returns an open + closed + merged + draft mix so a
 * single screenshot exercises every state badge and filter tab.
 */

import type {
  ForgeComment,
  Label,
  MrPr,
  MrPrDetail,
  MrPrDiffFile,
} from "../../lib/types";

export function makeLabel(overrides: Partial<Label> = {}): Label {
  return {
    name: "bug",
    color: "#d73a4a",
    description: "Something isn't working",
    ...overrides,
  };
}

export function makeMrPr(overrides: Partial<MrPr> = {}): MrPr {
  return {
    number: 42,
    title: "feat: add visual regression tests",
    state: "open",
    author: "adolfofuentes",
    source_branch: "feat/visual-tests",
    target_branch: "beta",
    url: "https://github.com/adolfofuentes/beardgit/pull/42",
    draft: false,
    labels: [makeLabel({ name: "enhancement", color: "#a2eeef", description: null })],
    reviewers: ["reviewer1"],
    created_at: "2026-05-01T10:30:00Z",
    updated_at: "2026-05-08T14:20:00Z",
    additions: 320,
    deletions: 14,
    changed_files: 12,
    base_sha: "0".repeat(40),
    head_sha: "1".repeat(40),
    head_repo_url: null,
    ...overrides,
  };
}

/** Realistic mix: open / draft / closed / merged. */
export function makeMrPrList(): MrPr[] {
  return [
    makeMrPr({
      number: 42,
      title: "feat: add visual regression tests",
      state: "open",
      labels: [makeLabel({ name: "enhancement", color: "#a2eeef", description: null })],
    }),
    makeMrPr({
      number: 41,
      title: "WIP: refactor mutation events",
      state: "open",
      draft: true,
      author: "octocat",
      source_branch: "refactor/mutations",
      labels: [makeLabel({ name: "refactor", color: "#fbca04", description: null })],
    }),
    makeMrPr({
      number: 40,
      title: "fix: graph viewport flicker on resize",
      state: "merged",
      author: "octocat",
      source_branch: "fix/graph-flicker",
      labels: [makeLabel(), makeLabel({ name: "graph", color: "#0e8a16", description: null })],
      created_at: "2026-04-22T08:00:00Z",
      updated_at: "2026-04-25T16:00:00Z",
    }),
    makeMrPr({
      number: 39,
      title: "chore: bump cargo deps",
      state: "closed",
      author: "dependabot[bot]",
      source_branch: "deps/bump-2026-04",
      labels: [makeLabel({ name: "dependencies", color: "#0366d6", description: null })],
      created_at: "2026-04-15T12:00:00Z",
      updated_at: "2026-04-18T09:30:00Z",
    }),
  ];
}

export function makeForgeComment(
  overrides: Partial<ForgeComment> = {},
): ForgeComment {
  return {
    id: 1,
    author: "reviewer1",
    body: "Looks good — one nit on the naming below.",
    created_at: "2026-05-02T11:00:00Z",
    path: null,
    line: null,
    is_review: false,
    resolvable: null,
    resolved: null,
    discussion_id: null,
    ...overrides,
  };
}

export function makeMrPrDetail(
  overrides: Partial<MrPrDetail> = {},
): MrPrDetail {
  return {
    summary: makeMrPr(),
    body:
      "## Summary\n\n- Adds Playwright visual regression suite\n- Mock IPC layer for tests against `npm run dev`\n- Component-level tests with state matrix\n\n## Test plan\n\n- [x] Local `npm run test:visual` passes\n- [x] Baselines match design",
    comments: [
      makeForgeComment({ id: 1, author: "reviewer1", body: "Looks good — one nit on the naming below." }),
      makeForgeComment({
        id: 2,
        author: "reviewer1",
        body: "Consider renaming `installMockIPC` to `mockTauriIPC` for clarity.",
        path: "tests/visual/helpers/mock-ipc.ts",
        line: 35,
        is_review: true,
      }),
      makeForgeComment({
        id: 3,
        author: "adolfofuentes",
        body: "Good call — pushed the rename.",
        created_at: "2026-05-03T09:15:00Z",
      }),
    ],
    review_status: "commented",
    mergeable: true,
    ...overrides,
  };
}

export function makeMrPrDiffFile(
  overrides: Partial<MrPrDiffFile> = {},
): MrPrDiffFile {
  return {
    path: "tests/visual/helpers/mock-ipc.ts",
    old_path: null,
    status: "A",
    additions: 142,
    deletions: 0,
    patch: null,
    ...overrides,
  };
}
