/**
 * ConversationRow — title/no-title/forked-from rendering + Resume wiring.
 *
 * Mocks `resumeConversation` so the button click doesn't try to spawn a
 * real PTY. Also asserts the row click sets `selectedConversationId`
 * and clears `selectedBackgroundSessionId`, since the list's
 * mutually-exclusive selection rule is co-owned by this row component.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";
import { tick } from "svelte";
import type { AiConversation } from "$lib/types";

const { resumeConversation } = vi.hoisted(() => ({
  resumeConversation: vi.fn(async () => true),
}));

vi.mock("$lib/stores/aiConversationActions", async () => {
  const actual = await vi.importActual<object>(
    "$lib/stores/aiConversationActions",
  );
  return { ...actual, resumeConversation };
});

import ConversationRow from "../ConversationRow.svelte";
import { selectedConversationId } from "$lib/stores/aiConversations";
import { selectedBackgroundSessionId } from "$lib/stores/aiBackground";

const BASE: AiConversation = {
  id: "abcdef1234",
  provider: "claude_code",
  cwd: "/repos/demo",
  created_at: 1_700_000_000_000,
  last_activity_at: 1_700_000_000_000,
  title: "Investigate the bug in the login flow",
};

beforeEach(() => {
  resumeConversation.mockClear();
  selectedConversationId.set(null);
  selectedBackgroundSessionId.set(null);
});

afterEach(() => cleanup());

describe("ConversationRow", () => {
  it("renders the conversation title when present", async () => {
    const { container } = render(ConversationRow, {
      props: { conversation: BASE },
    });
    await tick();
    expect(container.textContent).toContain(
      "Investigate the bug in the login flow",
    );
    // No forked-from badge on a root transcript.
    expect(
      container.querySelector('[data-testid="ai-conversation-row-forked"]'),
    ).toBeNull();
  });

  it("falls back to (no title) when title is empty", async () => {
    const { container } = render(ConversationRow, {
      props: { conversation: { ...BASE, title: "" } },
    });
    await tick();
    expect(container.textContent).toContain("(no title)");
  });

  it("renders the forked-from badge when parent_id is set", async () => {
    const { container, getByTestId } = render(ConversationRow, {
      props: { conversation: { ...BASE, parent_id: "cafed00d" } },
    });
    await tick();
    const badge = getByTestId("ai-conversation-row-forked");
    expect(badge).toBeTruthy();
    expect(container.textContent).toContain("cafed00d");
  });

  it("Resume button fires resumeConversation with the payload", async () => {
    const { getByTestId } = render(ConversationRow, {
      props: { conversation: BASE },
    });
    await tick();
    await fireEvent.click(getByTestId("ai-conversation-row-resume"));
    expect(resumeConversation).toHaveBeenCalledTimes(1);
    expect(resumeConversation).toHaveBeenCalledWith(BASE);
  });

  it("row click sets selectedConversationId and clears bg selection", async () => {
    selectedBackgroundSessionId.set("some-bg-id");
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
  });

  it("Resume click does NOT also select the row (stopPropagation)", async () => {
    const { getByTestId } = render(ConversationRow, {
      props: { conversation: BASE },
    });
    await tick();
    await fireEvent.click(getByTestId("ai-conversation-row-resume"));
    // Row click never fired → selection store remains null.
    expect(get(selectedConversationId)).toBeNull();
  });
});
