import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

import {
  sessions,
  refreshSessions,
  filterSessionsByProject,
  clearSessionState,
  mergedSessions,
  selectedSession,
} from "./aiSessions";
import {
  aiBackgroundRuns,
  selectedBackgroundSessionId,
} from "./aiBackground";
import type { AiSession } from "$lib/types";

describe("aiSessions store", () => {
  const mockSessions: AiSession[] = [
    {
      id: "s1",
      provider: "claude_code",
      cwd: "/repo",
      started_at: 1000,
      kind: "interactive",
      is_active: true,
    },
    {
      id: "s2",
      provider: "claude_code",
      cwd: "/other-repo",
      started_at: 2000,
      kind: "headless",
      is_active: true,
    },
    {
      id: "s3",
      provider: "claude_code",
      cwd: "/repo",
      started_at: 500,
      kind: "interactive",
      is_active: false,
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    clearSessionState();
  });

  it("filterSessionsByProject returns only matching sessions", () => {
    const filtered = filterSessionsByProject(mockSessions, "/repo");
    expect(filtered).toHaveLength(2);
    expect(filtered.every((s) => s.cwd.startsWith("/repo"))).toBe(true);
  });

  it("filterSessionsByProject normalizes trailing slashes", () => {
    const sessionsWithSlash: AiSession[] = [
      { id: "s4", provider: "claude_code", cwd: "/repo/", started_at: 3000, kind: "interactive", is_active: true },
    ];
    const filtered = filterSessionsByProject(sessionsWithSlash, "/repo");
    expect(filtered).toHaveLength(1);
    expect(filtered[0].id).toBe("s4");
  });

  it("filterSessionsByProject matches subdirectory sessions", () => {
    const sessionsWithSubdir: AiSession[] = [
      { id: "s5", provider: "claude_code", cwd: "/repo/worktree-1", started_at: 4000, kind: "interactive", is_active: true },
    ];
    const filtered = filterSessionsByProject(sessionsWithSubdir, "/repo");
    expect(filtered).toHaveLength(1);
    expect(filtered[0].id).toBe("s5");
  });

  it("sorts active sessions before ended", () => {
    const filtered = filterSessionsByProject(mockSessions, "/repo");
    expect(filtered[0].is_active).toBe(true);
    expect(filtered[1].is_active).toBe(false);
  });

  it("refreshSessions filters by project path", async () => {
    mockInvoke.mockResolvedValueOnce(mockSessions);
    await refreshSessions("/repo");
    const result = get(sessions);
    expect(result).toHaveLength(2);
    expect(result[0].id).toBe("s1");
  });

  it("refreshSessions returns all sessions when no project path", async () => {
    mockInvoke.mockResolvedValueOnce(mockSessions);
    await refreshSessions();
    const result = get(sessions);
    expect(result).toHaveLength(3);
  });

  it("refreshSessions clears sessions on error", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("fail"));
    await refreshSessions("/repo");
    const result = get(sessions);
    expect(result).toHaveLength(0);
  });

  it("clearSessionState resets store", () => {
    sessions.set(mockSessions);
    clearSessionState();
    expect(get(sessions)).toHaveLength(0);
  });
});

describe("mergedSessions (provider, cwd) dedupe", () => {
  beforeEach(() => {
    sessions.set([]);
    aiBackgroundRuns.set(new Map());
    selectedBackgroundSessionId.set(null);
  });

  it("collapses two active-interactive rows sharing (provider, cwd)", () => {
    // Two provider-reported entries with different ids but same process.
    sessions.set([
      {
        id: "pid-123",
        provider: "claude_code",
        cwd: "/repo",
        started_at: 1000,
        kind: "interactive",
        is_active: true,
      },
      {
        id: "claude-uuid-abc",
        provider: "claude_code",
        cwd: "/repo/",
        started_at: 2000,
        kind: "interactive",
        is_active: true,
      },
    ]);
    const list = get(mergedSessions);
    expect(list).toHaveLength(1);
    // Newer started_at wins the tie-break.
    expect(list[0].id).toBe("claude-uuid-abc");
  });

  it("prefers the bg-run entry over a PID-discovered sibling", () => {
    const bgRun: AiSession = {
      id: "bg-1",
      provider: "claude_code",
      cwd: "/repo",
      started_at: 500, // older than the PID entry below
      kind: "interactive",
      is_active: true,
      worktree_path: "/repo/.wt/ai-bg-1",
      background_status: { state: "running" },
      task_id: 7,
    };
    aiBackgroundRuns.set(new Map([[bgRun.id, bgRun]]));
    sessions.set([
      {
        id: "pid-999",
        provider: "claude_code",
        cwd: "/repo",
        started_at: 9000, // newer, but PID entry loses to bg-run
        kind: "interactive",
        is_active: true,
      },
    ]);
    const list = get(mergedSessions);
    expect(list).toHaveLength(1);
    expect(list[0].id).toBe("bg-1");
  });

  it("keeps ended and headless rows with the same cwd independent", () => {
    sessions.set([
      {
        id: "ended-1",
        provider: "claude_code",
        cwd: "/repo",
        started_at: 1,
        kind: "interactive",
        is_active: false,
      },
      {
        id: "ended-2",
        provider: "claude_code",
        cwd: "/repo",
        started_at: 2,
        kind: "interactive",
        is_active: false,
      },
      {
        id: "headless-1",
        provider: "claude_code",
        cwd: "/repo",
        started_at: 3,
        kind: "headless",
        is_active: true,
      },
      {
        id: "headless-2",
        provider: "claude_code",
        cwd: "/repo",
        started_at: 4,
        kind: "headless",
        is_active: true,
      },
    ]);
    const list = get(mergedSessions);
    expect(list.map((s) => s.id).sort()).toEqual([
      "ended-1",
      "ended-2",
      "headless-1",
      "headless-2",
    ]);
  });

  it("does NOT collapse rows across different providers or cwds", () => {
    sessions.set([
      {
        id: "claude-a",
        provider: "claude_code",
        cwd: "/repo",
        started_at: 1,
        kind: "interactive",
        is_active: true,
      },
      {
        id: "codex-a",
        provider: "codex",
        cwd: "/repo",
        started_at: 2,
        kind: "interactive",
        is_active: true,
      },
      {
        id: "claude-b",
        provider: "claude_code",
        cwd: "/other-repo",
        started_at: 3,
        kind: "interactive",
        is_active: true,
      },
    ]);
    expect(get(mergedSessions)).toHaveLength(3);
  });
});

describe("selectedSession", () => {
  beforeEach(() => {
    sessions.set([]);
    aiBackgroundRuns.set(new Map());
    selectedBackgroundSessionId.set(null);
  });

  it("returns null when no id is selected", () => {
    sessions.set([
      {
        id: "s1",
        provider: "claude_code",
        cwd: "/repo",
        started_at: 1,
        kind: "interactive",
        is_active: true,
      },
    ]);
    expect(get(selectedSession)).toBeNull();
  });

  it("resolves provider-reported sessions (bugfix: empty detail pane)", () => {
    const s: AiSession = {
      id: "external-pid",
      provider: "claude_code",
      cwd: "/repo",
      started_at: 1,
      kind: "interactive",
      is_active: true,
    };
    sessions.set([s]);
    selectedBackgroundSessionId.set("external-pid");
    expect(get(selectedSession)?.id).toBe("external-pid");
  });

  it("resolves background-run sessions through the same derive", () => {
    const bg: AiSession = {
      id: "bg-x",
      provider: "codex",
      cwd: "/repo",
      started_at: 1,
      kind: "headless",
      is_active: true,
      worktree_path: "/repo/.wt/ai-x",
      background_status: { state: "running" },
    };
    aiBackgroundRuns.set(new Map([[bg.id, bg]]));
    selectedBackgroundSessionId.set("bg-x");
    expect(get(selectedSession)?.id).toBe("bg-x");
  });

  it("returns null when the id doesn't match any session", () => {
    selectedBackgroundSessionId.set("ghost");
    expect(get(selectedSession)).toBeNull();
  });
});
