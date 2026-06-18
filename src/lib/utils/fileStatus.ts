/**
 * Canonical file-status normalisation.
 *
 * Two backend code paths emit file statuses with *different* vocabularies:
 *
 * - The working-directory status path (`git-engine::staging`) emits
 *   `"new" | "modified" | "deleted" | "renamed"`.
 * - The diff path (`git-engine::diff`, used by commit/stash/tag detail and
 *   the MR/PR diff) emits `"added" | "deleted" | "modified" | "renamed" |
 *   "copied" | "untracked"`.
 *
 * Before this helper each view hand-rolled its own switch, so the diff
 * vocabulary partly fell through to a bare "?" glyph. `normalizeFileStatus`
 * is the single source of truth: it maps either vocabulary to one canonical
 * `kind` (which drives the badge colour) plus the single-letter glyph.
 */
export type FileStatusKind =
  | "added"
  | "modified"
  | "deleted"
  | "renamed"
  | "copied"
  | "untracked"
  | "conflicted"
  | "unknown";

export interface FileStatusInfo {
  kind: FileStatusKind;
  /** Single-letter glyph shown inside the badge. */
  letter: string;
}

const STATUS_MAP: Record<string, FileStatusInfo> = {
  // Working-directory vocabulary (staging) collapses staged-add and
  // untracked into "new"; treat it as an addition.
  new: { kind: "added", letter: "A" },
  // Diff vocabulary.
  added: { kind: "added", letter: "A" },
  modified: { kind: "modified", letter: "M" },
  typechange: { kind: "modified", letter: "T" },
  deleted: { kind: "deleted", letter: "D" },
  renamed: { kind: "renamed", letter: "R" },
  copied: { kind: "copied", letter: "C" },
  untracked: { kind: "untracked", letter: "U" },
  conflicted: { kind: "conflicted", letter: "!" },
  unmerged: { kind: "conflicted", letter: "!" },
};

/**
 * Map a raw backend status string (either vocabulary, case-insensitive) to
 * its canonical kind + glyph. Genuinely unknown values return the `unknown`
 * kind with a dim "?" — distinct from the old behaviour where *known* diff
 * statuses also fell through to "?".
 */
export function normalizeFileStatus(raw: string): FileStatusInfo {
  return STATUS_MAP[raw?.toLowerCase()] ?? { kind: "unknown", letter: "?" };
}
