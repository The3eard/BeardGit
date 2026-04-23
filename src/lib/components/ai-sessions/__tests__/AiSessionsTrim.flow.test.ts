/**
 * AI Sessions list-trim — end-to-end flow via `AiSessionList` +
 * `AiSessionDetail`.
 *
 * Stand-in for the Playwright specs described in the design doc §10.
 * The repo uses Vitest + @testing-library/svelte throughout (no
 * Playwright install), so these integration tests wire the two real
 * components to the same store state and assert the user-visible
 * behaviour: select a row → the detail pane shows the right branch with
 * the right action → clicking the action calls the right handler.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { AiConversation, Tab } from "$lib/types";

const { focusTerminal, resumeConversation } = vi.hoisted(() => ({
  focusTerminal: vi.fn(() => true),
  resumeConversation: vi.fn(async () => true),
}));

vi.mock("$lib/stores/aiConversationActions", async () => {
  const actual = await vi.importActual<object>(
    "$lib/stores/aiConversationActions",
  );
  return { ...actual, focusTerminal, resumeConversation };
});

import AiSessionsFlowHost from "./AiSessionsFlowHost.test.svelte";
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
  last_activity_at: Date.now(),
  title: "Fix the login flow",
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
  focusTerminal.mockClear();
  resumeConversation.mockClear();
}

beforeEach(resetStores);
afterEach(() => {
  cleanup();
  resetStores();
});

describe("AI Sessions flow — conversation → Resume", () => {
  it("selecting a conversation swaps the detail pane and Resume button works", async () => {
    conversations.set([CONVERSATION]);

    const { container, getByTestId } = render(AiSessionsFlowHost);
    await tick();

    // Empty state up front.
    expect(getByTestId("ai-session-detail-empty")).toBeTruthy();

    // Click the conversation row.
    const row = container.querySelector(
      '[data-testid="ai-conversation-row"] .session-row',
    ) as HTMLElement;
    await fireEvent.click(row);
    await tick();

    // Detail pane now shows the conversation branch with a Resume button.
    expect(getByTestId("ai-session-detail-conversation")).toBeTruthy();
    await fireEvent.click(getByTestId("ai-session-detail-resume"));
    expect(resumeConversation).toHaveBeenCalledTimes(1);
    expect(resumeConversation).toHaveBeenCalledWith(CONVERSATION);
  });
});

describe("AI Sessions flow — active tab → Focus", () => {
  it("selecting an active tab shows the active branch and Focus routes to focusTerminal", async () => {
    openTabs.set([TAB]);

    const { container, getByTestId } = render(AiSessionsFlowHost);
    await tick();

    const row = container.querySelector(
      '[data-testid="ai-active-row"] .session-row',
    ) as HTMLElement;
    await fireEvent.click(row);
    await tick();

    expect(getByTestId("ai-session-detail-active")).toBeTruthy();
    await fireEvent.click(getByTestId("ai-session-detail-focus"));

    expect(focusTerminal).toHaveBeenCalledTimes(1);
    const calls = focusTerminal.mock.calls as unknown as Array<[{ kind: string }]>;
    const payload = calls[0]?.[0];
    expect(payload?.kind).toBe("tab");
  });
});
