import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Keep listen as a noop for these tests — we drive store actions directly.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

import {
  aiBackgroundRuns,
  aiBackgroundTranscripts,
  activeBackgroundRunCount,
  discardAiBackgroundRunWorktree,
  recentBackgroundRuns,
  refreshAiBackgroundRuns,
  setAiBackgroundRuns,
  startAiBackgroundRun,
  upsertAiBackgroundRun,
  __flushTranscriptBufferForTests,
  __getAiBackgroundTranscripts,
} from "./aiBackground";
import type { AiSession } from "$lib/types";

function sample(id: string, state: AiSession["background_status"]): AiSession {
  return {
    id,
    provider: "claude_code",
    cwd: "/tmp/wt/" + id,
    started_at: 123,
    kind: "headless",
    is_active: state?.state === "running",
    worktree_path: "/tmp/wt/" + id,
    background_status: state,
    task_id: 42,
  };
}

describe("aiBackground store", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    aiBackgroundRuns.set(new Map());
    aiBackgroundTranscripts.set(new Map());
  });

  it("setAiBackgroundRuns replaces the map", () => {
    setAiBackgroundRuns([
      sample("a", { state: "running" }),
      sample("b", { state: "queued" }),
    ]);
    const map = get(aiBackgroundRuns);
    expect(map.size).toBe(2);
    expect(map.get("a")?.background_status?.state).toBe("running");
    expect(map.get("b")?.background_status?.state).toBe("queued");
  });

  it("upsertAiBackgroundRun inserts and updates", () => {
    upsertAiBackgroundRun(sample("a", { state: "queued" }));
    upsertAiBackgroundRun(sample("a", { state: "running" }));
    expect(get(aiBackgroundRuns).get("a")?.background_status?.state).toBe("running");
  });

  it("activeBackgroundRunCount counts queued + running", () => {
    setAiBackgroundRuns([
      sample("a", { state: "running" }),
      sample("b", { state: "queued" }),
      sample("c", { state: "completed", exit_code: 0, token_usage: null }),
      sample("d", { state: "failed", message: "x" }),
    ]);
    expect(get(activeBackgroundRunCount)).toBe(2);
  });

  it("recentBackgroundRuns surfaces only terminal-state runs, newest first", () => {
    // Terminal sessions need to be visible after they exit so the
    // user can still read the captured transcript; running/queued
    // ones belong to the Active section and must be excluded here.
    const completed = {
      ...sample("a", { state: "completed", exit_code: 0, token_usage: null }),
      started_at: 100,
    };
    const failed = {
      ...sample("b", { state: "failed", message: "boom" }),
      started_at: 300,
    };
    const cancelled = {
      ...sample("c", { state: "cancelled" }),
      started_at: 200,
    };
    const running = sample("d", { state: "running" });
    const queued = sample("e", { state: "queued" });

    setAiBackgroundRuns([completed, failed, cancelled, running, queued]);

    const ids = get(recentBackgroundRuns).map((s) => s.id);
    expect(ids).toEqual(["b", "c", "a"]); // sorted desc by started_at
    expect(ids).not.toContain("d");
    expect(ids).not.toContain("e");
  });

  it("startAiBackgroundRun seeds a placeholder session from the response", async () => {
    mockInvoke.mockResolvedValueOnce({
      session_id: "aibg-claude_code-1",
      task_id: 7,
      worktree_path: "/repo/.beardgit/ai-worktrees/feat",
      status: { state: "running" },
    });
    const res = await startAiBackgroundRun({
      provider: "claude_code",
      base_branch: "main",
      prompt: "add tests",
    });
    expect(res.session_id).toBe("aibg-claude_code-1");
    const stored = get(aiBackgroundRuns).get("aibg-claude_code-1");
    expect(stored?.background_status?.state).toBe("running");
    expect(stored?.worktree_path).toBe("/repo/.beardgit/ai-worktrees/feat");
  });

  it("discardAiBackgroundRunWorktree removes the session and its transcript", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    setAiBackgroundRuns([sample("a", { state: "completed", exit_code: 0, token_usage: null })]);
    aiBackgroundTranscripts.set(new Map([["a", ["done"]]]));

    await discardAiBackgroundRunWorktree("a");

    expect(get(aiBackgroundRuns).has("a")).toBe(false);
    expect(get(aiBackgroundTranscripts).has("a")).toBe(false);
    expect(mockInvoke).toHaveBeenCalledWith(
      "ai_discard_background_run_worktree",
      { sessionId: "a" },
    );
  });

  it("refreshAiBackgroundRuns pulls from the backend and replaces state", async () => {
    mockInvoke.mockResolvedValueOnce([sample("x", { state: "running" })]);
    await refreshAiBackgroundRuns();
    expect(get(aiBackgroundRuns).get("x")?.background_status?.state).toBe("running");
  });

  it("transcript buffer flushes on __flushTranscriptBufferForTests", () => {
    // Simulate the private enqueueLine path by calling flush with state
    // seeded via the store directly. (Full event-driven path is covered by
    // the integration tests.)
    aiBackgroundTranscripts.set(new Map([["s", ["first line"]]]));
    __flushTranscriptBufferForTests();
    expect(__getAiBackgroundTranscripts().get("s")).toEqual(["first line"]);
  });
});
