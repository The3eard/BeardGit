/**
 * Per-repo Requests-panel state (RepoState slice).
 *
 * Holds one repo's currently-selected request source / parsed request doc,
 * the active environment, the run lifecycle, and the last response. These
 * used to be module-level singletons in `components/requests/stores.ts`,
 * shared by every open repo tab — so a `.http` selection made in repo A
 * survived a switch to repo B and could Send/Save against B's path (writing
 * or executing the file in the wrong repository). Giving each open repo its
 * own `RequestsSlice` makes the selection follow the active tab (spec 08
 * slice pattern, like `CompareSlice`).
 *
 * Fields are plain Svelte `writable`s (not `$state` runes) — see the note in
 * `./index.ts` for why the fallback was chosen for this migration step.
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

export class RequestsSlice {
  /** The request file currently selected in the collections tree. */
  readonly currentSource = writable<RequestSource | null>(null);
  /** The parsed request doc bound to the editor. */
  readonly currentRequest = writable<RequestDoc | null>(null);
  /** The active environment name (e.g. `"dev"`, `"prod"`), if any. */
  readonly currentEnv = writable<string | null>(null);
  /** Lifecycle state of the most recently triggered run. */
  readonly runState = writable<RunState>("idle");
  /** Last successful response body + metadata. */
  readonly lastResponse = writable<ResponseDoc | null>(null);
  /** Error message from the last run when `runState` is `"error"`. */
  readonly lastResponseError = writable<string | null>(null);

  /** Reset all requests selection/run state for this repo. */
  clear(): void {
    this.currentSource.set(null);
    this.currentRequest.set(null);
    this.currentEnv.set(null);
    this.runState.set("idle");
    this.lastResponse.set(null);
    this.lastResponseError.set(null);
  }
}
