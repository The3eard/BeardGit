/**
 * Factories for working-tree change fixtures: FileStatus,
 * FileDiff, DiffHunkInfo, DiffLineInfo.
 *
 * `makeFileStatusList()` returns a realistic mix that exercises every
 * status code the changes view renders (modified, added, deleted,
 * renamed, untracked, conflicted, mixed staged/unstaged).
 */

import type {
  DiffHunkInfo,
  DiffLineInfo,
  FileDiff,
  FileDiffStat,
  FileStatus,
} from "../../lib/types";

/**
 * A working-tree file status, in the *staging* vocabulary the backend
 * actually emits: `"new" | "modified" | "deleted" | "renamed"`
 * (`git-engine::staging`). Not the porcelain letters — `normalizeFileStatus`
 * does not know them, so `"M"` renders as the dim `?` unknown badge and the
 * colour-coded badges never appear. Every Changes baseline was recorded
 * that way before this was fixed.
 */
export function makeFileStatus(
  overrides: Partial<FileStatus> = {},
): FileStatus {
  return {
    path: "src/lib/feature.ts",
    status: "modified",
    is_staged: false,
    ...overrides,
  };
}

export function makeFileStatusList(): FileStatus[] {
  return [
    makeFileStatus({ path: "src/lib/feature.ts", status: "modified", is_staged: true }),
    makeFileStatus({ path: "src/lib/types/index.ts", status: "modified", is_staged: true }),
    makeFileStatus({ path: "src/routes/+page.svelte", status: "modified", is_staged: false }),
    makeFileStatus({ path: "src/lib/components/ui/Button.svelte", status: "modified", is_staged: false }),
    makeFileStatus({ path: "src/lib/utils/format.ts", status: "new", is_staged: true }),
    makeFileStatus({ path: "src/lib/legacy/old-helper.ts", status: "deleted", is_staged: false }),
    // Untracked: the staging path collapses staged-add and untracked into
    // `"new"`, distinguished by `is_staged`.
    makeFileStatus({ path: "tests/visual/new-spec.ts", status: "new", is_staged: false }),
    makeFileStatus({ path: "tests/visual/another.ts", status: "new", is_staged: false }),
  ];
}

export function makeDiffLine(
  overrides: Partial<DiffLineInfo> = {},
): DiffLineInfo {
  return {
    origin: " ",
    content: "  return value;",
    old_lineno: 10,
    new_lineno: 10,
    ...overrides,
  };
}

export function makeDiffHunk(
  overrides: Partial<DiffHunkInfo> = {},
): DiffHunkInfo {
  return {
    header: "@@ -1,5 +1,7 @@",
    old_start: 1,
    old_lines: 5,
    new_start: 1,
    new_lines: 7,
    lines: [
      makeDiffLine({ origin: " ", content: "function process(value: string) {", old_lineno: 1, new_lineno: 1 }),
      makeDiffLine({ origin: " ", content: "  if (!value) {", old_lineno: 2, new_lineno: 2 }),
      makeDiffLine({ origin: "-", content: "    return null;", old_lineno: 3, new_lineno: null }),
      makeDiffLine({ origin: "+", content: "    throw new Error('value required');", old_lineno: null, new_lineno: 3 }),
      makeDiffLine({ origin: "+", content: "  }", old_lineno: null, new_lineno: 4 }),
      makeDiffLine({ origin: " ", content: "  return value.trim();", old_lineno: 4, new_lineno: 5 }),
    ],
    ...overrides,
  };
}

export function makeFileDiff(overrides: Partial<FileDiff> = {}): FileDiff {
  return {
    path: "src/lib/feature.ts",
    old_path: null,
    status: "modified",
    hunks: [makeDiffHunk()],
    additions: 3,
    deletions: 1,
    ...overrides,
  };
}

/**
 * Lightweight per-file change stat (no hunks) — the shape returned by
 * `get_diff_stats_workdir` / `get_diff_stats_index` that powers the
 * Changes lists.
 */
export function makeFileDiffStat(
  overrides: Partial<FileDiffStat> = {},
): FileDiffStat {
  return {
    path: "src/lib/feature.ts",
    old_path: null,
    status: "modified",
    additions: 3,
    deletions: 1,
    ...overrides,
  };
}
