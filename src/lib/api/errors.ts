/**
 * Helpers for the two shapes a Tauri command can reject with:
 *
 *   1. A plain `string` — the legacy `Result<_, String>` commands.
 *   2. A structured `{ code, message }` envelope — commands migrated to
 *      `IpcError` (see crates/app-core/src/ipc_error.rs). The stable snake_case
 *      `code` lets the frontend branch on error kind (`not_a_repo`,
 *      `auth_required`, `not_fast_forward`, …) instead of matching free text.
 *
 * Every Tauri command now rejects with shape 2. Shape 1 survives for
 * non-command rejections — a thrown `Error`, a plugin, a `JSON.parse` — so
 * every helper here still degrades gracefully: string errors simply have
 * no `code`.
 *
 * **Use these rather than `String(e)`.** An `IpcError` is an object, so
 * `String(e)` renders it as the literal text `"[object Object]"`; that
 * broke 97 callsites across 38 files the day the last commands migrated,
 * including three that compared the result against known text to decide
 * what to do next. `eslint-rules/no-stringify-caught-error.cjs` blocks the
 * regression.
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
 *
 * `scripts/check-ipc-codes.mjs` asserts every code Rust can emit appears
 * either here or in `@unmapped` below. The check is there because nothing
 * used to notice a *new* code at all — a fresh `IpcError::new("…")` in Rust
 * raised no question about whether the UI should say something better than
 * the raw message, so the question never got asked. Adding a code now fails
 * the gate until it is classified.
 *
 * A code belongs here when the raw message would leave the user without a
 * next step. `not_fast_forward` is the archetype: git says "rejected
 * (non-fast-forward)", which does not tell anyone to pull first.
 *
 * Codes below are deliberately left to their raw message, grouped by why:
 *
 * - The message *is* the content, and a fixed sentence would replace
 *   detail with less. These carry stderr from git, a git hook, or a CLI:
 *   @unmapped git cli_error signing_failed io_error error internal
 *
 * - Already reported next to the field or dialog that caused them, where a
 *   toast-level sentence would be redundant or worse — several carry the
 *   offending path or URL as the whole message:
 *   @unmapped invalid_url invalid_destination destination_exists invalid_path
 *   @unmapped invalid_argument invalid_log_level
 *
 * - Step-level failures of a multi-step flow (clone, init, log config). The
 *   step name is in the code and the cause is in the message; a generic
 *   sentence per step would say only what the code already says:
 *   @unmapped clone_failed open_failed init_failed gitignore_failed
 *   @unmapped commit_failed create_remote_failed add_origin_failed
 *   @unmapped push_failed log_level_failed
 *
 * - Routine conditions, not failures. Raised via `IpcError::expected` (so
 *   they are not even logged as errors) when a read is dispatched against a
 *   background tab, where heavy state is `None` by the active-tab
 *   invariant. The UI shows an empty state, never a toast:
 *   @unmapped no_active_project no_repository_open
 *
 * - Content-shape refusals the viewer handles structurally, by showing a
 *   placeholder instead of text:
 *   @unmapped binary_file file_too_large
 *
 * - Partial failures whose whole value is the list of paths that survived.
 *   A fixed sentence would drop exactly the part the user needs in order to
 *   know what is still on disk:
 *   @unmapped discard_failed
 *
 * Kept as an alias for a code Rust no longer emits:
 * @legacy-code repo_not_found
 */
export function errorCodeMessage(code: string): string | null {
  switch (code) {
    case "auth_required":
      return "Authentication required";
    case "not_fast_forward":
      return "Push rejected — the remote has commits you don't have locally";
    case "not_a_repo":
    // `repo_not_found` was the legacy code the `open_repo` path emitted for the
    // same situation before it was unified onto `not_a_repo`; kept as an alias.
    case "repo_not_found":
      return "Not a git repository";
    case "would_lose_changes":
      return "Checkout would overwrite uncommitted changes — commit or stash first";
    case "not_fully_merged":
      return "Branch has unmerged commits — delete with force to discard them";
    case "branch_exists":
      return "A branch with that name already exists — choose a different name";
    default:
      return null;
  }
}
