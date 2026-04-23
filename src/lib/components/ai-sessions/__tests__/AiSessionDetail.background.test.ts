/**
 * AiSessionDetail — background branch Focus wiring.
 *
 * The list-trim refactor moves the row-level Focus button onto the
 * detail pane. This test covers the bg branch: selecting a bg run and
 * clicking Focus must call `focusTerminal` with a `{ kind: "bg" }` payload.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { AiSession } from "$lib/types";

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

import AiSessionDetail from "../AiSessionDetail.svelte";
import {
  aiBackgroundRuns,
  selectedBackgroundSessionId,
} from "$lib/stores/aiBackground";
import {
  conversations,
  selectedConversationId,
} from "$lib/stores/aiConversations";
import { selectedActiveTerminal } from "$lib/stores/aiActiveTerminals";

const BG: AiSession = {
  id: "bg-1",
  provider: "claude_code",
  cwd: "/repos/demo",
  started_at: Math.floor(Date.now() / 1000) - 60,
  kind: "headless",
  is_active: true,
  worktree_path: "/repos/demo/.wt/ai-bg",
  background_status: { state: "running" },
};

function resetStores() {
  conversations.set([]);
  selectedConversationId.set(null);
  aiBackgroundRuns.set(new Map());
  selectedBackgroundSessionId.set(null);
  selectedActiveTerminal.set(null);
  focusTerminal.mockClear();
  resumeConversation.mockClear();
}

beforeEach(resetStores);
afterEach(() => {
  cleanup();
  resetStores();
});

describe("AiSessionDetail bg branch — Focus", () => {
  it("renders a Focus button when a bg run is selected", async () => {
    aiBackgroundRuns.set(new Map([[BG.id, BG]]));
    selectedBackgroundSessionId.set(BG.id);

    const { getByTestId } = render(AiSessionDetail);
    await tick();
    expect(getByTestId("ai-session-detail-focus")).toBeTruthy();
  });

  it("clicking Focus calls focusTerminal with the bg active payload", async () => {
    aiBackgroundRuns.set(new Map([[BG.id, BG]]));
    selectedBackgroundSessionId.set(BG.id);

    const { getByTestId } = render(AiSessionDetail);
    await tick();
    await fireEvent.click(getByTestId("ai-session-detail-focus"));

    expect(focusTerminal).toHaveBeenCalledTimes(1);
    const call = focusTerminal.mock.calls[0] as unknown as [
      { kind: "bg"; session: AiSession },
    ];
    expect(call?.[0]?.kind).toBe("bg");
    expect(call?.[0]?.session.id).toBe(BG.id);
  });
});
