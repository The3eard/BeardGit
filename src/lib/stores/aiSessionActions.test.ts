/**
 * Tests for the shared AI session action helpers.
 *
 * Focus is on `focusSessionTab` — the Phase-10 composite-focus bug from the
 * spec lived inside its composite branch (it failed to rewrite
 * `activeSegmentIndex`). We also cover `getSessionTier`'s standalone and
 * composite match paths and the no-match fallthrough.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

// Keep tabs.ts happy under jsdom — it imports these at module load.
vi.mock("$lib/api/tauri", () => ({
  terminalSpawn: vi.fn().mockResolvedValue(100),
  terminalKill: vi.fn().mockResolvedValue(undefined),
  terminalSetActive: vi.fn().mockResolvedValue(undefined),
  aiLaunchInteractive: vi.fn().mockResolvedValue(200),
  aiResumeSession: vi.fn().mockResolvedValue(300),
}));

vi.mock("$lib/stores/terminal", () => ({
  onTerminalOutput: vi.fn(),
  offTerminalOutput: vi.fn(),
}));

import {
  openTabs,
  activeTabIndex,
} from "$lib/stores/tabs";
import {
  getSessionTier,
  focusSessionTab,
} from "$lib/stores/aiSessionActions";
import type { AiSession, Tab, ProjectInfo } from "$lib/types";

const PROJECT: ProjectInfo = {
  path: "/repos/demo",
  name: "demo",
  head_branch: "main",
  change_count: 0,
};

const CLAUDE_SESSION: AiSession = {
  id: "claude-session-1",
  provider: "claude_code",
  cwd: "/repos/demo",
  started_at: 1,
  kind: "interactive",
  is_active: true,
};

function resetTabs() {
  openTabs.set([]);
  activeTabIndex.set(-1);
}

describe("getSessionTier", () => {
  beforeEach(resetTabs);

  it("returns 'ended' for inactive sessions", () => {
    expect(
      getSessionTier({ ...CLAUDE_SESSION, is_active: false }),
    ).toBe("ended");
  });

  it("returns 'focus' when a standalone terminal tab matches", () => {
    openTabs.set([
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
    expect(getSessionTier(CLAUDE_SESSION)).toBe("focus");
  });

  it("returns 'focus' when a composite segment matches (trailing-slash tolerant)", () => {
    const tab: Tab = {
      kind: "composite",
      project: PROJECT,
      segments: [
        {
          type: "terminal",
          info: {
            sessionId: 2,
            title: "Claude",
            cwd: "/repos/demo/", // trailing slash
            provider: "claude_code",
          },
        },
      ],
      activeSegmentIndex: 0,
    };
    openTabs.set([tab]);
    expect(getSessionTier(CLAUDE_SESSION)).toBe("focus");
  });

  it("returns 'resume' when an active session has no tab", () => {
    openTabs.set([{ kind: "project", project: PROJECT }]);
    expect(getSessionTier(CLAUDE_SESSION)).toBe("resume");
  });
});

describe("focusSessionTab", () => {
  beforeEach(resetTabs);

  it("on standalone terminal match → only activeTabIndex updates", () => {
    const terminalTab: Tab = {
      kind: "terminal",
      terminal: {
        sessionId: 1,
        title: "Claude",
        cwd: "/repos/demo",
        provider: "claude_code",
      },
    };
    openTabs.set([{ kind: "project", project: PROJECT }, terminalTab]);
    activeTabIndex.set(0);
    const snapshotBefore = get(openTabs);

    const ok = focusSessionTab(CLAUDE_SESSION);
    expect(ok).toBe(true);
    expect(get(activeTabIndex)).toBe(1);
    // openTabs reference unchanged — no rewrite on standalone path.
    expect(get(openTabs)).toBe(snapshotBefore);
  });

  it("on composite match → both activeTabIndex and activeSegmentIndex update", () => {
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
      activeSegmentIndex: 0, // project/worktree active — must flip to 1
    };
    openTabs.set([composite]);
    activeTabIndex.set(-1);

    const ok = focusSessionTab(CLAUDE_SESSION);
    expect(ok).toBe(true);
    expect(get(activeTabIndex)).toBe(0);
    const compositeAfter = get(openTabs)[0] as Extract<
      Tab,
      { kind: "composite" }
    >;
    expect(compositeAfter.activeSegmentIndex).toBe(1);
  });

  it("with no match → returns false, stores untouched", () => {
    openTabs.set([{ kind: "project", project: PROJECT }]);
    activeTabIndex.set(0);
    const snapshotTabs = get(openTabs);
    const ok = focusSessionTab(CLAUDE_SESSION);
    expect(ok).toBe(false);
    expect(get(openTabs)).toBe(snapshotTabs);
    expect(get(activeTabIndex)).toBe(0);
  });

  it("ignores non-matching providers at the same cwd", () => {
    // A codex terminal lives in the same cwd — Claude session must not
    // accidentally focus it.
    openTabs.set([
      {
        kind: "terminal",
        terminal: {
          sessionId: 9,
          title: "Codex",
          cwd: "/repos/demo",
          provider: "codex",
        },
      },
    ]);
    activeTabIndex.set(-1);
    const ok = focusSessionTab(CLAUDE_SESSION);
    expect(ok).toBe(false);
    expect(get(activeTabIndex)).toBe(-1);
  });
});
