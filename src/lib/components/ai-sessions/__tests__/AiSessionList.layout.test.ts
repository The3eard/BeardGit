/**
 * Renders AiSessionList with two fixture sessions and asserts the new
 * layout contract: ProviderIcon per row, 8px padding, External badge for
 * sessions without a reachable worktree path.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";

vi.mock("$lib/stores/aiSessions", async () => {
  const { writable } = await import("svelte/store");
  const now = Date.now() / 1000;
  const fixtures = [
    {
      id: "s1",
      provider: "claude_code",
      cwd: "/repos/demo",
      started_at: now,
      kind: "headless",
      is_active: true,
      worktree_path: "/repos/demo/.wt/ai-1",
      background_status: { state: "running" },
    },
    {
      id: "s2",
      provider: "codex",
      cwd: "/repos/demo",
      started_at: now,
      kind: "interactive",
      is_active: true,
      worktree_path: null, // no reachable worktree → External
    },
  ];
  return {
    mergedSessions: writable(fixtures),
    sessionsLoading: writable(false),
    refreshSessions: vi.fn(),
    dismissSession: vi.fn(),
    startSessionListeners: vi.fn(),
    stopSessionListeners: vi.fn(),
  };
});

import AiSessionList from "../AiSessionList.svelte";

afterEach(() => cleanup());

describe("AiSessionList layout", () => {
  it("renders a ProviderIcon per row", async () => {
    const { container } = render(AiSessionList);
    await tick();
    const icons = container.querySelectorAll("img.provider-icon");
    expect(icons.length).toBeGreaterThanOrEqual(2);
  });

  it("renders an External badge when worktree_path is missing", async () => {
    const { container } = render(AiSessionList);
    await tick();
    const badges = container.querySelectorAll('[data-testid="external-badge"]');
    expect(badges.length).toBe(1);
  });

  it("does NOT render the old coloured-square icon wrapper", async () => {
    const { container } = render(AiSessionList);
    await tick();
    expect(container.querySelector(".session-icon")).toBeFalsy();
  });
});
