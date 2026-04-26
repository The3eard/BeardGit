/**
 * ActiveRow — wraps `SessionRow` with derivations for the three active
 * discriminators (tab / segment / bg).
 *
 * After the list-trim refactor the row renders ONLY:
 *   [provider icon] [title] [relative date-or-em-dash]
 *
 * Title copy:
 *   - tab     → "Terminal N+1"
 *   - segment → "Terminal in <basename(cwd)>"
 *   - bg      → provider display name (e.g. "Claude Code")
 *
 * Date:
 *   - bg      → `formatRelativeTimeUnix(started_at)`
 *   - others  → null (renders "—" in SessionRow)
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";
import { tick } from "svelte";
import type { AiSession, TerminalTabInfo } from "$lib/types";
import type { ActiveTerminal } from "$lib/stores/aiActiveTerminals";

import ActiveRow from "../ActiveRow.svelte";
import {
  selectedActiveTerminal,
} from "$lib/stores/aiActiveTerminals";
import { selectedConversationId } from "$lib/stores/aiConversations";
import { selectedBackgroundSessionId } from "$lib/stores/aiBackground";

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
  started_at: Math.floor(Date.now() / 1000) - 120, // 2 minutes ago
  kind: "headless",
  is_active: true,
  worktree_path: "/repos/demo/.wt/ai-bg",
  background_status: { state: "running" },
};

beforeEach(() => {
  selectedConversationId.set(null);
  selectedBackgroundSessionId.set(null);
  selectedActiveTerminal.set(null);
});
afterEach(() => cleanup());

describe("ActiveRow (trimmed)", () => {
  it("tab branch: title is 'Terminal N+1', date renders em-dash", async () => {
    const active: ActiveTerminal = { kind: "tab", tabIndex: 2, info: TAB_INFO };
    const { container } = render(ActiveRow, { props: { active } });
    await tick();

    expect(container.textContent).toContain("Terminal 3");
    expect(container.textContent).toContain("—");
    // Legacy artefacts are gone.
    expect(
      container.querySelector('[data-testid="ai-active-row-focus"]'),
    ).toBeNull();
    expect(container.querySelector(".badge")).toBeNull();
  });

  it("segment branch: title references the cwd basename", async () => {
    const active: ActiveTerminal = {
      kind: "segment",
      tabIndex: 0,
      segmentIndex: 1,
      info: SEG_INFO,
    };
    const { container } = render(ActiveRow, { props: { active } });
    await tick();

    expect(container.textContent).toContain("Terminal in sub");
    expect(container.textContent).toContain("—");
  });

  it("bg branch: title is provider name, date is a relative time", async () => {
    const active: ActiveTerminal = { kind: "bg", session: BG_SESSION };
    const { container } = render(ActiveRow, { props: { active } });
    await tick();

    expect(container.textContent?.toLowerCase()).toContain("opencode");
    expect(container.textContent?.toLowerCase()).toMatch(/ago|just now/);
    // Status badge is gone from the row.
    expect(container.querySelector(".badge")).toBeNull();
  });

  it("row click writes selectedActiveTerminal and clears the other two", async () => {
    selectedConversationId.set("some-conv");
    selectedBackgroundSessionId.set("some-bg");

    const active: ActiveTerminal = { kind: "tab", tabIndex: 0, info: TAB_INFO };
    const { container } = render(ActiveRow, { props: { active } });
    await tick();

    const row = container.querySelector('[data-testid="ai-active-row"]') as HTMLElement;
    await fireEvent.click(row);

    expect(get(selectedActiveTerminal)).toEqual(active);
    expect(get(selectedConversationId)).toBeNull();
    expect(get(selectedBackgroundSessionId)).toBeNull();
  });

  it("bg row selection routes to selectedBackgroundSessionId, clearing the others", async () => {
    // Bg rows render via the bg-run detail branch (status badge, transcript,
    // focus/cancel/discard). The detail pane intentionally drops bg
    // selections from the active branch, so the click must store the bg
    // session id on `selectedBackgroundSessionId` — not on
    // `selectedActiveTerminal` — for the detail to render at all.
    selectedConversationId.set("some-conv");
    selectedActiveTerminal.set({ kind: "tab", tabIndex: 0, info: TAB_INFO });

    const active: ActiveTerminal = { kind: "bg", session: BG_SESSION };
    const { container } = render(ActiveRow, { props: { active } });
    await tick();
    await fireEvent.click(
      container.querySelector('[data-testid="ai-active-row"]') as HTMLElement,
    );

    expect(get(selectedBackgroundSessionId)).toBe(BG_SESSION.id);
    expect(get(selectedActiveTerminal)).toBeNull();
    expect(get(selectedConversationId)).toBeNull();
  });

  it("bg row marks selected when selectedBackgroundSessionId matches", async () => {
    const active: ActiveTerminal = { kind: "bg", session: BG_SESSION };
    selectedBackgroundSessionId.set(BG_SESSION.id);
    const { container } = render(ActiveRow, { props: { active } });
    await tick();
    expect(container.querySelector(".session-row.selected")).toBeTruthy();
  });

  it("selected class reflects store state for tab kind", async () => {
    const active: ActiveTerminal = { kind: "tab", tabIndex: 2, info: TAB_INFO };
    selectedActiveTerminal.set(active);
    const { container } = render(ActiveRow, { props: { active } });
    await tick();
    const row = container.querySelector(".session-row") as HTMLElement;
    expect(row.classList.contains("selected")).toBe(true);
  });

  it("selected class reflects store state for segment kind (matches on both indices)", async () => {
    const active: ActiveTerminal = {
      kind: "segment",
      tabIndex: 0,
      segmentIndex: 1,
      info: SEG_INFO,
    };
    selectedActiveTerminal.set(active);
    const { container } = render(ActiveRow, { props: { active } });
    await tick();
    expect(container.querySelector(".session-row.selected")).toBeTruthy();
  });
});
