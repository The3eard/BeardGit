/**
 * Tests for the shared AI conversation action helpers.
 *
 * Covers `focusTerminal` across all three `ActiveTerminal` variants (tab,
 * composite segment, background run) and `resumeConversation`'s pass-through
 * to `resumeAiConversationTab`.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

// Keep tabs.ts happy under jsdom — it imports these at module load.
vi.mock("$lib/api/tauri", () => ({
  terminalSpawn: vi.fn().mockResolvedValue(100),
  terminalKill: vi.fn().mockResolvedValue(undefined),
  aiLaunchInteractive: vi.fn().mockResolvedValue(200),
  aiResumeConversation: vi.fn().mockResolvedValue(400),
}));

vi.mock("$lib/stores/terminal", () => ({
  onTerminalOutput: vi.fn(),
  offTerminalOutput: vi.fn(),
}));

// Override `resumeAiConversationTab` so we can assert call args / flip the
// return value — the underlying tauri mock would always resolve to 400.
const resumeAiConversationTabMock = vi.fn();
vi.mock("./tabs", async () => {
  const actual = await vi.importActual<typeof import("./tabs")>("./tabs");
  return {
    ...actual,
    resumeAiConversationTab: (
      ...args: Parameters<typeof actual.resumeAiConversationTab>
    ) => resumeAiConversationTabMock(...args),
  };
});

import { openTabs, activeTabIndex } from "./tabs";
import { selectedBackgroundSessionId } from "./aiBackground";
import { selectedConversationId } from "./aiConversations";
import {
  focusTerminal,
  resumeConversation,
} from "./aiConversationActions";
import type {
  ActiveTerminal,
} from "./aiActiveTerminals";
import type {
  AiConversation,
  AiSession,
  ProjectInfo,
  Tab,
} from "$lib/types";

const PROJECT: ProjectInfo = {
  path: "/repos/demo",
  name: "demo",
  head_branch: "main",
  change_count: 0,
};

function resetStores() {
  openTabs.set([]);
  activeTabIndex.set(-1);
  selectedBackgroundSessionId.set(null);
  selectedConversationId.set(null);
  resumeAiConversationTabMock.mockReset();
}

describe("focusTerminal", () => {
  beforeEach(resetStores);

  it("kind='tab' sets activeTabIndex and returns true", () => {
    openTabs.set([
      { kind: "project", project: PROJECT },
      { kind: "project", project: PROJECT },
      {
        kind: "terminal",
        terminal: {
          sessionId: 1,
          title: "Claude",
          cwd: "/repos/demo",
          provider: "claude_code",
        },
      },
    ]);
    const active: ActiveTerminal = {
      kind: "tab",
      tabIndex: 2,
      info: {
        sessionId: 1,
        title: "Claude",
        cwd: "/repos/demo",
        provider: "claude_code",
      },
    };
    expect(focusTerminal(active)).toBe(true);
    expect(get(activeTabIndex)).toBe(2);
  });

  it("kind='segment' updates both activeTabIndex and the composite's activeSegmentIndex", () => {
    const composite: Tab = {
      kind: "composite",
      project: PROJECT,
      segments: [
        {
          type: "worktree",
          path: "/repos/demo/.wt/feature",
          branch: "feature",
        },
        {
          type: "terminal",
          info: {
            sessionId: 2,
            title: "Claude",
            cwd: "/repos/demo",
            provider: "claude_code",
          },
        },
      ],
      activeSegmentIndex: 0, // worktree active — must flip to 1
    };
    openTabs.set([composite]);

    const active: ActiveTerminal = {
      kind: "segment",
      tabIndex: 0,
      segmentIndex: 1,
      info: composite.segments[1].type === "terminal"
        ? composite.segments[1].info
        : (() => {
            throw new Error("fixture");
          })(),
    };
    expect(focusTerminal(active)).toBe(true);
    expect(get(activeTabIndex)).toBe(0);
    const after = get(openTabs)[0] as Extract<Tab, { kind: "composite" }>;
    expect(after.activeSegmentIndex).toBe(1);
  });

  it("kind='segment' with a stale tabIndex still sets activeTabIndex without crashing", () => {
    // Simulates the race where the tab was closed between the list
    // snapshot and the focus call.
    openTabs.set([{ kind: "project", project: PROJECT }]);
    const active: ActiveTerminal = {
      kind: "segment",
      tabIndex: 0,
      segmentIndex: 1,
      info: {
        sessionId: 42,
        title: "ghost",
        cwd: "/repos/demo",
        provider: "claude_code",
      },
    };
    expect(focusTerminal(active)).toBe(true);
    expect(get(activeTabIndex)).toBe(0);
  });

  it("kind='bg' sets selectedBackgroundSessionId, clears selectedConversationId, returns true", () => {
    selectedConversationId.set("some-conv");
    const session: AiSession = {
      id: "bg-42",
      provider: "claude_code",
      cwd: "/repos/demo",
      started_at: 1,
      kind: "headless",
      is_active: true,
      background_status: { state: "running" },
    };
    const active: ActiveTerminal = { kind: "bg", session };
    expect(focusTerminal(active)).toBe(true);
    expect(get(selectedBackgroundSessionId)).toBe("bg-42");
    expect(get(selectedConversationId)).toBeNull();
  });
});

describe("resumeConversation", () => {
  beforeEach(resetStores);

  const conv: AiConversation = {
    id: "conv-uuid-1",
    provider: "claude_code",
    cwd: "/repos/demo",
    created_at: 10,
    last_activity_at: 100,
    title: "investigate a bug",
  };

  it("returns true when the wrapper succeeds and passes (cwd, providerName, provider, id)", async () => {
    resumeAiConversationTabMock.mockResolvedValueOnce(true);
    const ok = await resumeConversation(conv);
    expect(ok).toBe(true);
    expect(resumeAiConversationTabMock).toHaveBeenCalledTimes(1);
    const [cwd, title, provider, id] =
      resumeAiConversationTabMock.mock.calls[0];
    expect(cwd).toBe("/repos/demo");
    // providerName("claude_code") → "Claude Code"
    expect(typeof title).toBe("string");
    expect(title.length).toBeGreaterThan(0);
    expect(provider).toBe("claude_code");
    expect(id).toBe("conv-uuid-1");
  });

  it("returns false when the provider has no resume command", async () => {
    resumeAiConversationTabMock.mockResolvedValueOnce(false);
    expect(await resumeConversation(conv)).toBe(false);
  });
});
