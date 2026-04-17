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
import { aiBackgroundRuns } from "./aiBackground";

// ─── State ───

/** AI sessions filtered to the current project (provider-reported). */
export const sessions = writable<AiSession[]>([]);

/**
 * Merged view: background runs (sorted first) followed by provider-reported
 * sessions. Dedupes by id so a session that exists in both lists is shown
 * once. Consumers reading "sessions for the sidebar" should pull from this.
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
    return [...bgList, ...tail];
  },
);

/** True while loading sessions. */
export const sessionsLoading = writable(false);

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
