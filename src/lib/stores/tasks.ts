/**
 * Unified tasks aggregator — powers the statusbar tasks popover
 * (`TasksPopover.svelte`) and every derived surface the popover
 * exposes (icon spin + colour, list badge, drill-down).
 *
 * Three independent bridges feed into one internal `Map<string, TaskEntry>`:
 *
 * 1. **Rust `task://update` events** — emitted from
 *    `crates/app-core/src/task_events.rs` for every git fetch / pull / push
 *    / clone lifecycle transition (and AI-background / app-update kinds
 *    once those producers wire their emitters).
 * 2. **`aiBackgroundRuns` store** — headless AI runs surfaced by
 *    `src/lib/stores/aiBackground.ts`. Each `AiSession` projects to a
 *    `TaskEntry` with id `"ai-background:<session_id>"` so Rust
 *    `task://update` events and AI-background store updates never collide
 *    on the same key.
 * 3. **`autoUpdate.updateTask`** — the derived `TaskEntry` from
 *    `src/lib/stores/autoUpdate.ts` with the stable id
 *    `"auto-update"`.
 *
 * All three subscribers funnel through a single `requestAnimationFrame`-
 * coalesced flush so burst updates (for example a fast fetch emitting many
 * progress ticks) only trigger one Svelte re-render per frame.
 *
 * Cancellation is routed back to the source producer by `cancelTaskById`:
 * git ops → `task_cancel` IPC, AI headless → `ai_cancel_background_run`,
 * AI interactive → `terminal_kill`, auto-update → `cancelUpdateDownload`.
 *
 * The legacy raw `task-*` Tauri events remain wired in
 * `src/lib/stores/taskPanel.ts`, which still feeds the stdout/stderr
 * drill-down view and the AI commit-message harvester.
 */

import { derived, get, writable, type Readable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import * as api from "../api/tauri";
import type {
  TaskAction,
  TaskEntry,
  TaskKind,
  TaskProgress,
  TaskStatus,
} from "../types/tasks";
import type { AiSession } from "../types";
import { aiBackgroundRuns } from "./aiBackground";
import {
  AUTO_UPDATE_TASK_ID,
  cancelUpdateDownload,
  updateTask as autoUpdateTask,
} from "./autoUpdate";
import * as m from "$lib/paraglide/messages";

// ─── Public stores ────────────────────────────────────────────────────────────

/** Flat ordered list of every task the drawer should display. */
export const tasksStore = writable<TaskEntry[]>([]);

/**
 * Count of entries currently in the `running` state.
 *
 * Used by the statusbar tasks slot for its badge, and by the drawer's
 * active-section heading.
 */
export const activeTaskCount: Readable<number> = derived(
  tasksStore,
  ($tasks) => $tasks.filter((t) => t.status === "running").length,
);

/** Window (in ms) during which finished tasks stay in the drawer. */
export const RECENTLY_FINISHED_WINDOW_MS = 5 * 60 * 1000;

/**
 * Terminal-state entries finished within the last
 * {@link RECENTLY_FINISHED_WINDOW_MS}.
 *
 * Entries older than the window are hidden from the drawer; the full
 * entry is still in `tasksStore` until {@link clearFinished} prunes it
 * (or the next `triggerUpdate` with the same id brings it back to life).
 */
export const recentlyFinishedTasks: Readable<TaskEntry[]> = derived(
  tasksStore,
  ($tasks) => {
    const cutoff = Date.now() - RECENTLY_FINISHED_WINDOW_MS;
    return $tasks.filter(
      (t) =>
        t.status !== "running" &&
        typeof t.finishedAt === "number" &&
        t.finishedAt >= cutoff,
    );
  },
);

/** Internal: errors reported in this session that the user hasn't seen yet. */
const unseenErrorIds = writable<Set<string>>(new Set());

/**
 * `true` while at least one failed task has not been acknowledged by the
 * user (drawer open / `markSeen()` called).
 *
 * The statusbar's tasks slot uses this to render a red unseen-error dot.
 */
export const hasUnseenError: Readable<boolean> = derived(
  unseenErrorIds,
  ($set) => $set.size > 0,
);

/**
 * `true` when at least one entry in {@link tasksStore} is currently
 * `running`.
 *
 * Drives the statusbar tasks icon's spin animation so the user gets
 * peripheral-vision feedback that background work is in flight.
 */
export const anyRunning: Readable<boolean> = derived(
  tasksStore,
  ($tasks) => $tasks.some((t) => t.status === "running"),
);

/**
 * The most recent entry across every bridge — preferring running tasks
 * (newest `startedAt` wins), then the newest finished entry by
 * `finishedAt` (or `startedAt` when `finishedAt` is missing).
 *
 * Powers the statusbar tasks icon's status-aware colour: running →
 * accent, success → green, error → red, cancelled → muted, null →
 * default. When the user has no history the store emits `null`.
 */
export const latestEntry: Readable<TaskEntry | null> = derived(
  tasksStore,
  ($tasks) => {
    if ($tasks.length === 0) return null;
    const running = $tasks.filter((t) => t.status === "running");
    if (running.length > 0) {
      return running.reduce((a, b) => (a.startedAt >= b.startedAt ? a : b));
    }
    return $tasks.reduce((a, b) => {
      const at = a.finishedAt ?? a.startedAt;
      const bt = b.finishedAt ?? b.startedAt;
      return at >= bt ? a : b;
    });
  },
);

// ─── Internal state ──────────────────────────────────────────────────────────

/** Source of truth — keyed by `TaskEntry.id`. */
const entryMap = new Map<string, TaskEntry>();

/** rAF handle so coalesced flushes don't schedule multiple ticks. */
let flushScheduled = false;

/** Tauri event unlistener (returned from `listen("task://update", …)`). */
let unlistenTaskEvent: UnlistenFn | null = null;

/** Unsubscribers for the Svelte-store bridges. */
let unsubscribeAiBackground: (() => void) | null = null;
let unsubscribeAutoUpdate: (() => void) | null = null;

// ─── Coalesced flush ─────────────────────────────────────────────────────────

/**
 * Queue a flush of the internal `entryMap` to {@link tasksStore}.
 *
 * All writers funnel through here so N rapid updates within a single
 * frame produce at most one Svelte re-render. In Node test environments
 * without `requestAnimationFrame` the caller can stub it to run
 * synchronously (see the setup shim in `tasks.test.ts`).
 */
function scheduleFlush(): void {
  if (flushScheduled) return;
  flushScheduled = true;
  const schedule =
    typeof requestAnimationFrame === "function"
      ? requestAnimationFrame
      : (cb: () => void) => setTimeout(cb, 16);
  schedule(() => {
    flushScheduled = false;
    tasksStore.set(Array.from(entryMap.values()));
  });
}

/**
 * Upsert an entry into the internal map and schedule a flush.
 *
 * Preserves the caller's action list — producers decide the right
 * actions based on kind + status and we trust them.
 */
function upsert(entry: TaskEntry): void {
  // Re-apply any pending subtitle override so the AI review's saved
  // filename (and any future override-style metadata) survives a late
  // `task://update` upsert that wouldn't otherwise know about it.
  const override = subtitleOverrides.get(entry.id);
  const final = override !== undefined ? { ...entry, subtitle: override } : entry;
  entryMap.set(entry.id, final);
  if (final.status === "error") {
    unseenErrorIds.update((s) => {
      if (s.has(final.id)) return s;
      const next = new Set(s);
      next.add(final.id);
      return next;
    });
  }
  scheduleFlush();
}

/**
 * Per-id subtitle override. The Rust task producer doesn't know about
 * downstream artefacts (e.g. the saved `.beardgit/reviews/...md` path
 * for an AI review), so the FE assigns those out-of-band via
 * [`setTaskSubtitle`]. Storing the override here — separate from the
 * entry record — means any subsequent `upsert` (e.g. from a late
 * `task://update` event that races behind `task-completed`) won't
 * silently clobber it; `upsert` re-applies the override on every
 * write.
 */
const subtitleOverrides = new Map<string, string>();

/**
 * Patch the `subtitle` of an already-tracked entry. Survives any
 * later `upsert` for the same id because `upsert` consults
 * [`subtitleOverrides`] before writing.
 *
 * Safe to call before the entry has been ingested — the override
 * waits for the matching upsert, which then applies it. No-ops when
 * the override happens to land alongside an in-flight entry.
 */
export function setTaskSubtitle(id: string, subtitle: string): void {
  subtitleOverrides.set(id, subtitle);
  const existing = entryMap.get(id);
  if (existing) {
    entryMap.set(id, { ...existing, subtitle });
    scheduleFlush();
  }
}

/**
 * Remove a single entry from the drawer (and its subtitle override).
 * Called when the user clicks per-row "Dismiss" — distinct from the
 * header's "Clear" button, which is the bulk wipe.
 */
export function removeTask(id: string): void {
  entryMap.delete(id);
  subtitleOverrides.delete(id);
  aiBridgeIds.delete(id);
  unseenErrorIds.update((s) => {
    if (!s.has(id)) return s;
    const next = new Set(s);
    next.delete(id);
    return next;
  });
  scheduleFlush();
}

// ─── Kind-aware action dispatch ─────────────────────────────────────────────

/**
 * Build the default action list for a given kind + status.
 *
 * Rules (first slice):
 *
 * - **running** — `[Cancel]`
 * - **error** — `[Retry (when supported), Dismiss]`
 * - **success + ai_background** — `[Open output, Dismiss]`
 * - **success** — `[Dismiss]`
 * - **cancelled** — `[Dismiss]`
 */
function actionsFor(kind: TaskKind, status: TaskStatus): TaskAction[] {
  const cancel: TaskAction = {
    id: "cancel",
    label: m.tasks_action_cancel(),
    variant: "danger",
  };
  const retry: TaskAction = {
    id: "retry",
    label: m.tasks_action_retry(),
    variant: "primary",
  };
  const dismiss: TaskAction = {
    id: "dismiss",
    label: m.tasks_action_dismiss(),
    variant: "neutral",
  };
  const openOutput: TaskAction = {
    id: "open_output",
    label: m.tasks_action_open_output(),
    variant: "primary",
  };

  if (status === "running") return [cancel];

  if (status === "error") {
    // git_* producers don't yet surface retry metadata reliably — first
    // slice only exposes retry for AI and update kinds.
    const supportsRetry =
      kind === "ai_background" || kind === "app_update";
    return supportsRetry ? [retry, dismiss] : [dismiss];
  }

  if (status === "success" && kind === "ai_background") {
    return [openOutput, dismiss];
  }

  // success + other kinds, and cancelled for every kind.
  return [dismiss];
}

// ─── Bridge 1: Rust task://update events ────────────────────────────────────

/**
 * Wire snake_case payload matching `crates/app-core/src/task_events.rs`.
 *
 * Kept private to the module — external callers always read the
 * camelCase `TaskEntry` from {@link tasksStore}.
 */
interface TaskEventPayload {
  id: string;
  kind: TaskKind;
  title: string;
  subtitle?: string;
  started_at_ms: number;
  finished_at_ms?: number;
  status: TaskStatus;
  progress?: TaskProgress;
  error_message?: string;
}

function taskEventToEntry(payload: TaskEventPayload): TaskEntry {
  return {
    id: payload.id,
    kind: payload.kind,
    title: payload.title,
    subtitle: payload.subtitle,
    startedAt: payload.started_at_ms,
    finishedAt: payload.finished_at_ms,
    status: payload.status,
    progress: payload.progress,
    errorMessage: payload.error_message,
    actions: actionsFor(payload.kind, payload.status),
  };
}

// ─── Bridge 2: aiBackgroundRuns store ────────────────────────────────────────

/** Stable prefix so AI-session ids never collide with Rust `TaskId` strings. */
const AI_BACKGROUND_PREFIX = "ai-background:";

function aiSessionToEntry(session: AiSession): TaskEntry {
  const runStatus = session.background_status?.state;
  const status: TaskStatus =
    runStatus === "queued" || runStatus === "running"
      ? "running"
      : runStatus === "completed"
        ? "success"
        : runStatus === "failed"
          ? "error"
          : runStatus === "cancelled"
            ? "cancelled"
            : "running";

  const errorMessage =
    session.background_status?.state === "failed"
      ? session.background_status.message
      : undefined;

  const startedAt = session.started_at ?? Date.now();
  const finishedAt =
    status === "running" ? undefined : (session.started_at ?? Date.now());

  return {
    id: `${AI_BACKGROUND_PREFIX}${session.id}`,
    kind: "ai_background",
    title: session.provider,
    subtitle: session.worktree_path ?? session.cwd,
    startedAt,
    finishedAt,
    status,
    errorMessage,
    actions: actionsFor("ai_background", status),
  };
}

/** Ids of AI-background entries currently tracked — used to prune removals. */
const aiBridgeIds = new Set<string>();

function syncAiBackgroundBridge(
  runs: Map<string, AiSession>,
): void {
  const nextIds = new Set<string>();
  for (const session of runs.values()) {
    const entry = aiSessionToEntry(session);
    nextIds.add(entry.id);
    upsert(entry);
  }
  // Remove bridged entries that the source no longer reports.
  for (const id of aiBridgeIds) {
    if (!nextIds.has(id)) {
      entryMap.delete(id);
    }
  }
  aiBridgeIds.clear();
  for (const id of nextIds) aiBridgeIds.add(id);
  scheduleFlush();
}

// ─── Bridge 3: autoUpdate.updateTask ─────────────────────────────────────────

function syncAutoUpdateBridge(entry: TaskEntry | null): void {
  if (entry) {
    // Ensure actions are drawer-canonical even if autoUpdate.ts ships a
    // different subset later — single source of truth lives here.
    upsert({
      ...entry,
      actions: actionsFor(entry.kind, entry.status),
    });
  } else if (entryMap.has(AUTO_UPDATE_TASK_ID)) {
    entryMap.delete(AUTO_UPDATE_TASK_ID);
    scheduleFlush();
  }
}

// ─── Public API ──────────────────────────────────────────────────────────────

/**
 * Register the three bridges. Idempotent — calling it twice is a no-op
 * beyond a tiny cleanup; the app shell calls this once at mount.
 */
export async function initTasksStore(): Promise<void> {
  if (unlistenTaskEvent) return;

  unlistenTaskEvent = await listen<TaskEventPayload>(
    "task://update",
    (event) => {
      upsert(taskEventToEntry(event.payload));
    },
  );

  unsubscribeAiBackground = aiBackgroundRuns.subscribe((runs) => {
    syncAiBackgroundBridge(runs);
  });

  unsubscribeAutoUpdate = autoUpdateTask.subscribe((entry) => {
    syncAutoUpdateBridge(entry);
  });
}

/** Teardown for app shutdown or tests. Safe to call when never inited. */
export function stopTasksStore(): void {
  unlistenTaskEvent?.();
  unlistenTaskEvent = null;
  unsubscribeAiBackground?.();
  unsubscribeAiBackground = null;
  unsubscribeAutoUpdate?.();
  unsubscribeAutoUpdate = null;
  entryMap.clear();
  aiBridgeIds.clear();
  unseenErrorIds.set(new Set());
  tasksStore.set([]);
}

/**
 * Mark all currently-unseen errors as seen — called when the drawer
 * opens so the statusbar's red dot clears. Does NOT delete the failed
 * tasks; users dismiss those separately.
 */
export function markSeen(): void {
  unseenErrorIds.set(new Set());
}

/**
 * Remove every entry in a terminal state (success / error / cancelled),
 * regardless of age. Running tasks are preserved.
 */
export function clearFinished(): void {
  for (const [id, entry] of entryMap) {
    if (entry.status !== "running") {
      entryMap.delete(id);
      aiBridgeIds.delete(id);
      subtitleOverrides.delete(id);
    }
  }
  unseenErrorIds.set(new Set());
  scheduleFlush();
}

/**
 * Route a cancel action back to the source producer.
 *
 * - `ai_background` → `ai_cancel_background_run`
 * - `ai_interactive` → `terminal_kill`
 * - `git_*` → `task_cancel`
 * - `app_update` → `cancelUpdateDownload()`
 *
 * The entry must exist in the store at call time; otherwise the call is
 * a no-op (the task likely already terminated).
 */
export async function cancelTaskById(id: string): Promise<void> {
  const entry = entryMap.get(id);
  if (!entry) return;

  switch (entry.kind) {
    case "ai_background": {
      const sessionId = id.startsWith(AI_BACKGROUND_PREFIX)
        ? id.slice(AI_BACKGROUND_PREFIX.length)
        : id;
      await api.aiCancelBackgroundRun(sessionId);
      return;
    }
    case "ai_interactive": {
      const terminalId = Number.parseInt(id, 10);
      if (Number.isFinite(terminalId)) {
        await api.terminalKill(terminalId);
      }
      return;
    }
    case "git_fetch":
    case "git_pull":
    case "git_push":
    case "git_clone":
    case "ai_headless": {
      await api.taskCancel(id);
      return;
    }
    case "app_update": {
      cancelUpdateDownload();
      return;
    }
  }
}

/**
 * Test helper: synchronous snapshot of the internal entry map. Not
 * exported through the module's public types but available for unit
 * tests via `get(tasksStore)` or this shortcut.
 */
export function __getEntrySnapshotForTests(): TaskEntry[] {
  return Array.from(entryMap.values());
}

// ─── Dev-time sanity check ──────────────────────────────────────────────────

// `tasksStore` is the public surface — the aggregator keeps its own
// `entryMap` to avoid the N-subscriber tax of round-tripping through the
// writable. This `get`-based assertion forces the derived stores to read
// the initial empty value so Svelte doesn't warn about unused derived.
void get;
