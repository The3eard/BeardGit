import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

vi.mock("$lib/api/tauri", () => ({
  terminalSpawn: vi.fn().mockResolvedValue(100),
  terminalKill: vi.fn().mockResolvedValue(undefined),
  terminalSetActive: vi.fn().mockResolvedValue(undefined),
  aiLaunchInteractive: vi.fn().mockResolvedValue(200),
}));

vi.mock("$lib/stores/terminal", () => ({
  onTerminalOutput: vi.fn(),
  offTerminalOutput: vi.fn(),
}));

import {
  openTabs,
  activeTabIndex,
  onTerminalProcessChanged,
} from "$lib/stores/tabs";
import type { Tab, TerminalTabInfo, ProjectInfo } from "$lib/types";

function resetTabs() {
  openTabs.set([]);
  activeTabIndex.set(-1);
}

const mockProject: ProjectInfo = {
  path: "/Users/adolfo/Projects/BeardGit",
  name: "BeardGit",
  head_branch: "main",
  change_count: 0,
};

describe("onTerminalProcessChanged", () => {
  beforeEach(() => {
    resetTabs();
  });

  it("sets provider on standalone terminal when claude detected", () => {
    const info: TerminalTabInfo = { sessionId: 1, title: "term", cwd: "/tmp" };
    openTabs.set([{ kind: "terminal", terminal: info }]);

    onTerminalProcessChanged(1, "claude");

    const tabs = get(openTabs);
    const tab = tabs[0] as Extract<Tab, { kind: "terminal" }>;
    expect(tab.terminal.provider).toBe("claude_code");
  });

  it("sets provider on standalone terminal when codex detected", () => {
    const info: TerminalTabInfo = { sessionId: 1, title: "term", cwd: "/tmp" };
    openTabs.set([{ kind: "terminal", terminal: info }]);

    onTerminalProcessChanged(1, "codex");

    const tabs = get(openTabs);
    const tab = tabs[0] as Extract<Tab, { kind: "terminal" }>;
    expect(tab.terminal.provider).toBe("codex");
  });

  it("sets provider on standalone terminal when opencode detected", () => {
    const info: TerminalTabInfo = { sessionId: 1, title: "term", cwd: "/tmp" };
    openTabs.set([{ kind: "terminal", terminal: info }]);

    onTerminalProcessChanged(1, "opencode");

    const tabs = get(openTabs);
    const tab = tabs[0] as Extract<Tab, { kind: "terminal" }>;
    expect(tab.terminal.provider).toBe("open_code");
  });

  it("clears provider when process reverts to shell", () => {
    const info: TerminalTabInfo = {
      sessionId: 1,
      title: "term",
      cwd: "/tmp",
      provider: "claude_code",
    };
    openTabs.set([{ kind: "terminal", terminal: info }]);

    onTerminalProcessChanged(1, "zsh");

    const tabs = get(openTabs);
    const tab = tabs[0] as Extract<Tab, { kind: "terminal" }>;
    expect(tab.terminal.provider).toBeUndefined();
  });

  it("clears provider when process is null", () => {
    const info: TerminalTabInfo = {
      sessionId: 1,
      title: "term",
      cwd: "/tmp",
      provider: "codex",
    };
    openTabs.set([{ kind: "terminal", terminal: info }]);

    onTerminalProcessChanged(1, null);

    const tabs = get(openTabs);
    const tab = tabs[0] as Extract<Tab, { kind: "terminal" }>;
    expect(tab.terminal.provider).toBeUndefined();
  });

  it("updates provider inside composite segment", () => {
    const info: TerminalTabInfo = {
      sessionId: 1,
      title: "term",
      cwd: mockProject.path,
    };
    openTabs.set([
      {
        kind: "composite",
        project: mockProject,
        segments: [{ type: "terminal", info }],
        activeSegmentIndex: 0,
      },
    ]);

    onTerminalProcessChanged(1, "opencode");

    const tabs = get(openTabs);
    const composite = tabs[0] as Extract<Tab, { kind: "composite" }>;
    const seg = composite.segments[0] as Extract<
      (typeof composite.segments)[0],
      { type: "terminal" }
    >;
    expect(seg.info.provider).toBe("open_code");
  });

  it("does nothing for unknown session IDs", () => {
    const info: TerminalTabInfo = { sessionId: 1, title: "term", cwd: "/tmp" };
    openTabs.set([{ kind: "terminal", terminal: info }]);

    onTerminalProcessChanged(999, "claude");

    const tabs = get(openTabs);
    const tab = tabs[0] as Extract<Tab, { kind: "terminal" }>;
    expect(tab.terminal.provider).toBeUndefined();
  });

  it("skips update when provider has not changed", () => {
    const info: TerminalTabInfo = {
      sessionId: 1,
      title: "term",
      cwd: "/tmp",
      provider: "claude_code",
    };
    openTabs.set([{ kind: "terminal", terminal: info }]);

    const tabsBefore = get(openTabs);
    onTerminalProcessChanged(1, "claude");
    const tabsAfter = get(openTabs);

    // Reference equality — store was not updated.
    expect(tabsBefore).toBe(tabsAfter);
  });
});
