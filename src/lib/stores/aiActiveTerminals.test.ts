/**
 * Tests for the `activeAiTerminals` derive.
 *
 * Pure store logic — no Tauri mocks needed. We stub out the tauri / terminal
 * modules that `tabs.ts` pulls in at module load so the test runner doesn't
 * choke, then drive `openTabs` + `aiBackgroundRuns` directly and assert the
 * shape of the derive.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

// Keep tabs.ts happy under jsdom — it imports these at module load.
vi.mock("$lib/api/tauri", () => ({
  terminalSpawn: vi.fn().mockResolvedValue(100),
  terminalKill: vi.fn().mockResolvedValue(undefined),
  aiLaunchInteractive: vi.fn().mockResolvedValue(200),
  aiResumeConversation: vi.fn().mockResolvedValue(400),
  aiListBackgroundRuns: vi.fn().mockResolvedValue([]),
  aiStartBackgroundRun: vi.fn(),
  aiCancelBackgroundRun: vi.fn(),
  aiDiscardBackgroundRunWorktree: vi.fn(),
  aiOpenBackgroundTerminal: vi.fn(),
}));

vi.mock("$lib/stores/terminal", () => ({
  onTerminalOutput: vi.fn(),
  offTerminalOutput: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

import { openTabs } from "./tabs";
import { aiBackgroundRuns } from "./aiBackground";
import {
  activeAiTerminals,
  countActiveAiTerminals,
} from "./aiActiveTerminals";
import type { AiSession, ProjectInfo, Tab } from "$lib/types";

const PROJECT: ProjectInfo = {
  path: "/repos/demo",
  name: "demo",
  head_branch: "main",
  change_count: 0,
};

function resetStores() {
  openTabs.set([]);
  aiBackgroundRuns.set(new Map());
}

describe("activeAiTerminals", () => {
  beforeEach(resetStores);

  it("emits an empty list when there are no tabs or bg-runs", () => {
    expect(get(activeAiTerminals)).toEqual([]);
  });

  it("includes a standalone terminal tab when provider is set", () => {
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
    const list = get(activeAiTerminals);
    expect(list).toHaveLength(1);
    expect(list[0]).toMatchObject({
      kind: "tab",
      tabIndex: 0,
      info: { sessionId: 1, provider: "claude_code" },
    });
  });

  it("excludes standalone terminal tabs with no provider (non-AI terminals)", () => {
    openTabs.set([
      {
        kind: "terminal",
        terminal: { sessionId: 1, title: "zsh", cwd: "/repos/demo" },
      },
    ]);
    expect(get(activeAiTerminals)).toEqual([]);
  });

  it("includes composite segments whose terminal has a provider", () => {
    const tab: Tab = {
      kind: "composite",
      project: PROJECT,
      segments: [
        { type: "worktree", path: "/repos/demo/.wt/feature", branch: "feature" },
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
      activeSegmentIndex: 0,
    };
    openTabs.set([tab]);
    const list = get(activeAiTerminals);
    expect(list).toHaveLength(1);
    expect(list[0]).toMatchObject({
      kind: "segment",
      tabIndex: 0,
      segmentIndex: 1,
      info: { sessionId: 2, provider: "claude_code" },
    });
  });

  it("excludes composite segments without a provider and non-terminal segments", () => {
    const tab: Tab = {
      kind: "composite",
      project: PROJECT,
      segments: [
        { type: "worktree", path: "/repos/demo/.wt/feature", branch: "feature" },
        {
          type: "terminal",
          info: { sessionId: 3, title: "zsh", cwd: "/repos/demo" },
        },
      ],
      activeSegmentIndex: 0,
    };
    openTabs.set([tab]);
    expect(get(activeAiTerminals)).toEqual([]);
  });

  it("excludes project tabs outright", () => {
    openTabs.set([{ kind: "project", project: PROJECT }]);
    expect(get(activeAiTerminals)).toEqual([]);
  });

  it("emits tabs first, then bg-runs (running state included)", () => {
    const standalone: Tab = {
      kind: "terminal",
      terminal: {
        sessionId: 1,
        title: "Claude",
        cwd: "/repos/demo",
        provider: "claude_code",
      },
    };
    const composite: Tab = {
      kind: "composite",
      project: PROJECT,
      segments: [
        {
          type: "terminal",
          info: {
            sessionId: 2,
            title: "Codex",
            cwd: "/repos/demo",
            provider: "codex",
          },
        },
      ],
      activeSegmentIndex: 0,
    };
    openTabs.set([standalone, composite]);

    const bg: AiSession = {
      id: "bg-1",
      provider: "claude_code",
      cwd: "/repos/demo",
      started_at: 1,
      kind: "headless",
      is_active: true,
      worktree_path: "/repos/demo/.wt/bg",
      background_status: { state: "running" },
    };
    aiBackgroundRuns.set(new Map([[bg.id, bg]]));

    const list = get(activeAiTerminals);
    expect(list).toHaveLength(3);
    expect(list[0].kind).toBe("tab");
    expect(list[1].kind).toBe("segment");
    expect(list[2].kind).toBe("bg");
    if (list[2].kind === "bg") expect(list[2].session.id).toBe("bg-1");
  });

  it("includes a queued bg-run", () => {
    const queued: AiSession = {
      id: "bg-q",
      provider: "claude_code",
      cwd: "/repos/demo",
      started_at: 1,
      kind: "headless",
      is_active: true,
      background_status: { state: "queued" },
    };
    aiBackgroundRuns.set(new Map([[queued.id, queued]]));
    const list = get(activeAiTerminals);
    expect(list).toHaveLength(1);
    expect(list[0].kind).toBe("bg");
  });

  it("excludes a completed bg-run", () => {
    const done: AiSession = {
      id: "bg-done",
      provider: "claude_code",
      cwd: "/repos/demo",
      started_at: 1,
      kind: "headless",
      is_active: false,
      background_status: { state: "completed", exit_code: 0 },
    };
    aiBackgroundRuns.set(new Map([[done.id, done]]));
    expect(get(activeAiTerminals)).toEqual([]);
  });

  it("excludes failed and cancelled bg-runs", () => {
    const failed: AiSession = {
      id: "bg-fail",
      provider: "claude_code",
      cwd: "/repos/demo",
      started_at: 1,
      kind: "headless",
      is_active: false,
      background_status: { state: "failed", message: "boom" },
    };
    const cancelled: AiSession = {
      id: "bg-cancel",
      provider: "claude_code",
      cwd: "/repos/demo",
      started_at: 1,
      kind: "headless",
      is_active: false,
      background_status: { state: "cancelled" },
    };
    aiBackgroundRuns.set(
      new Map([
        [failed.id, failed],
        [cancelled.id, cancelled],
      ]),
    );
    expect(get(activeAiTerminals)).toEqual([]);
  });

  it("does NOT dedupe across sources — tab + matching bg-run coexist", () => {
    // Regression guard: the spec explicitly says no cross-source dedupe.
    openTabs.set([
      {
        kind: "terminal",
        terminal: {
          sessionId: 42,
          title: "Claude",
          cwd: "/repos/demo",
          provider: "claude_code",
        },
      },
    ]);
    aiBackgroundRuns.set(
      new Map([
        [
          "bg-dup",
          {
            id: "bg-dup",
            provider: "claude_code",
            cwd: "/repos/demo",
            started_at: 1,
            kind: "headless",
            is_active: true,
            background_status: { state: "running" },
          } satisfies AiSession,
        ],
      ]),
    );
    expect(get(activeAiTerminals)).toHaveLength(2);
  });
});

describe("countActiveAiTerminals", () => {
  it("returns the length of the list", () => {
    expect(countActiveAiTerminals([])).toBe(0);
    expect(
      countActiveAiTerminals([
        {
          kind: "tab",
          tabIndex: 0,
          info: {
            sessionId: 1,
            title: "x",
            cwd: "/",
            provider: "claude_code",
          },
        },
        {
          kind: "tab",
          tabIndex: 1,
          info: {
            sessionId: 2,
            title: "y",
            cwd: "/",
            provider: "claude_code",
          },
        },
      ]),
    ).toBe(2);
  });
});
