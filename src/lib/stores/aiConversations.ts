/**
 * AI Conversations store — transcript-first listing for the current project.
 *
 * Mirrors `aiSessions.ts`'s shape (fetch + filter + auto-refresh via Tauri
 * events), but consumes `ai_list_conversations` so each row is a real
 * on-disk transcript rather than a PID-scanned live process. This is the
 * data source the transcript-first v2 UI (Phase 5) will render.
 *
 * Both stores deliberately coexist through Phase 5 — the legacy
 * `aiSessions.ts` keeps the old UI compiling until Phase 8 deletes the
 * PID-scan path outright.
 */

import { writable, derived } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import * as api from "$lib/api/tauri";
import type { AiConversation } from "$lib/types";

// ─── State ───

/** Conversations for the current project, most-recent first. */
export const conversations = writable<AiConversation[]>([]);

/**
 * True while a refresh is in flight.
 *
 * Defaults to `true` so the initial paint of the v2 list shows a spinner
 * instead of an empty state — matches the behaviour of
 * `sessionsLoading` in `aiSessions.ts`. The first `refreshConversations`
 * flips it back to `false` in its `finally` block.
 */
export const conversationsLoading = writable(true);

/** Currently selected conversation id, or null when nothing is selected. */
export const selectedConversationId = writable<string | null>(null);

/**
 * Resolved row from `conversations` for the current selection. Returns
 * `null` either when there's no selection or when the selected id is
 * stale (e.g. the conversation was dismissed between selection and read).
 */
export const selectedConversation = derived(
  [conversations, selectedConversationId],
  ([$list, $id]) => ($id ? ($list.find((c) => c.id === $id) ?? null) : null),
);

// ─── Helpers ───

/**
 * Filter conversations by project path.
 *
 * Normalises trailing slashes before comparing (so a cwd of `/repo/` and a
 * project path of `/repo` still match) and includes conversations whose
 * cwd is a subdirectory of the project — this matters for worktree
 * conversations, whose cwd is `<project>/.wt/<slug>`.
 *
 * The Rust side already filters by provider but not by cwd, so doing it
 * here keeps the store idempotent regardless of backend filtering
 * changes.
 */
export function filterConversationsByProject(
  all: AiConversation[],
  projectPath: string,
): AiConversation[] {
  const normalizedProject = projectPath.replace(/\/+$/, "");
  return all.filter((c) => {
    const normalizedCwd = c.cwd.replace(/\/+$/, "");
    return (
      normalizedCwd === normalizedProject ||
      normalizedCwd.startsWith(normalizedProject + "/")
    );
  });
}

// ─── Actions ───

/**
 * Fetch conversation transcripts and set the store.
 *
 * If `projectPath` is provided, filters by cwd locally (see
 * `filterConversationsByProject`). Always re-sorts by `last_activity_at`
 * descending — the Rust side already sorts, but doing it here means a
 * custom filter or future API change can't accidentally de-order the UI.
 *
 * On error, the store is cleared and loading is cleared; the error is
 * swallowed to match the sessions store's behaviour (a single failed
 * refresh shouldn't crash the view).
 */
export async function refreshConversations(
  projectPath?: string,
): Promise<void> {
  conversationsLoading.set(true);
  try {
    const all = await api.aiListConversations();
    const filtered = projectPath
      ? filterConversationsByProject(all, projectPath)
      : all;
    const sorted = [...filtered].sort(
      (a, b) => b.last_activity_at - a.last_activity_at,
    );
    conversations.set(sorted);
  } catch {
    conversations.set([]);
  } finally {
    conversationsLoading.set(false);
  }
}

/**
 * Remove a conversation from the local list without deleting the
 * on-disk transcript. Used by the dismiss affordance in the v2 UI.
 */
export function dismissConversation(id: string): void {
  conversations.update((list) => list.filter((c) => c.id !== id));
}

/** Reset state on view/project switch. */
export function clearConversationState(): void {
  conversations.set([]);
  conversationsLoading.set(false);
  selectedConversationId.set(null);
}

// ─── Event listeners ───

let unlistenSessionsChanged: UnlistenFn | null = null;
let unlistenTerminalClose: UnlistenFn | null = null;

/**
 * Start listening for filesystem/terminal events that imply a transcript
 * list refresh.
 *
 * - `ai-sessions-changed` is emitted by the Rust filesystem watcher when
 *   a provider writes or rotates a transcript file.
 * - `terminal-closed` covers the race where the fs watcher lags a freshly
 *   exited bg-run; the transcript exists but the change notification is
 *   still in flight when the user lands on the list.
 */
export async function startConversationListeners(
  projectPath: string,
): Promise<void> {
  const unlisten1 = await listen("ai-sessions-changed", () => {
    refreshConversations(projectPath);
  });
  unlistenSessionsChanged = unlisten1;

  const unlisten2 = await listen("terminal-closed", () => {
    refreshConversations(projectPath);
  });
  unlistenTerminalClose = unlisten2;
}

/** Tear down auto-refresh listeners. Call on view unmount. */
export function stopConversationListeners(): void {
  unlistenSessionsChanged?.();
  unlistenSessionsChanged = null;
  unlistenTerminalClose?.();
  unlistenTerminalClose = null;
}
