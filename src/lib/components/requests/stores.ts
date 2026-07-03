/**
 * Frontend stores for the Requests panel.
 *
 * Holds the user's currently-selected request source / request doc plus
 * the run state and last response. Kept deliberately small at this
 * stage — Phase 9 only wires the panel shell; subsequent phases (10–12)
 * will add list-loading, env switching, run/cancel, and response render
 * actions on top of these stores.
 *
 * `RequestSource` distinguishes project-local `.http` collections
 * (under the repo's `requests/` folder) from global ones (under the
 * app config dir). The backend exposes both via the
 * `requests_list_project` and `requests_list_global` commands.
 */

import { writable } from "svelte/store";
import { activeField } from "$lib/stores/repo-state";
import type {
  RequestSource,
  RunState,
  RequestDoc,
  ResponseDoc,
} from "$lib/stores/repo-state/RequestsSlice";

export type { RequestSource, RunState, RequestDoc, ResponseDoc };

// The user's request selection / run state is per-repo: it lives in the
// active repo's `RequestsSlice` so switching project tabs shows that repo's
// selection (and never Sends/Saves against the wrong repo). These facades
// proxy the active slice so existing component imports keep working.

/** The request file currently selected in the collections tree. */
export const currentSource = activeField<RequestSource | null>((rs) => rs.requests.currentSource);
/** The parsed request doc bound to the editor. */
export const currentRequest = activeField<RequestDoc | null>((rs) => rs.requests.currentRequest);
/** The active environment name (e.g. `"dev"`, `"prod"`), if any. */
export const currentEnv = activeField<string | null>((rs) => rs.requests.currentEnv);
/** Lifecycle state of the most recently triggered run. */
export const runState = activeField<RunState>((rs) => rs.requests.runState);
/** Last successful response body + metadata. */
export const lastResponse = activeField<ResponseDoc | null>((rs) => rs.requests.lastResponse);
/** Error message from the last run when `runState` is `"error"`. */
export const lastResponseError = activeField<string | null>((rs) => rs.requests.lastResponseError);

/**
 * Bumped by any action that mutates the on-disk requests tree (seeding,
 * external-edit watcher, future create/rename/delete commands).
 * `CollectionsTree` subscribes to this so the file tree refreshes
 * without a full panel remount.
 */
export const treeReloadSignal = writable(0);

/**
 * Toggled to open the New Request dialog from anywhere in the panel.
 * `CollectionsTree` owns the actual dialog markup and listens to this
 * store so secondary triggers — like the SeedPrompt's "Create new
 * request" button when the tree is empty — can request the dialog
 * without lifting the dialog state into the parent.
 */
export const newRequestOpen = writable(false);

/**
 * Serialize a `RequestDoc` back to `.http` text suitable for
 * `requests_save`. The backend's executor reads `.http` files from disk,
 * so the editor must persist the in-memory doc before each run.
 *
 * The format is intentionally minimal: an optional `# @name` line, the
 * request line (`METHOD URL`), header lines, and — when a body is
 * present — a blank separator followed by the raw body. Non-trivial
 * features (multiple requests per file, comments, leading metadata) are
 * preserved at parse time elsewhere; this writer always emits a single
 * canonical request.
 */
export function requestDocToHttp(req: RequestDoc): string {
  const lines: string[] = [];
  if (req.name) lines.push(`# @name ${req.name}`);
  lines.push(`${req.method} ${req.url}`);
  for (const [k, v] of req.headers) {
    if (k.trim()) lines.push(`${k}: ${v}`);
  }
  if (req.body && req.body.trim()) {
    lines.push("");
    lines.push(req.body);
  }
  return lines.join("\n") + "\n";
}
