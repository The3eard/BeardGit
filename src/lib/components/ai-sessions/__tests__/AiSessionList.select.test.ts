/**
 * AiSessionList — mutually-exclusive selection contract across the three
 * selection stores:
 *   - `selectedConversationId`
 *   - `selectedBackgroundSessionId`
 *   - `selectedActiveTerminal`
 *
 * Each row's `onSelect` goes through `selectAiSessionRow`, which sets the
 * appropriate store and clears the other two. The three tests below cover
 * every pair-wise transition end-to-end through the real list component.
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";
import { tick } from "svelte";
import type { AiConversation, AiSession, Tab } from "$lib/types";

import AiSessionList from "../AiSessionList.svelte";
import {
  conversations,
  conversationsLoading,
  selectedConversationId,
} from "$lib/stores/aiConversations";
import {
  aiBackgroundRuns,
  selectedBackgroundSessionId,
} from "$lib/stores/aiBackground";
import { selectedActiveTerminal } from "$lib/stores/aiActiveTerminals";
import { openTabs, activeTabIndex } from "$lib/stores/tabs";

const CONVERSATION: AiConversation = {
  id: "conv-1",
  provider: "claude_code",
  cwd: "/repos/demo",
  created_at: 1_700_000_000_000,
  last_activity_at: 1_700_000_000_000,
  title: "Fix the login flow",
};

const BG_RUN: AiSession = {
  id: "bg-1",
  provider: "claude_code",
  cwd: "/repos/demo",
  started_at: 1_700_000_000,
  kind: "headless",
  is_active: true,
  worktree_path: "/repos/demo/.wt/ai-bg",
  background_status: { state: "running" },
};

const TAB: Tab = {
  kind: "terminal",
  terminal: {
    sessionId: 42,
    title: "Claude",
    cwd: "/repos/demo",
    provider: "claude_code",
  },
};

function resetStores() {
  conversations.set([]);
  conversationsLoading.set(false);
  selectedConversationId.set(null);
  aiBackgroundRuns.set(new Map());
  selectedBackgroundSessionId.set(null);
  selectedActiveTerminal.set(null);
  openTabs.set([]);
  activeTabIndex.set(-1);
}

beforeEach(resetStores);
afterEach(() => {
  cleanup();
  resetStores();
});

describe("AiSessionList selection — mutual exclusion", () => {
  it("conversation row sets selectedConversationId and clears the other two", async () => {
    selectedBackgroundSessionId.set("some-bg");
    selectedActiveTerminal.set({ kind: "tab", tabIndex: 0, info: TAB.terminal });
    conversations.set([CONVERSATION]);

    const { container } = render(AiSessionList);
    await tick();

    const row = container.querySelector(
      '[data-testid="ai-conversation-row"] .session-row',
    ) as HTMLElement;
    await fireEvent.click(row);

    expect(get(selectedConversationId)).toBe(CONVERSATION.id);
    expect(get(selectedBackgroundSessionId)).toBeNull();
    expect(get(selectedActiveTerminal)).toBeNull();
  });

  it("tab active-row sets selectedActiveTerminal and clears conversation + bg", async () => {
    openTabs.set([TAB]);
    selectedConversationId.set("keep-nothing");
    selectedBackgroundSessionId.set("also-nothing");

    const { container } = render(AiSessionList);
    await tick();

    const row = container.querySelector(
      '[data-testid="ai-active-row"] .session-row',
    ) as HTMLElement;
    expect(row).toBeTruthy();
    await fireEvent.click(row);

    const sel = get(selectedActiveTerminal);
    expect(sel?.kind).toBe("tab");
    expect(get(selectedConversationId)).toBeNull();
    expect(get(selectedBackgroundSessionId)).toBeNull();
  });

  it("bg active-row stores the bg variant in selectedActiveTerminal", async () => {
    aiBackgroundRuns.set(new Map([[BG_RUN.id, BG_RUN]]));
    selectedConversationId.set("keep-nothing");
    selectedBackgroundSessionId.set("also-nothing");

    const { container } = render(AiSessionList);
    await tick();

    const row = container.querySelector(
      '[data-testid="ai-active-row"][data-kind="bg"] .session-row',
    ) as HTMLElement;
    expect(row).toBeTruthy();
    await fireEvent.click(row);

    const sel = get(selectedActiveTerminal);
    expect(sel?.kind).toBe("bg");
    if (sel?.kind === "bg") {
      expect(sel.session.id).toBe(BG_RUN.id);
    }
    expect(get(selectedConversationId)).toBeNull();
    expect(get(selectedBackgroundSessionId)).toBeNull();
  });
});
