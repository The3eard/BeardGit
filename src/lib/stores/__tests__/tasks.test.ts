/**
 * Unit tests for the unified tasks aggregator store (`tasks.ts`).
 *
 * The aggregator merges three independent producers — Rust `task://update`
 * events, the `aiBackgroundRuns` store, and `autoUpdate.updateTask` — into
 * one flat `TaskEntry` feed. These tests exercise the public surface:
 *
 * - Ingesting a Rust event maps it to a `TaskEntry` with the right kind.
 * - Same-id re-emissions update in place (no duplicate rows).
 * - `aiBackgroundRuns` changes bridge into the feed.
 * - Derived counts + `recentlyFinishedTasks` + `hasUnseenError` signal.
 * - `cancelTaskById` routes by kind to the right backend command.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import type { TaskEntry } from "../../types/tasks";
import type { AiSession } from "../../types";

// ---------------------------------------------------------------------------
// Tauri event listener mock — exposes a `triggerUpdate(payload)` hook so the
// tests can pretend a Rust-side `task://update` fired.
// ---------------------------------------------------------------------------

type TaskEventPayload = {
  id: string;
  kind:
    | "ai_background"
    | "ai_interactive"
    | "git_fetch"
    | "git_pull"
    | "git_push"
    | "git_clone"
    | "app_update";
  title: string;
  subtitle?: string;
  started_at_ms: number;
  finished_at_ms?: number;
  status: "running" | "success" | "error" | "cancelled";
  progress?: {
    determinate: boolean;
    current?: number;
    total?: number;
    percent?: number;
  };
  error_message?: string;
};

type Listener = (ev: { payload: TaskEventPayload }) => void;
const listeners = new Set<Listener>();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_name: string, cb: Listener) => {
    listeners.add(cb);
    return () => listeners.delete(cb);
  }),
  emit: vi.fn(),
  once: vi.fn(),
}));

function triggerUpdate(payload: TaskEventPayload): void {
  for (const cb of listeners) cb({ payload });
}

// ---------------------------------------------------------------------------
// Producer-command mocks
// ---------------------------------------------------------------------------

const taskCancelMock = vi.fn(async (_id: string) => {
  void _id;
});
const aiBackgroundCancelMock = vi.fn(async (_id: string) => {
  void _id;
});
const terminalKillMock = vi.fn(async (_id: number) => {
  void _id;
});

vi.mock("../../api/tauri", async () => {
  const actual =
    await vi.importActual<typeof import("../../api/tauri")>("../../api/tauri");
  return {
    ...actual,
    taskCancel: (id: string) => taskCancelMock(id),
    aiCancelBackgroundRun: (id: string) => aiBackgroundCancelMock(id),
    terminalKill: (id: number) => terminalKillMock(id),
  };
});

// ---------------------------------------------------------------------------
// rAF shim so coalesced updates flush synchronously inside tests.
// ---------------------------------------------------------------------------

beforeEach(() => {
  listeners.clear();
  vi.resetModules();
  taskCancelMock.mockClear();
  aiBackgroundCancelMock.mockClear();
  terminalKillMock.mockClear();

  // Execute rAF callbacks synchronously so tests don't race the tick.
  vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
    cb(0);
    return 0;
  });
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date("2026-04-20T12:00:00Z"));
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeEvent(over: Partial<TaskEventPayload> = {}): TaskEventPayload {
  return {
    id: "1",
    kind: "git_fetch",
    title: "Fetch origin",
    started_at_ms: Date.now(),
    status: "running",
    ...over,
  };
}

describe("tasks aggregator store", () => {
  it("starts empty", async () => {
    const mod = await import("../tasks");
    await mod.initTasksStore();
    expect(get(mod.tasksStore)).toEqual([]);
    expect(get(mod.activeTaskCount)).toBe(0);
    expect(get(mod.hasUnseenError)).toBe(false);
  });

  it("maps a git_fetch Rust event to a TaskEntry", async () => {
    const mod = await import("../tasks");
    await mod.initTasksStore();

    triggerUpdate(
      makeEvent({ id: "42", kind: "git_fetch", title: "Fetch origin" }),
    );

    const entries = get(mod.tasksStore);
    expect(entries).toHaveLength(1);
    expect(entries[0].id).toBe("42");
    expect(entries[0].kind).toBe("git_fetch");
    expect(entries[0].title).toBe("Fetch origin");
    expect(entries[0].status).toBe("running");
    expect(entries[0].actions.some((a) => a.id === "cancel")).toBe(true);
  });

  it("upserts same-id events in place", async () => {
    const mod = await import("../tasks");
    await mod.initTasksStore();

    triggerUpdate(makeEvent({ id: "1", status: "running" }));
    triggerUpdate(
      makeEvent({
        id: "1",
        status: "success",
        finished_at_ms: Date.now() + 1_000,
      }),
    );

    const entries = get(mod.tasksStore);
    expect(entries).toHaveLength(1);
    expect(entries[0].status).toBe("success");
    expect(entries[0].finishedAt).toBeGreaterThan(entries[0].startedAt);
  });

  it("bridges an aiBackgroundRuns entry into the feed", async () => {
    const mod = await import("../tasks");
    const { aiBackgroundRuns, upsertAiBackgroundRun } = await import(
      "../aiBackground"
    );
    await mod.initTasksStore();

    const session: AiSession = {
      id: "sess-1",
      provider: "claude_code",
      cwd: "/tmp/wt",
      started_at: Date.now(),
      kind: "headless",
      is_active: true,
      worktree_path: "/tmp/wt",
      background_status: { state: "running" },
      task_id: 7,
    };
    upsertAiBackgroundRun(session);

    const entries = get(mod.tasksStore);
    const aiEntry = entries.find(
      (e: TaskEntry) => e.id === "ai-background:sess-1",
    );
    expect(aiEntry).toBeDefined();
    expect(aiEntry!.kind).toBe("ai_background");
    expect(aiEntry!.status).toBe("running");

    // Flush the noop aiBackgroundRuns write
    void aiBackgroundRuns;
  });

  it("activeTaskCount counts only running entries", async () => {
    const mod = await import("../tasks");
    await mod.initTasksStore();

    triggerUpdate(makeEvent({ id: "a", status: "running" }));
    triggerUpdate(makeEvent({ id: "b", status: "success" }));
    triggerUpdate(makeEvent({ id: "c", status: "running" }));

    expect(get(mod.activeTaskCount)).toBe(2);
  });

  it("recentlyFinishedTasks excludes entries finished > 5 min ago", async () => {
    const mod = await import("../tasks");
    await mod.initTasksStore();

    const now = Date.now();
    triggerUpdate(
      makeEvent({
        id: "old",
        status: "success",
        started_at_ms: now - 10 * 60_000,
        finished_at_ms: now - 6 * 60_000,
      }),
    );
    triggerUpdate(
      makeEvent({
        id: "new",
        status: "success",
        started_at_ms: now - 2 * 60_000,
        finished_at_ms: now - 1 * 60_000,
      }),
    );

    const finished = get(mod.recentlyFinishedTasks);
    expect(finished.map((e: TaskEntry) => e.id)).toEqual(["new"]);
  });

  it("hasUnseenError flips on error then clears on markSeen()", async () => {
    const mod = await import("../tasks");
    await mod.initTasksStore();

    triggerUpdate(
      makeEvent({
        id: "bad",
        status: "error",
        finished_at_ms: Date.now(),
        error_message: "boom",
      }),
    );

    expect(get(mod.hasUnseenError)).toBe(true);
    mod.markSeen();
    expect(get(mod.hasUnseenError)).toBe(false);
  });

  it("clearFinished removes terminal entries but keeps running ones", async () => {
    const mod = await import("../tasks");
    await mod.initTasksStore();

    triggerUpdate(makeEvent({ id: "running-1", status: "running" }));
    triggerUpdate(
      makeEvent({
        id: "done-1",
        status: "success",
        finished_at_ms: Date.now(),
      }),
    );

    expect(get(mod.tasksStore)).toHaveLength(2);
    mod.clearFinished();
    const remaining = get(mod.tasksStore);
    expect(remaining).toHaveLength(1);
    expect(remaining[0].id).toBe("running-1");
  });

  it("cancelTaskById routes git_fetch → taskCancel", async () => {
    const mod = await import("../tasks");
    await mod.initTasksStore();

    triggerUpdate(makeEvent({ id: "99", kind: "git_fetch" }));
    await mod.cancelTaskById("99");

    expect(taskCancelMock).toHaveBeenCalledWith("99");
    expect(aiBackgroundCancelMock).not.toHaveBeenCalled();
    expect(terminalKillMock).not.toHaveBeenCalled();
  });

  it("cancelTaskById routes git_push / git_pull / git_clone through taskCancel", async () => {
    const mod = await import("../tasks");
    await mod.initTasksStore();

    triggerUpdate(makeEvent({ id: "101", kind: "git_push" }));
    triggerUpdate(makeEvent({ id: "102", kind: "git_pull" }));
    triggerUpdate(makeEvent({ id: "103", kind: "git_clone" }));

    await mod.cancelTaskById("101");
    await mod.cancelTaskById("102");
    await mod.cancelTaskById("103");

    expect(taskCancelMock).toHaveBeenCalledTimes(3);
    expect(taskCancelMock).toHaveBeenNthCalledWith(1, "101");
    expect(taskCancelMock).toHaveBeenNthCalledWith(2, "102");
    expect(taskCancelMock).toHaveBeenNthCalledWith(3, "103");
  });

  it("cancelTaskById routes ai_background → aiCancelBackgroundRun", async () => {
    const mod = await import("../tasks");
    const { upsertAiBackgroundRun } = await import("../aiBackground");
    await mod.initTasksStore();

    upsertAiBackgroundRun({
      id: "sess-42",
      provider: "claude_code",
      cwd: "/tmp",
      started_at: Date.now(),
      kind: "headless",
      is_active: true,
      worktree_path: "/tmp",
      background_status: { state: "running" },
      task_id: 3,
    });

    await mod.cancelTaskById("ai-background:sess-42");

    expect(aiBackgroundCancelMock).toHaveBeenCalledWith("sess-42");
    expect(taskCancelMock).not.toHaveBeenCalled();
    expect(terminalKillMock).not.toHaveBeenCalled();
  });

  it("cancelTaskById routes ai_interactive → terminalKill", async () => {
    const mod = await import("../tasks");
    await mod.initTasksStore();

    triggerUpdate(
      makeEvent({
        id: "77",
        kind: "ai_interactive",
        title: "Claude interactive",
      }),
    );
    await mod.cancelTaskById("77");

    expect(terminalKillMock).toHaveBeenCalledWith(77);
    expect(taskCancelMock).not.toHaveBeenCalled();
  });

  it("cancelTaskById routes app_update → autoUpdate.cancelUpdateDownload", async () => {
    const mod = await import("../tasks");
    const autoUpdate = await import("../autoUpdate");
    await mod.initTasksStore();

    const spy = vi.spyOn(autoUpdate, "cancelUpdateDownload");

    // Prime the feed via the autoUpdate bridge.
    autoUpdate.autoUpdateState.set({
      status: "downloading",
      totalBytes: 1000,
      downloadedBytes: 100,
    });

    await mod.cancelTaskById("auto-update");

    expect(spy).toHaveBeenCalledTimes(1);
  });

  it("cancelTaskById is a no-op for unknown ids", async () => {
    const mod = await import("../tasks");
    await mod.initTasksStore();

    await mod.cancelTaskById("nonexistent");

    expect(taskCancelMock).not.toHaveBeenCalled();
    expect(aiBackgroundCancelMock).not.toHaveBeenCalled();
    expect(terminalKillMock).not.toHaveBeenCalled();
  });
});
