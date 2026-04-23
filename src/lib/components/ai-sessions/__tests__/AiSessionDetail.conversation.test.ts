/**
 * AiSessionDetail — conversation branch.
 *
 * Exercises the Phase-5 detail-pane contract when a conversation is
 * selected: header renders the title/cwd, a fork-row appears when the
 * conversation has a parent_id, and the Resume button calls
 * `resumeConversation` with the selected row.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
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

import AiSessionDetail from "../AiSessionDetail.svelte";
import {
  conversations,
  selectedConversationId,
} from "$lib/stores/aiConversations";
import {
  aiBackgroundRuns,
  selectedBackgroundSessionId,
} from "$lib/stores/aiBackground";

const CONVERSATION: AiConversation = {
  id: "conv-abc",
  provider: "claude_code",
  cwd: "/repos/demo",
  created_at: 1_700_000_000_000,
  last_activity_at: 1_700_000_000_000,
  title: "Investigate the bug in the login flow",
};

function resetStores() {
  conversations.set([]);
  selectedConversationId.set(null);
  aiBackgroundRuns.set(new Map());
  selectedBackgroundSessionId.set(null);
  resumeConversation.mockClear();
}

beforeEach(resetStores);
afterEach(() => {
  cleanup();
  resetStores();
});

describe("AiSessionDetail conversation branch", () => {
  it("renders title + cwd + Resume button when a conversation is selected", async () => {
    conversations.set([CONVERSATION]);
    selectedConversationId.set(CONVERSATION.id);

    const { container, getByTestId } = render(AiSessionDetail);
    await tick();

    expect(getByTestId("ai-session-detail-conversation")).toBeTruthy();
    expect(container.textContent).toContain("Claude Code");
    expect(container.textContent).toContain(
      "Investigate the bug in the login flow",
    );
    expect(container.textContent).toContain("/repos/demo");
    expect(getByTestId("ai-session-detail-resume")).toBeTruthy();
  });

  it("renders the Forked-from row when parent_id is set", async () => {
    conversations.set([{ ...CONVERSATION, parent_id: "cafed00d" }]);
    selectedConversationId.set(CONVERSATION.id);

    const { container, getByTestId } = render(AiSessionDetail);
    await tick();

    expect(getByTestId("ai-session-detail-forked")).toBeTruthy();
    expect(container.textContent).toContain("cafed00d");
  });

  it("Resume click calls resumeConversation with the selected conversation", async () => {
    conversations.set([CONVERSATION]);
    selectedConversationId.set(CONVERSATION.id);

    const { getByTestId } = render(AiSessionDetail);
    await tick();
    await fireEvent.click(getByTestId("ai-session-detail-resume"));

    expect(resumeConversation).toHaveBeenCalledTimes(1);
    expect(resumeConversation).toHaveBeenCalledWith(CONVERSATION);
  });

  it("falls back to (no title) when the transcript had no suitable first message", async () => {
    conversations.set([{ ...CONVERSATION, title: "" }]);
    selectedConversationId.set(CONVERSATION.id);

    const { getByTestId } = render(AiSessionDetail);
    await tick();
    expect(getByTestId("ai-session-detail-title").textContent).toContain(
      "(no title)",
    );
  });
});
