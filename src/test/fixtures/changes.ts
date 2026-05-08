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
  FileStatus,
} from "../../lib/types";

export function makeFileStatus(
  overrides: Partial<FileStatus> = {},
): FileStatus {
  return {
    path: "src/lib/feature.ts",
    status: "M",
    is_staged: false,
    ...overrides,
  };
}

export function makeFileStatusList(): FileStatus[] {
  return [
    makeFileStatus({ path: "src/lib/feature.ts", status: "M", is_staged: true }),
    makeFileStatus({ path: "src/lib/types/index.ts", status: "M", is_staged: true }),
    makeFileStatus({ path: "src/routes/+page.svelte", status: "M", is_staged: false }),
    makeFileStatus({ path: "src/lib/components/ui/Button.svelte", status: "M", is_staged: false }),
    makeFileStatus({ path: "src/lib/utils/format.ts", status: "A", is_staged: true }),
    makeFileStatus({ path: "src/lib/legacy/old-helper.ts", status: "D", is_staged: false }),
    makeFileStatus({ path: "tests/visual/new-spec.ts", status: "?", is_staged: false }),
    makeFileStatus({ path: "tests/visual/another.ts", status: "?", is_staged: false }),
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
    status: "M",
    hunks: [makeDiffHunk()],
    additions: 3,
    deletions: 1,
    ...overrides,
  };
}
