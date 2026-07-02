/**
 * Helpers for the two shapes a Tauri command can reject with:
 *
 *   1. A plain `string` — the legacy `Result<_, String>` commands.
 *   2. A structured `{ code, message }` envelope — commands migrated to
 *      `IpcError` (see crates/app-core/src/ipc_error.rs). The stable snake_case
 *      `code` lets the frontend branch on error kind (`not_a_repo`,
 *      `auth_required`, `not_fast_forward`, …) instead of matching free text.
 *
 * Both shapes coexist during the incremental migration, so every helper here
 * degrades gracefully: string errors simply have no `code`.
 */

/**
 * Extract the stable machine-readable `code` from a Tauri rejection, or `null`
 * when the error is a plain string (or otherwise carries no string `code`).
 */
export function getErrorCode(e: unknown): string | null {
  if (e && typeof e === "object" && "code" in e) {
    const code = (e as { code: unknown }).code;
    if (typeof code === "string") return code;
  }
  return null;
}

/**
 * Normalize any Tauri rejection to a human-readable message string. Handles
 * plain strings, `Error` instances, and `{ message }` / `{ code, message }`
 * objects; falls back to `String(e)` for anything else.
 */
export function getErrorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  if (e && typeof e === "object" && "message" in e) {
    const m = (e as { message: unknown }).message;
    if (typeof m === "string") return m;
  }
  return String(e);
}

/** First non-empty line of {@link getErrorMessage} — the single-line form used in toasts. */
export function firstErrorLine(e: unknown): string {
  const msg = getErrorMessage(e);
  return msg.split(/\r?\n/, 1)[0] ?? msg;
}

/**
 * Concise label for the handful of error codes worth surfacing distinctly in a
 * toast. Returns `null` for unmapped codes so callers fall back to the raw
 * message. This is deliberately NOT an exhaustive i18n table (spec 05 defers
 * full codegen + per-code localization); extend it only as codes prove
 * branch-worthy.
 */
export function errorCodeMessage(code: string): string | null {
  switch (code) {
    case "auth_required":
      return "Authentication required";
    case "not_fast_forward":
      return "Push rejected — the remote has commits you don't have locally";
    case "repo_not_found":
      return "Not a git repository";
    default:
      return null;
  }
}
