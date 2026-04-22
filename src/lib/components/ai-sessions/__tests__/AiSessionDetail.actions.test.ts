/**
 * Detail pane — provider-reported branch.
 *
 * These are the acceptance tests for the Phase-10 fixes:
 *   - Clicking a row with no `background_status` populates the detail pane
 *     (the old code rendered an empty placeholder — bug 1/5 from the spec).
 *   - The action bar shows Focus/Open-Terminal/Dismiss with the correct
 *     tier logic and routes clicks through the shared `aiSessionActions`
 *     helpers.
 *
 * The helpers are mocked so we assert wiring without spinning up a real
 * PTY; `dismissSession` is mocked for the same reason.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { AiSession, Tab } from "$lib/types";

const { focusSessionTab, resumeSession } = vi.hoisted(() => ({
  focusSessionTab: vi.fn(() => true),
  resumeSession: vi.fn(async () => true),
}));

const { dismissSession } = vi.hoisted(() => ({
  dismissSession: vi.fn(),
}));

vi.mock("$lib/stores/aiSessionActions", async () => {
  const actual = await vi.importActual<object>(
    "$lib/stores/aiSessionActions",
  );
  return { ...actual, focusSessionTab, resumeSession };
});

vi.mock("$lib/stores/aiSessions", async () => {
  const actual = await vi.importActual<object>("$lib/stores/aiSessions");
  return { ...actual, dismissSession };
});

import {
  aiBackgroundRuns,
  selectedBackgroundSessionId,
} from "$lib/stores/aiBackground";
import { openTabs, activeTabIndex } from "$lib/stores/tabs";
import { sessions } from "$lib/stores/aiSessions";
import AiSessionDetail from "../AiSessionDetail.svelte";

const EXTERNAL_ACTIVE: AiSession = {
  id: "ext-1",
  provider: "claude_code",
  cwd: "/repos/demo",
  started_at: 1_700_000_000,
  kind: "interactive",
  is_active: true,
  // no worktree_path → EXTERNAL badge
  // no background_status → provider-reported branch
};

beforeEach(() => {
  aiBackgroundRuns.set(new Map());
  selectedBackgroundSessionId.set(null);
  sessions.set([]);
  openTabs.set([]);
  activeTabIndex.set(-1);
  focusSessionTab.mockClear();
  resumeSession.mockClear();
  dismissSession.mockClear();
});

afterEach(() => {
  cleanup();
});

describe("AiSessionDetail provider-reported branch", () => {
  it("renders header with ACTIVE + kind + EXTERNAL badges", async () => {
    sessions.set([EXTERNAL_ACTIVE]);
    selectedBackgroundSessionId.set(EXTERNAL_ACTIVE.id);
    const { container, getByTestId } = render(AiSessionDetail);
    await tick();

    const detail = getByTestId("ai-session-detail");
    expect(detail.textContent).toContain("Claude Code");
    expect(detail.textContent?.toUpperCase()).toContain("ACTIVE");
    expect(detail.textContent?.toLowerCase()).toContain("interactive");
    expect(
      container.querySelector('[data-testid="external-badge"]'),
    ).toBeTruthy();
    expect(container.textContent).toContain("/repos/demo");
  });

  it("shows Open Terminal when tier === 'resume' and calls resumeSession on click", async () => {
    sessions.set([EXTERNAL_ACTIVE]);
    selectedBackgroundSessionId.set(EXTERNAL_ACTIVE.id);
    const { getByTestId, queryByTestId } = render(AiSessionDetail);
    await tick();

    expect(queryByTestId("ai-session-detail-focus")).toBeNull();
    const openBtn = getByTestId("ai-session-detail-open-terminal");
    await fireEvent.click(openBtn);
    expect(resumeSession).toHaveBeenCalledTimes(1);
    expect(resumeSession).toHaveBeenCalledWith(EXTERNAL_ACTIVE);
  });

  it("shows Focus when a matching tab exists and calls focusSessionTab", async () => {
    sessions.set([EXTERNAL_ACTIVE]);
    selectedBackgroundSessionId.set(EXTERNAL_ACTIVE.id);
    const terminalTab: Tab = {
      kind: "terminal",
      terminal: {
        sessionId: 42,
        title: "Claude",
        cwd: "/repos/demo",
        provider: "claude_code",
      },
    };
    openTabs.set([terminalTab]);
    const { getByTestId, queryByTestId } = render(AiSessionDetail);
    await tick();

    expect(queryByTestId("ai-session-detail-open-terminal")).toBeNull();
    const focusBtn = getByTestId("ai-session-detail-focus");
    await fireEvent.click(focusBtn);
    expect(focusSessionTab).toHaveBeenCalledTimes(1);
    expect(focusSessionTab).toHaveBeenCalledWith(EXTERNAL_ACTIVE);
  });

  it("Dismiss calls dismissSession with the session id", async () => {
    sessions.set([EXTERNAL_ACTIVE]);
    selectedBackgroundSessionId.set(EXTERNAL_ACTIVE.id);
    const { getByTestId } = render(AiSessionDetail);
    await tick();

    const dismissBtn = getByTestId("ai-session-detail-dismiss");
    await fireEvent.click(dismissBtn);
    expect(dismissSession).toHaveBeenCalledWith("ext-1");
  });

  it("ENDED sessions show only Dismiss (no Focus / Open Terminal)", async () => {
    const ended: AiSession = { ...EXTERNAL_ACTIVE, is_active: false };
    sessions.set([ended]);
    selectedBackgroundSessionId.set(ended.id);
    const { queryByTestId, getByTestId } = render(AiSessionDetail);
    await tick();

    expect(queryByTestId("ai-session-detail-focus")).toBeNull();
    expect(queryByTestId("ai-session-detail-open-terminal")).toBeNull();
    expect(getByTestId("ai-session-detail-dismiss")).toBeTruthy();
  });
});

