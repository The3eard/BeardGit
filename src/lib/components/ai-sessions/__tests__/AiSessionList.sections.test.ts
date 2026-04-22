/**
 * AiSessionList — two-section layout contract.
 *
 * Asserts:
 *   - Empty stores render both empty-state messages + zero counts.
 *   - One conversation + one active tab populate the corresponding
 *     sections and the row counts reflect the data.
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { AiConversation, Tab } from "$lib/types";

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

function resetStores() {
  conversations.set([]);
  conversationsLoading.set(false);
  selectedConversationId.set(null);
  aiBackgroundRuns.set(new Map());
  selectedBackgroundSessionId.set(null);
  openTabs.set([]);
  activeTabIndex.set(-1);
}

beforeEach(resetStores);
afterEach(() => {
  cleanup();
  resetStores();
});

describe("AiSessionList sections", () => {
  it("renders both empty states when stores are empty", async () => {
    const { getByTestId } = render(AiSessionList);
    await tick();

    expect(getByTestId("ai-session-list-active-empty")).toBeTruthy();
    expect(getByTestId("ai-session-list-conversations-empty")).toBeTruthy();
    expect(getByTestId("ai-session-list-active-count").textContent?.trim()).toBe(
      "0",
    );
    expect(
      getByTestId("ai-session-list-conversations-count").textContent?.trim(),
    ).toBe("0");
  });

  it("renders one row per section when each store has one entry", async () => {
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
    activeTabIndex.set(0);

    const conversation: AiConversation = {
      id: "conv-1",
      provider: "claude_code",
      cwd: "/repos/demo",
      created_at: 1_700_000_000_000,
      last_activity_at: 1_700_000_000_000,
      title: "Fix the login flow",
    };
    conversations.set([conversation]);

    const { container, getByTestId } = render(AiSessionList);
    await tick();

    expect(getByTestId("ai-session-list-active-count").textContent?.trim()).toBe(
      "1",
    );
    expect(
      getByTestId("ai-session-list-conversations-count").textContent?.trim(),
    ).toBe("1");

    const activeRows = container.querySelectorAll(
      '[data-testid="ai-active-row"]',
    );
    expect(activeRows.length).toBe(1);

    const convRows = container.querySelectorAll(
      '[data-testid="ai-conversation-row"]',
    );
    expect(convRows.length).toBe(1);
    expect(container.textContent).toContain("Fix the login flow");
  });
});
