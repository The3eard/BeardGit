/**
 * AI Sessions store — session listing with auto-refresh.
 *
 * Fetches active AI sessions for the current project, filtered by cwd.
 * Auto-refreshes via Tauri event bridge (ai-sessions-changed) and terminal lifecycle events.
 */

import { writable, derived } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import * as api from "$lib/api/tauri";
import type { AiSession } from "$lib/types";
import { aiBackgroundRuns, selectedBackgroundSessionId } from "./aiBackground";

// ─── State ───

/** AI sessions filtered to the current project (provider-reported). */
export const sessions = writable<AiSession[]>([]);

/**
 * Merged view: background runs (sorted first) followed by provider-reported
 * sessions. Two-pass dedupe:
 *
 * 1. **By `id`** — a session that shows up in both `aiBackgroundRuns` and the
 *    provider-reported `sessions` list is kept once (bg-run wins).
 * 2. **By `(provider, normalizedCwd)` on active-interactive entries** — when
 *    the Rust side spawned an interactive Claude in a BeardGit tab *and*
 *    provider file-watching reports the same process under a different id
 *    (Claude writes its own UUID into `~/.claude/sessions/{pid}.json`), both
 *    rows would otherwise appear with independent "Focus" buttons. We
 *    collapse them, preferring the bg-run entry (richer metadata — it has a
 *    `task_id`, a `worktree_path`, and a `background_status`); otherwise
 *    the most recently started entry wins.
 *
 * Ended or headless rows are left untouched even when they share a cwd —
 * they represent distinct historical sessions the user may want to dismiss
 * independently, and collapsing them would hide information rather than
 * deduplicate a bug.
 */
export const mergedSessions = derived(
  [sessions, aiBackgroundRuns],
  ([$sessions, $bg]) => {
    const bgList = Array.from($bg.values());
    // Most-recent background run first, Queued/Running before terminal states.
    bgList.sort((a, b) => {
      const aActive =
        a.background_status?.state === "running" ||
        a.background_status?.state === "queued"
          ? 0
          : 1;
      const bActive =
        b.background_status?.state === "running" ||
        b.background_status?.state === "queued"
          ? 0
          : 1;
      if (aActive !== bActive) return aActive - bActive;
      return (b.started_at ?? 0) - (a.started_at ?? 0);
    });
    const seen = new Set(bgList.map((s) => s.id));
    const tail = $sessions.filter((s) => !seen.has(s.id));
    const combined = [...bgList, ...tail];

    // Second-pass dedupe: collapse active-interactive siblings that share
    // `(provider, cwd)`. Stability of the original ordering matters here —
    // we walk `combined` in its current order and only drop subsequent
    // duplicates when the already-stored one "wins" the tie-break.
    const result: AiSession[] = [];
    const keyToIndex = new Map<string, number>();
    for (const s of combined) {
      if (!s.is_active || s.kind !== "interactive") {
        result.push(s);
        continue;
      }
      const key = `${s.provider}|${s.cwd.replace(/\/+$/, "")}`;
      const existingIdx = keyToIndex.get(key);
      if (existingIdx === undefined) {
        keyToIndex.set(key, result.length);
        result.push(s);
      } else if (preferNewer(s, result[existingIdx])) {
        result[existingIdx] = s;
      }
    }
    return result;
  },
);

/**
 * Tie-breaker for the `(provider, cwd)` dedupe pass. Prefers the entry that
 * carries richer metadata (bg-run > PID-discovered), falling back to the
 * most recent `started_at`. A `true` return means `candidate` should replace
 * `current` in the deduped list.
 */
function preferNewer(candidate: AiSession, current: AiSession): boolean {
  const candidateIsBg = candidate.background_status != null;
  const currentIsBg = current.background_status != null;
  if (candidateIsBg !== currentIsBg) return candidateIsBg;
  return (candidate.started_at ?? 0) > (current.started_at ?? 0);
}

/**
 * Currently selected session (any kind), resolved against the merged list.
 *
 * This is the store that `AiSessionDetail.svelte` consumes so *every* row
 * type populates the detail pane — provider-reported sessions included.
 * The narrower `selectedBackgroundSession` export in `./aiBackground` is
 * still used for background-run-only concerns (transcripts, discard/cancel
 * handlers that require a `background_status`).
 */
export const selectedSession = derived(
  [mergedSessions, selectedBackgroundSessionId],
  ([$list, $id]) =>
    $id ? ($list.find((s) => s.id === $id) ?? null) : null,
);

/** True while loading sessions.
 *
 * Defaults to `true` so the very first paint of `AiSessionList` (before any
 * refresh has fired) shows the spinner instead of the empty state — matches
 * the pipelines UX of "click section → section appears → spinner → list".
 * The first `refreshSessions` flips it back to `false` in its `finally`. */
export const sessionsLoading = writable(true);

// ─── Helpers ───

/**
 * Filter sessions by project path and sort: active first, then by start time descending.
 *
 * Normalizes trailing slashes before comparing so that symlink resolution or
 * minor path differences don't cause mismatches. Also includes sessions whose
 * cwd is a subdirectory of the project (e.g. worktree paths).
 */
export function filterSessionsByProject(
  allSessions: AiSession[],
  projectPath: string,
): AiSession[] {
  const normalizedProject = projectPath.replace(/\/+$/, "");
  return allSessions
    .filter((s) => {
      const normalizedCwd = s.cwd.replace(/\/+$/, "");
      return (
        normalizedCwd === normalizedProject ||
        normalizedCwd.startsWith(normalizedProject + "/")
      );
    })
    .sort((a, b) => {
      if (a.is_active !== b.is_active) return a.is_active ? -1 : 1;
      return (b.started_at ?? 0) - (a.started_at ?? 0);
    });
}

// ─── Actions ───

/** Fetch all sessions and filter to current project path. */
export async function refreshSessions(
  projectPath?: string,
): Promise<void> {
  sessionsLoading.set(true);
  try {
    const all = await api.aiListSessions();
    if (projectPath) {
      sessions.set(filterSessionsByProject(all, projectPath));
    } else {
      sessions.set(all);
    }
  } catch {
    sessions.set([]);
  } finally {
    sessionsLoading.set(false);
  }
}

/** Remove a session from the local list (does not delete session files). */
export function dismissSession(id: string): void {
  sessions.update((list) => list.filter((s) => s.id !== id));
}

/** Clear session state. Called on view/project switch. */
export function clearSessionState(): void {
  sessions.set([]);
  sessionsLoading.set(false);
}

// ─── Event Listeners ───

let unlistenSessionsChanged: (() => void) | null = null;
let unlistenTerminalClose: (() => void) | null = null;

/** Set up auto-refresh listeners. Call once on view mount. */
export async function startSessionListeners(
  projectPath: string,
): Promise<void> {
  const unlisten1 = await listen("ai-sessions-changed", () => {
    refreshSessions(projectPath);
  });
  unlistenSessionsChanged = unlisten1;

  const unlisten2 = await listen("terminal-closed", () => {
    refreshSessions(projectPath);
  });
  unlistenTerminalClose = unlisten2;
}

/** Tear down auto-refresh listeners. Call on view unmount. */
export function stopSessionListeners(): void {
  unlistenSessionsChanged?.();
  unlistenSessionsChanged = null;
  unlistenTerminalClose?.();
  unlistenTerminalClose = null;
}
