/**
 * AiSessionDetail — active tab/segment branch.
 *
 * The list-trim refactor introduces a third detail branch: when a
 * tab/segment ActiveTerminal is selected, the pane shows provider name,
 * title, cwd, and a Focus button. Bg-kind ActiveTerminals keep flowing
 * through the existing bg branch — NOT this new branch — because the bg
 * branch already renders richer detail (transcript, cancel, etc).
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { TerminalTabInfo } from "$lib/types";
import type { ActiveTerminal } from "$lib/stores/aiActiveTerminals";

const { focusTerminal } = vi.hoisted(() => ({
  focusTerminal: vi.fn(() => true),
}));

vi.mock("$lib/stores/aiConversationActions", async () => {
  const actual = await vi.importActual<object>(
    "$lib/stores/aiConversationActions",
  );
  return { ...actual, focusTerminal };
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
import { selectedActiveTerminal } from "$lib/stores/aiActiveTerminals";

const TAB_INFO: TerminalTabInfo = {
  sessionId: 42,
  title: "Claude",
  cwd: "/repos/demo",
  provider: "claude_code",
};

const SEG_INFO: TerminalTabInfo = {
  sessionId: 43,
  title: "Codex",
  cwd: "/repos/demo/sub",
  provider: "codex",
};

function resetStores() {
  conversations.set([]);
  selectedConversationId.set(null);
  aiBackgroundRuns.set(new Map());
  selectedBackgroundSessionId.set(null);
  selectedActiveTerminal.set(null);
  focusTerminal.mockClear();
}

beforeEach(resetStores);
afterEach(() => {
  cleanup();
  resetStores();
});

describe("AiSessionDetail active branch — tab", () => {
  it("renders provider + title + cwd + Focus when a tab is selected", async () => {
    const active: ActiveTerminal = { kind: "tab", tabIndex: 2, info: TAB_INFO };
    selectedActiveTerminal.set(active);

    const { container, getByTestId } = render(AiSessionDetail);
    await tick();

    expect(getByTestId("ai-session-detail-active")).toBeTruthy();
    expect(container.textContent).toContain("Claude Code");
    expect(container.textContent).toContain("Terminal 3");
    expect(container.textContent).toContain("/repos/demo");
    expect(getByTestId("ai-session-detail-focus")).toBeTruthy();
  });

  it("Focus click routes through focusTerminal with the active payload", async () => {
    const active: ActiveTerminal = { kind: "tab", tabIndex: 0, info: TAB_INFO };
    selectedActiveTerminal.set(active);

    const { getByTestId } = render(AiSessionDetail);
    await tick();
    await fireEvent.click(getByTestId("ai-session-detail-focus"));

    expect(focusTerminal).toHaveBeenCalledTimes(1);
    expect(focusTerminal).toHaveBeenCalledWith(active);
  });
});

describe("AiSessionDetail active branch — segment", () => {
  it("renders 'Terminal in <basename>' title + Focus button", async () => {
    const active: ActiveTerminal = {
      kind: "segment",
      tabIndex: 0,
      segmentIndex: 1,
      info: SEG_INFO,
    };
    selectedActiveTerminal.set(active);

    const { container, getByTestId } = render(AiSessionDetail);
    await tick();

    expect(container.textContent).toContain("Terminal in sub");
    expect(container.textContent).toContain("/repos/demo/sub");
    expect(getByTestId("ai-session-detail-focus")).toBeTruthy();
  });
});

describe("AiSessionDetail active branch — bg is excluded", () => {
  it("bg-kind ActiveTerminal does NOT render the active branch (bg branch handles it via selectedBackgroundSessionId)", async () => {
    const active: ActiveTerminal = {
      kind: "bg",
      session: {
        id: "bg-x",
        provider: "claude_code",
        cwd: "/repos/demo",
        started_at: 1,
        kind: "headless",
        is_active: true,
        worktree_path: null,
        background_status: { state: "running" },
      },
    };
    selectedActiveTerminal.set(active);

    const { container } = render(AiSessionDetail);
    await tick();

    // Neither the new active branch nor the bg branch is selected
    // (bg-branch needs selectedBackgroundSessionId). Empty state wins.
    expect(
      container.querySelector('[data-testid="ai-session-detail-active"]'),
    ).toBeNull();
    expect(
      container.querySelector('[data-testid="ai-session-detail-empty"]'),
    ).toBeTruthy();
  });
});

describe("AiSessionDetail branch precedence", () => {
  it("conversation branch wins when both conversation and active are set (defensive tie-break)", async () => {
    const active: ActiveTerminal = { kind: "tab", tabIndex: 0, info: TAB_INFO };
    selectedActiveTerminal.set(active);
    conversations.set([
      {
        id: "c1",
        provider: "claude_code",
        cwd: "/repos/demo",
        created_at: 1,
        last_activity_at: 1,
        title: "Hello",
      },
    ]);
    selectedConversationId.set("c1");

    const { container } = render(AiSessionDetail);
    await tick();
    expect(
      container.querySelector('[data-testid="ai-session-detail-conversation"]'),
    ).toBeTruthy();
    expect(
      container.querySelector('[data-testid="ai-session-detail-active"]'),
    ).toBeNull();
  });
});
