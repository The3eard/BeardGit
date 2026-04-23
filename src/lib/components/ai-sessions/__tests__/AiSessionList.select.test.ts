/**
 * AiSessionList — mutually-exclusive selection contract.
 *
 *   - Clicking a conversation row writes `selectedConversationId` and
 *     clears `selectedBackgroundSessionId`.
 *   - Clicking an active bg-run row writes `selectedBackgroundSessionId`
 *     via `focusTerminal` (kind === "bg"). The List component itself
 *     delegates to `focusTerminal` without touching either selection
 *     store — the "clear selectedConversationId so the bg detail surfaces"
 *     behaviour lives inside the real `focusTerminal` and is tested in
 *     `aiConversationActions.test.ts`.
 *   - Clicking the Focus button on a tab/segment row calls
 *     `focusTerminal` without mutating either selection store.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";
import { tick } from "svelte";
import type { AiConversation, AiSession, Tab } from "$lib/types";

const { focusTerminal } = vi.hoisted(() => ({
  focusTerminal: vi.fn(() => true),
}));

vi.mock("$lib/stores/aiConversationActions", async () => {
  const actual = await vi.importActual<object>(
    "$lib/stores/aiConversationActions",
  );
  return { ...actual, focusTerminal };
});

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

function resetStores() {
  conversations.set([]);
  conversationsLoading.set(false);
  selectedConversationId.set(null);
  aiBackgroundRuns.set(new Map());
  selectedBackgroundSessionId.set(null);
  openTabs.set([]);
  activeTabIndex.set(-1);
  focusTerminal.mockClear();
}

beforeEach(resetStores);
afterEach(() => {
  cleanup();
  resetStores();
});

describe("AiSessionList selection", () => {
  it("clicking a conversation row selects it and clears bg selection", async () => {
    selectedBackgroundSessionId.set("some-bg");
    conversations.set([CONVERSATION]);

    const { container } = render(AiSessionList);
    await tick();

    const row = container.querySelector(
      '[data-testid="ai-conversation-row"]',
    ) as HTMLElement;
    expect(row).toBeTruthy();
    await fireEvent.click(row);

    expect(get(selectedConversationId)).toBe(CONVERSATION.id);
    expect(get(selectedBackgroundSessionId)).toBeNull();
  });

  it("clicking a bg-run active-row Focus button routes through focusTerminal", async () => {
    aiBackgroundRuns.set(new Map([[BG_RUN.id, BG_RUN]]));
    selectedConversationId.set("some-conv");

    const { container } = render(AiSessionList);
    await tick();

    const focusBtn = container.querySelector(
      '[data-testid="ai-active-row-focus"]',
    ) as HTMLElement;
    expect(focusBtn).toBeTruthy();
    await fireEvent.click(focusBtn);

    expect(focusTerminal).toHaveBeenCalledTimes(1);
    const call = focusTerminal.mock.calls[0] as unknown as [
      { kind: "bg"; session: AiSession },
    ];
    expect(call?.[0]?.kind).toBe("bg");
    expect(call?.[0]?.session.id).toBe(BG_RUN.id);
    // Conversation selection is a separate concern — focusTerminal on a bg
    // row shouldn't alter it.
    expect(get(selectedConversationId)).toBe("some-conv");
  });

  it("clicking Focus on a tab row calls focusTerminal without touching selection", async () => {
    const tab: Tab = {
      kind: "terminal",
      terminal: {
        sessionId: 42,
        title: "Claude",
        cwd: "/repos/demo",
        provider: "claude_code",
      },
    };
    openTabs.set([tab]);
    selectedConversationId.set("keep-me");
    selectedBackgroundSessionId.set("keep-me-too");

    const { container } = render(AiSessionList);
    await tick();

    const focusBtn = container.querySelector(
      '[data-testid="ai-active-row-focus"]',
    ) as HTMLElement;
    await fireEvent.click(focusBtn);

    expect(focusTerminal).toHaveBeenCalledTimes(1);
    expect(get(selectedConversationId)).toBe("keep-me");
    expect(get(selectedBackgroundSessionId)).toBe("keep-me-too");
  });
});
