/** Tagged error returned by the Rust `open_project` command. */
export type OpenProjectErr =
  | { kind: "not_a_repo"; path: string }
  | { kind: "other"; message: string };

/**
 * Best-effort parser for whatever a Tauri `invoke` rejection contains.
 *
 * Tauri serialises Rust enum errors as JSON objects when the command
 * returns `Result<T, E: Serialize>` — but it can also reject with a plain
 * string for older commands. This handles both shapes; returns `null`
 * for anything that doesn't match a known form.
 */
export function parseOpenProjectError(raw: unknown): OpenProjectErr | null {
  if (typeof raw === "string") {
    return { kind: "other", message: raw };
  }
  if (raw && typeof raw === "object") {
    const obj = raw as Record<string, unknown>;
    if (obj.kind === "not_a_repo" && typeof obj.path === "string") {
      return { kind: "not_a_repo", path: obj.path };
    }
    if (obj.kind === "other" && typeof obj.message === "string") {
      return { kind: "other", message: obj.message };
    }
  }
  return null;
}
