/**
 * ConversationRow — wraps `SessionRow` with conversation-payload derivations.
 *
 * After the list-trim refactor the row renders ONLY:
 *   [provider icon] [title] [relative date]
 *
 * The Resume button, provider name, cwd, and forked badge are gone from
 * the row — they live on the detail pane now. These tests lock in that
 * minimalism and verify the mutually-exclusive selection contract
 * (clicking a conversation clears bg + active-terminal selection).
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";
import { tick } from "svelte";
import type { AiConversation } from "$lib/types";

import ConversationRow from "../ConversationRow.svelte";
import { selectedConversationId } from "$lib/stores/aiConversations";
import { selectedBackgroundSessionId } from "$lib/stores/aiBackground";
import {
  selectedActiveTerminal,
  type ActiveTerminal,
} from "$lib/stores/aiActiveTerminals";

const BASE: AiConversation = {
  id: "abcdef1234",
  provider: "claude_code",
  cwd: "/repos/demo",
  created_at: 1_700_000_000_000,
  last_activity_at: Date.now(),
  title: "Investigate the bug in the login flow",
};

const TAB_ACTIVE: ActiveTerminal = {
  kind: "tab",
  tabIndex: 0,
  info: {
    sessionId: 1,
    title: "Claude",
    cwd: "/repos/demo",
    provider: "claude_code",
  },
};

beforeEach(() => {
  selectedConversationId.set(null);
  selectedBackgroundSessionId.set(null);
  selectedActiveTerminal.set(null);
});
afterEach(() => cleanup());

describe("ConversationRow (trimmed)", () => {
  it("renders only title + relative date (no provider name, cwd, forked badge, resume button)", async () => {
    const { container } = render(ConversationRow, {
      props: { conversation: { ...BASE, parent_id: "cafed00d" } },
    });
    await tick();

    expect(container.textContent).toContain("Investigate the bug in the login flow");
    // Relative time util yields a stable suffix for anything recent.
    expect(container.textContent?.toLowerCase()).toMatch(
      /just now|ago/,
    );

    // None of the removed artefacts should survive.
    expect(
      container.querySelector('[data-testid="ai-conversation-row-forked"]'),
    ).toBeNull();
    expect(
      container.querySelector('[data-testid="ai-conversation-row-resume"]'),
    ).toBeNull();
    expect(container.textContent).not.toContain("Claude Code");
    expect(container.textContent).not.toContain("demo");
  });

  it("falls back to (no title) when title is empty", async () => {
    const { container } = render(ConversationRow, {
      props: { conversation: { ...BASE, title: "" } },
    });
    await tick();
    expect(container.textContent).toContain("(no title)");
  });

  it("row click selects conversation and clears bg + active-terminal", async () => {
    selectedBackgroundSessionId.set("some-bg-id");
    selectedActiveTerminal.set(TAB_ACTIVE);

    const { container } = render(ConversationRow, {
      props: { conversation: BASE },
    });
    await tick();

    const row = container.querySelector(
      '[data-testid="ai-conversation-row"]',
    ) as HTMLElement;
    await fireEvent.click(row);

    expect(get(selectedConversationId)).toBe(BASE.id);
    expect(get(selectedBackgroundSessionId)).toBeNull();
    expect(get(selectedActiveTerminal)).toBeNull();
  });

  it("applies selected styling when this conversation is selected", async () => {
    selectedConversationId.set(BASE.id);
    const { container } = render(ConversationRow, {
      props: { conversation: BASE },
    });
    await tick();
    const row = container.querySelector(".session-row") as HTMLElement;
    expect(row.classList.contains("selected")).toBe(true);
  });
});
