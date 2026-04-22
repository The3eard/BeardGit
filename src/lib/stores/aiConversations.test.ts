/**
 * Tests for the transcript-first AI conversations store.
 *
 * Mirrors the testing approach in `aiSessions.test.ts`: mock the Tauri
 * invoke + listen entry points and drive `refreshConversations` through
 * its main paths (no project, with project, error). Also covers the
 * selectedConversation derive, dismiss, clearConversationState, and the
 * `filterConversationsByProject` helper's normalisation rules.
 */
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
  conversations,
  conversationsLoading,
  selectedConversation,
  selectedConversationId,
  refreshConversations,
  filterConversationsByProject,
  dismissConversation,
  clearConversationState,
} from "./aiConversations";
import type { AiConversation } from "$lib/types";

function makeConv(
  id: string,
  cwd: string,
  last_activity_at: number,
): AiConversation {
  return {
    id,
    provider: "claude_code",
    cwd,
    created_at: last_activity_at - 1000,
    last_activity_at,
    title: `Conversation ${id}`,
  };
}

describe("aiConversations store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    conversations.set([]);
    conversationsLoading.set(true);
    selectedConversationId.set(null);
  });

  it("conversationsLoading defaults to true", () => {
    // Freshly reset in beforeEach — verifies the documented default.
    expect(get(conversationsLoading)).toBe(true);
  });

  it("refreshConversations with no project path stores the fetched list", async () => {
    const list: AiConversation[] = [
      makeConv("a", "/repo", 3000),
      makeConv("b", "/other", 2000),
    ];
    mockInvoke.mockResolvedValueOnce(list);
    await refreshConversations();
    expect(get(conversations)).toHaveLength(2);
    expect(get(conversationsLoading)).toBe(false);
  });

  it("refreshConversations with project path filters by cwd", async () => {
    const list: AiConversation[] = [
      makeConv("in-repo", "/repo", 3000),
      makeConv("elsewhere", "/elsewhere", 2000),
      makeConv("in-worktree", "/repo/.wt/feature", 1000),
    ];
    mockInvoke.mockResolvedValueOnce(list);
    await refreshConversations("/repo");
    const ids = get(conversations).map((c) => c.id);
    expect(ids).toEqual(["in-repo", "in-worktree"]);
  });

  it("refreshConversations re-sorts by last_activity_at descending", async () => {
    // Rust already sorts, but we defensively re-sort after filtering.
    // Provide an out-of-order input to prove it.
    const list: AiConversation[] = [
      makeConv("old", "/repo", 1000),
      makeConv("new", "/repo", 9000),
      makeConv("mid", "/repo", 5000),
    ];
    mockInvoke.mockResolvedValueOnce(list);
    await refreshConversations("/repo");
    const ids = get(conversations).map((c) => c.id);
    expect(ids).toEqual(["new", "mid", "old"]);
  });

  it("refreshConversations clears the list and loading flag on error", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("boom"));
    // Seed with content so we can verify it gets cleared.
    conversations.set([makeConv("stale", "/repo", 1)]);
    await refreshConversations("/repo");
    expect(get(conversations)).toEqual([]);
    expect(get(conversationsLoading)).toBe(false);
  });

  it("conversationsLoading flips true → false across a refresh", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    conversationsLoading.set(true);
    await refreshConversations();
    expect(get(conversationsLoading)).toBe(false);
  });

  it("selectedConversation resolves to the matching row", () => {
    const conv = makeConv("target", "/repo", 1);
    conversations.set([conv]);
    selectedConversationId.set("target");
    expect(get(selectedConversation)?.id).toBe("target");
  });

  it("selectedConversation returns null for an unknown id", () => {
    conversations.set([makeConv("a", "/repo", 1)]);
    selectedConversationId.set("ghost");
    expect(get(selectedConversation)).toBeNull();
  });

  it("selectedConversation returns null when no id selected", () => {
    conversations.set([makeConv("a", "/repo", 1)]);
    selectedConversationId.set(null);
    expect(get(selectedConversation)).toBeNull();
  });

  it("dismissConversation removes only the matching id", () => {
    conversations.set([
      makeConv("keep-1", "/repo", 3),
      makeConv("drop", "/repo", 2),
      makeConv("keep-2", "/repo", 1),
    ]);
    dismissConversation("drop");
    expect(get(conversations).map((c) => c.id)).toEqual(["keep-1", "keep-2"]);
  });

  it("clearConversationState empties the list and resets loading + selection", () => {
    conversations.set([makeConv("a", "/repo", 1)]);
    conversationsLoading.set(true);
    selectedConversationId.set("a");
    clearConversationState();
    expect(get(conversations)).toEqual([]);
    expect(get(conversationsLoading)).toBe(false);
    expect(get(selectedConversationId)).toBeNull();
  });
});

describe("filterConversationsByProject", () => {
  it("matches exact cwd", () => {
    const out = filterConversationsByProject(
      [makeConv("a", "/repo", 1)],
      "/repo",
    );
    expect(out).toHaveLength(1);
  });

  it("normalises trailing slashes on both sides", () => {
    const out = filterConversationsByProject(
      [
        makeConv("cwd-slash", "/repo/", 1),
        makeConv("no-slash", "/repo", 2),
      ],
      "/repo/",
    );
    expect(out.map((c) => c.id).sort()).toEqual(["cwd-slash", "no-slash"]);
  });

  it("matches subdirectory cwds (worktrees)", () => {
    const out = filterConversationsByProject(
      [
        makeConv("wt", "/repo/.wt/feature", 1),
        makeConv("nested", "/repo/sub/dir", 2),
      ],
      "/repo",
    );
    expect(out).toHaveLength(2);
  });

  it("filters out non-matching cwds", () => {
    const out = filterConversationsByProject(
      [
        makeConv("far", "/elsewhere", 1),
        makeConv("prefix-collision", "/repository-two", 2),
      ],
      "/repo",
    );
    expect(out).toEqual([]);
  });
});
