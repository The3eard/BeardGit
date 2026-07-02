/**
 * Pure path helpers for the file-editor PathDialog.
 *
 * Kept framework-free (no i18n, no Svelte) so they can be unit-tested and
 * so the dialog stays presentational. Errors are returned as stable codes;
 * the component maps them to a localized message. These are a client-side
 * convenience only — the backend (`validate_repo_relative_path`) remains
 * the authority and rejects the same shapes.
 */

/** Why an input failed validation. All map to one friendly message today. */
export type PathError = "empty" | "traversal" | "absolute" | "invalid-chars";

/**
 * Characters disallowed inside a single path segment (Windows-reserved).
 * Path separators are intentionally excluded here — a *directory* may
 * legitimately contain `/`; leaves reject separators via `LEAF_INVALID`.
 */
const SEGMENT_INVALID = /[<>:"|?*]/;

/** Leaf-name reserved set: Windows-illegal chars plus both separators. */
const LEAF_INVALID = /[<>:"|?*\\/]/;

/**
 * Normalize a user-typed directory into a clean repo-relative form:
 * backslashes become forward slashes and empty segments (leading,
 * trailing, and duplicate separators) are dropped. `""` means the repo
 * root.
 */
export function normalizeDir(input: string): string {
  return input
    .replace(/\\/g, "/")
    .split("/")
    .map((s) => s.trim())
    .filter((s) => s !== "")
    .join("/");
}

/**
 * Validate a single leaf name (a file or folder name with no separators).
 * Behavior matches the dialog's original checks so the rename flow is
 * unaffected.
 */
export function validateLeaf(name: string): PathError | null {
  const trimmed = name.trim();
  if (trimmed === "") return "empty";
  if (trimmed.startsWith("/")) return "absolute";
  if (trimmed.startsWith("..")) return "traversal";
  if (trimmed.split("/").some((part) => part === "..")) return "traversal";
  if (LEAF_INVALID.test(trimmed)) return "invalid-chars";
  return null;
}

/**
 * Validate a parent directory typed by the user. An empty string is the
 * repo root (valid). Nested directories separated by `/` are allowed;
 * `..` segments, absolute paths, and Windows-illegal characters are
 * rejected.
 */
export function validateDir(input: string): PathError | null {
  const trimmed = input.trim();
  if (trimmed === "") return null; // repo root
  if (/^[/\\]/.test(trimmed)) return "absolute";
  for (const segment of normalizeDir(trimmed).split("/")) {
    if (segment === "..") return "traversal";
    if (SEGMENT_INVALID.test(segment)) return "invalid-chars";
  }
  return null;
}

/**
 * Join a parent directory and a leaf name into a repo-relative path,
 * normalizing the parent first. `""` parent yields the bare leaf.
 */
export function joinRepoPath(parent: string, leaf: string): string {
  const dir = normalizeDir(parent);
  const name = leaf.trim();
  return dir === "" ? name : `${dir}/${name}`;
}
