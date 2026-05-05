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

/** A pointer to a `.http` file on disk — either project-local or global. */
export type RequestSource = { kind: "project" | "global"; path: string };

/** Lifecycle of a single request execution. */
export type RunState = "idle" | "running" | "done" | "error" | "canceled";

/**
 * In-memory shape of a parsed `.http` request that the editor binds to.
 * Mirrors the relevant fields of the backend `RequestDoc` type.
 */
export interface RequestDoc {
  name?: string;
  method: string;
  url: string;
  headers: [string, string][];
  body?: string;
}

/**
 * In-memory shape of a single executed response. Body is the raw bytes
 * (truncated server-side when over the configured cap), and the viewer
 * is responsible for any text decoding.
 */
export interface ResponseDoc {
  status: number;
  headers: [string, string][];
  body: Uint8Array;
  truncated: boolean;
  durationMs: number;
}

/** The request file currently selected in the collections tree. */
export const currentSource = writable<RequestSource | null>(null);
/** The parsed request doc bound to the editor. */
export const currentRequest = writable<RequestDoc | null>(null);
/** The active environment name (e.g. `"dev"`, `"prod"`), if any. */
export const currentEnv = writable<string | null>(null);
/** Lifecycle state of the most recently triggered run. */
export const runState = writable<RunState>("idle");
/** Last successful response body + metadata. */
export const lastResponse = writable<ResponseDoc | null>(null);
/** Error message from the last run when `runState` is `"error"`. */
export const lastResponseError = writable<string | null>(null);

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
