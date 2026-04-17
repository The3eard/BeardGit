/**
 * AI Background Worktree store.
 *
 * Tracks headless AI runs launched from the "New AI worktree run" dialog.
 * Subscribes to the Tauri `ai-background-status` and `ai-background-output`
 * events and keeps:
 *
 * - A `Map<sessionId, AiSession>` with the live status of every known run.
 * - A `Map<sessionId, string[]>` with each session's captured transcript.
 *
 * Output updates are batched via `requestAnimationFrame` (mirroring the
 * `tasks.ts` pattern) so the UI doesn't repaint on every line when the
 * provider dumps hundreds of lines of JSON stream output.
 */

import { writable, derived, get } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AiBackgroundOutputEvent,
  AiSession,
  StartBackgroundRunRequest,
  StartBackgroundRunResponse,
} from "../types";
import * as api from "../api/tauri";

// ─── State ────────────────────────────────────────────────────────────────

/** All known AI background runs, keyed by session id. */
export const aiBackgroundRuns = writable<Map<string, AiSession>>(new Map());

/** Per-session transcript (stdout+stderr lines, in arrival order). */
export const aiBackgroundTranscripts = writable<Map<string, string[]>>(new Map());

/** The session currently selected in the Sessions detail pane, if any. */
export const selectedBackgroundSessionId = writable<string | null>(null);

/**
 * A simple signal (increments on each open request) so any number of entry
 * points (tab bar, sidebar, shortcut) can trigger the same
 * `CreateBackgroundRunDialog` instance mounted at the app shell level.
 */
export const openCreateBackgroundRunDialogRequest = writable(0);

/** Helper: request the dialog open. Idempotent across concurrent callers. */
export function requestOpenCreateBackgroundRunDialog(): void {
  openCreateBackgroundRunDialogRequest.update((n) => n + 1);
}

/** Derived count of runs whose status is `queued` or `running`. */
export const activeBackgroundRunCount = derived(
  aiBackgroundRuns,
  ($runs) =>
    Array.from($runs.values()).filter(
      (s) =>
        s.background_status?.state === "queued" ||
        s.background_status?.state === "running",
    ).length,
);

/** Derived: currently selected session (full object). */
export const selectedBackgroundSession = derived(
  [aiBackgroundRuns, selectedBackgroundSessionId],
  ([$runs, $id]) => ($id ? $runs.get($id) ?? null : null),
);

// ─── rAF batching for transcript updates ──────────────────────────────────

/** Buffered lines between repaints, keyed by session id. */
const pendingLines = new Map<string, string[]>();
let rafScheduled = false;

function flushPendingLines(): void {
  rafScheduled = false;
  if (pendingLines.size === 0) return;
  aiBackgroundTranscripts.update((map) => {
    const next = new Map(map);
    for (const [sessionId, lines] of pendingLines) {
      const existing = next.get(sessionId) ?? [];
      next.set(sessionId, existing.concat(lines));
    }
    return next;
  });
  pendingLines.clear();
}

function enqueueLine(sessionId: string, line: string): void {
  const buf = pendingLines.get(sessionId) ?? [];
  buf.push(line);
  pendingLines.set(sessionId, buf);
  if (!rafScheduled) {
    rafScheduled = true;
    // Fall back to `setTimeout` for Node/test environments that don't expose rAF.
    const schedule =
      typeof requestAnimationFrame === "function"
        ? requestAnimationFrame
        : (cb: () => void) => setTimeout(cb, 16);
    schedule(flushPendingLines);
  }
}

// ─── Actions ──────────────────────────────────────────────────────────────

/** Bulk-replace the map from a fresh backend list. */
export function setAiBackgroundRuns(sessions: AiSession[]): void {
  aiBackgroundRuns.set(new Map(sessions.map((s) => [s.id, s])));
}

/** Merge a single session into the store (insert-or-update). */
export function upsertAiBackgroundRun(session: AiSession): void {
  aiBackgroundRuns.update((map) => {
    const next = new Map(map);
    next.set(session.id, session);
    return next;
  });
}

/** Launch a new background run. Returns the coordinator's response. */
export async function startAiBackgroundRun(
  request: StartBackgroundRunRequest,
): Promise<StartBackgroundRunResponse> {
  const res = await api.aiStartBackgroundRun(request);
  // Seed the store with a placeholder so the UI can show it instantly even
  // before the first `ai-background-status` event arrives.
  const placeholder: AiSession = {
    id: res.session_id,
    provider: request.provider,
    cwd: res.worktree_path,
    started_at: Date.now(),
    kind: "headless",
    is_active: true,
    worktree_path: res.worktree_path,
    background_status: res.status,
    task_id: res.task_id,
  };
  upsertAiBackgroundRun(placeholder);
  return res;
}

/** Request cancellation of a running session. */
export async function cancelAiBackgroundRun(sessionId: string): Promise<void> {
  await api.aiCancelBackgroundRun(sessionId);
}

/** Remove the worktree for a terminal session and drop it from the store. */
export async function discardAiBackgroundRunWorktree(sessionId: string): Promise<void> {
  await api.aiDiscardBackgroundRunWorktree(sessionId);
  aiBackgroundRuns.update((map) => {
    const next = new Map(map);
    next.delete(sessionId);
    return next;
  });
  aiBackgroundTranscripts.update((map) => {
    const next = new Map(map);
    next.delete(sessionId);
    return next;
  });
}

/** Open a PTY terminal attached to the session's worktree. */
export async function openTerminalForAiBackgroundSession(sessionId: string): Promise<number> {
  return api.aiOpenBackgroundTerminal(sessionId);
}

/** Refresh the full list from the backend (e.g. on app start). */
export async function refreshAiBackgroundRuns(): Promise<void> {
  const runs = await api.aiListBackgroundRuns();
  setAiBackgroundRuns(runs);
}

// ─── Event wiring ────────────────────────────────────────────────────────

let unlistenStatus: UnlistenFn | null = null;
let unlistenOutput: UnlistenFn | null = null;

/** Register Tauri event listeners. Idempotent. */
export async function startAiBackgroundListeners(): Promise<void> {
  if (unlistenStatus && unlistenOutput) return;

  if (!unlistenStatus) {
    unlistenStatus = await listen<AiSession>("ai-background-status", (ev) => {
      upsertAiBackgroundRun(ev.payload);
    });
  }

  if (!unlistenOutput) {
    unlistenOutput = await listen<AiBackgroundOutputEvent>(
      "ai-background-output",
      (ev) => {
        enqueueLine(ev.payload.session_id, ev.payload.line);
      },
    );
  }
}

/** Tear down Tauri event listeners. */
export function stopAiBackgroundListeners(): void {
  unlistenStatus?.();
  unlistenStatus = null;
  unlistenOutput?.();
  unlistenOutput = null;
}

// ─── Test helpers ─────────────────────────────────────────────────────────

/**
 * Synchronously process any pending transcript lines. Test-only hook — in
 * production the `requestAnimationFrame` tick does this.
 */
export function __flushTranscriptBufferForTests(): void {
  flushPendingLines();
}

/** Snapshot of the internal transcript buffer — test helper. */
export function __getAiBackgroundTranscripts(): Map<string, string[]> {
  return get(aiBackgroundTranscripts);
}
