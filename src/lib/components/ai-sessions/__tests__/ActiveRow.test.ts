/**
 * ActiveRow — three-branch (tab / segment / bg) render + focus wiring.
 *
 * Uses `vi.hoisted` to mock `focusTerminal` before the component module
 * loads, then asserts each discriminator renders the expected copy and
 * that the Focus button routes through the mock.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { AiSession, TerminalTabInfo } from "$lib/types";
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

import ActiveRow from "../ActiveRow.svelte";

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

const BG_SESSION: AiSession = {
  id: "bg-1",
  provider: "open_code",
  cwd: "/repos/demo",
  started_at: 1_700_000_000,
  kind: "headless",
  is_active: true,
  worktree_path: "/repos/demo/.wt/ai-bg",
  background_status: { state: "running" },
};

beforeEach(() => {
  focusTerminal.mockClear();
});

afterEach(() => cleanup());

describe("ActiveRow", () => {
  it("renders the tab branch with provider name and cwd", async () => {
    const active: ActiveTerminal = { kind: "tab", tabIndex: 2, info: TAB_INFO };
    const { container } = render(ActiveRow, { props: { active } });
    await tick();

    const row = container.querySelector('[data-testid="ai-active-row"]');
    expect(row).toBeTruthy();
    expect(row?.getAttribute("data-kind")).toBe("tab");
    expect(container.textContent).toContain("Claude Code");
    expect(container.textContent).toContain("demo");
  });

  it("renders the segment branch with a cwd-aware title", async () => {
    const active: ActiveTerminal = {
      kind: "segment",
      tabIndex: 0,
      segmentIndex: 1,
      info: SEG_INFO,
    };
    const { container } = render(ActiveRow, { props: { active } });
    await tick();

    const row = container.querySelector('[data-testid="ai-active-row"]');
    expect(row?.getAttribute("data-kind")).toBe("segment");
    expect(container.textContent).toContain("Codex");
    // segment title references the cwd basename
    expect(container.textContent).toContain("sub");
  });

  it("renders the bg branch with a status badge", async () => {
    const active: ActiveTerminal = { kind: "bg", session: BG_SESSION };
    const { container } = render(ActiveRow, { props: { active } });
    await tick();

    const row = container.querySelector('[data-testid="ai-active-row"]');
    expect(row?.getAttribute("data-kind")).toBe("bg");
    expect(container.textContent?.toLowerCase()).toContain("opencode");
    // BackgroundRunStatusBadge renders a .badge element
    expect(container.querySelector(".badge")).toBeTruthy();
  });

  it("Focus button routes through focusTerminal with the active payload", async () => {
    const active: ActiveTerminal = { kind: "tab", tabIndex: 0, info: TAB_INFO };
    const { getByTestId } = render(ActiveRow, { props: { active } });
    await tick();

    await fireEvent.click(getByTestId("ai-active-row-focus"));
    expect(focusTerminal).toHaveBeenCalledTimes(1);
    expect(focusTerminal).toHaveBeenCalledWith(active);
  });
});
