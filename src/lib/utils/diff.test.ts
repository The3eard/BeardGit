import { describe, it, expect } from 'vitest';
import { fileDiffToContents } from './diff';
import type { FileDiff, DiffHunkInfo } from '$lib/types';

/** Build a minimal FileDiff fixture for test use. */
function makeDiff(hunks: DiffHunkInfo[]): FileDiff {
  return {
    path: 'test.ts',
    old_path: null,
    status: 'Modified',
    hunks,
    additions: 0,
    deletions: 0,
  };
}

describe('fileDiffToContents', () => {
  it('empty diff (no hunks) returns empty strings on both sides', () => {
    const result = fileDiffToContents(makeDiff([]));
    expect(result.oldContent).toBe('');
    expect(result.newContent).toBe('');
  });

  it('context-only diff produces identical old and new content', () => {
    const diff = makeDiff([
      {
        header: '@@ -1,3 +1,3 @@',
        old_start: 1,
        old_lines: 3,
        new_start: 1,
        new_lines: 3,
        lines: [
          { origin: ' ', content: 'line one', old_lineno: 1, new_lineno: 1 },
          { origin: ' ', content: 'line two', old_lineno: 2, new_lineno: 2 },
          { origin: ' ', content: 'line three', old_lineno: 3, new_lineno: 3 },
        ],
      },
    ]);
    const { oldContent, newContent } = fileDiffToContents(diff);
    expect(oldContent).toBe('line one\nline two\nline three');
    expect(newContent).toBe('line one\nline two\nline three');
  });

  it('added lines only — old is empty, new has all lines', () => {
    const diff = makeDiff([
      {
        header: '@@ -0,0 +1,2 @@',
        old_start: 0,
        old_lines: 0,
        new_start: 1,
        new_lines: 2,
        lines: [
          { origin: '+', content: 'new line 1', old_lineno: null, new_lineno: 1 },
          { origin: '+', content: 'new line 2', old_lineno: null, new_lineno: 2 },
        ],
      },
    ]);
    const { oldContent, newContent } = fileDiffToContents(diff);
    expect(oldContent).toBe('');
    expect(newContent).toBe('new line 1\nnew line 2');
  });

  it('removed lines only — old has all lines, new is empty', () => {
    const diff = makeDiff([
      {
        header: '@@ -1,2 +0,0 @@',
        old_start: 1,
        old_lines: 2,
        new_start: 0,
        new_lines: 0,
        lines: [
          { origin: '-', content: 'old line 1', old_lineno: 1, new_lineno: null },
          { origin: '-', content: 'old line 2', old_lineno: 2, new_lineno: null },
        ],
      },
    ]);
    const { oldContent, newContent } = fileDiffToContents(diff);
    expect(oldContent).toBe('old line 1\nold line 2');
    expect(newContent).toBe('');
  });

  it('mixed add / remove / context — lines go to correct side', () => {
    const diff = makeDiff([
      {
        header: '@@ -1,3 +1,3 @@',
        old_start: 1,
        old_lines: 3,
        new_start: 1,
        new_lines: 3,
        lines: [
          { origin: ' ', content: 'context', old_lineno: 1, new_lineno: 1 },
          { origin: '-', content: 'removed', old_lineno: 2, new_lineno: null },
          { origin: '+', content: 'added', old_lineno: null, new_lineno: 2 },
          { origin: ' ', content: 'more context', old_lineno: 3, new_lineno: 3 },
        ],
      },
    ]);
    const { oldContent, newContent } = fileDiffToContents(diff);
    expect(oldContent).toBe('context\nremoved\nmore context');
    expect(newContent).toBe('context\nadded\nmore context');
  });

  it('multiple hunks — lines from all hunks are assembled in order', () => {
    const diff = makeDiff([
      {
        header: '@@ -1,1 +1,1 @@',
        old_start: 1,
        old_lines: 1,
        new_start: 1,
        new_lines: 1,
        lines: [
          { origin: '-', content: 'hunk1 old', old_lineno: 1, new_lineno: null },
          { origin: '+', content: 'hunk1 new', old_lineno: null, new_lineno: 1 },
        ],
      },
      {
        header: '@@ -10,1 +10,1 @@',
        old_start: 10,
        old_lines: 1,
        new_start: 10,
        new_lines: 1,
        lines: [
          { origin: '-', content: 'hunk2 old', old_lineno: 10, new_lineno: null },
          { origin: '+', content: 'hunk2 new', old_lineno: null, new_lineno: 10 },
        ],
      },
    ]);
    const { oldContent, newContent } = fileDiffToContents(diff);
    expect(oldContent).toBe('hunk1 old\nhunk2 old');
    expect(newContent).toBe('hunk1 new\nhunk2 new');
  });

  it('single-line change — one line in old, one line in new', () => {
    const diff = makeDiff([
      {
        header: '@@ -1,1 +1,1 @@',
        old_start: 1,
        old_lines: 1,
        new_start: 1,
        new_lines: 1,
        lines: [
          { origin: '-', content: 'before', old_lineno: 1, new_lineno: null },
          { origin: '+', content: 'after', old_lineno: null, new_lineno: 1 },
        ],
      },
    ]);
    const { oldContent, newContent } = fileDiffToContents(diff);
    expect(oldContent).toBe('before');
    expect(newContent).toBe('after');
  });
});
