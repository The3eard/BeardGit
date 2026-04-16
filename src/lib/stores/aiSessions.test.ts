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
} from "./aiSessions";
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
    expect(filtered.every((s) => s.cwd === "/repo")).toBe(true);
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
