import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

// Mock tauri.ts to avoid actual IPC calls.
vi.mock("$lib/api/tauri", () => ({
  terminalSpawn: vi.fn().mockResolvedValue(100),
  terminalKill: vi.fn().mockResolvedValue(undefined),
  terminalSetActive: vi.fn().mockResolvedValue(undefined),
  aiLaunchInteractive: vi.fn().mockResolvedValue(200),
}));

// Mock terminal.ts to avoid registering Tauri event listeners during import.
vi.mock("$lib/stores/terminal", () => ({
  onTerminalOutput: vi.fn(),
  offTerminalOutput: vi.fn(),
}));

import {
  openTabs,
  activeTabIndex,
  addProjectTab,
  onTerminalCwdChanged,
} from "$lib/stores/tabs";
import type { Tab, ProjectInfo, TerminalTabInfo } from "$lib/types";

function resetTabs() {
  openTabs.set([]);
  activeTabIndex.set(-1);
}

const mockProject: ProjectInfo = {
  path: "/Users/adolfo/Projects/BeardGit",
  name: "BeardGit",
  head_branch: "main",
  change_count: 0,
  is_worktree: false,
};

describe("onTerminalCwdChanged", () => {
  beforeEach(() => {
    resetTabs();
  });

  it("updates standalone terminal cwd and title", () => {
    const terminalInfo: TerminalTabInfo = {
      sessionId: 1,
      title: "old-dir",
      cwd: "/tmp/old-dir",
    };
    openTabs.set([{ kind: "terminal", terminal: terminalInfo }]);
    activeTabIndex.set(0);

    onTerminalCwdChanged(1, "/tmp/new-dir");

    const tabs = get(openTabs);
    expect(tabs[0].kind).toBe("terminal");
    const tab = tabs[0] as Extract<Tab, { kind: "terminal" }>;
    expect(tab.terminal.cwd).toBe("/tmp/new-dir");
    expect(tab.terminal.title).toBe("new-dir");
  });

  it("auto-links standalone terminal to matching project tab", () => {
    addProjectTab(mockProject);
    const terminalInfo: TerminalTabInfo = {
      sessionId: 1,
      title: "tmp",
      cwd: "/tmp",
    };
    openTabs.update((tabs) => [
      ...tabs,
      { kind: "terminal", terminal: terminalInfo },
    ]);

    onTerminalCwdChanged(1, mockProject.path);

    const tabs = get(openTabs);
    expect(tabs.length).toBe(1);
    expect(tabs[0].kind).toBe("composite");
    const composite = tabs[0] as Extract<Tab, { kind: "composite" }>;
    expect(composite.project.path).toBe(mockProject.path);
    expect(composite.segments.length).toBe(1);
    expect(composite.segments[0].type).toBe("terminal");
  });

  it("updates cwd inside a composite segment when still in project", () => {
    const terminalInfo: TerminalTabInfo = {
      sessionId: 1,
      title: "BeardGit",
      cwd: mockProject.path,
    };
    openTabs.set([
      {
        kind: "composite",
        project: mockProject,
        segments: [{ type: "terminal", info: terminalInfo }],
        activeSegmentIndex: 0,
      },
    ]);

    // cd to the same path (e.g. after re-entering it) — should update in place.
    onTerminalCwdChanged(1, mockProject.path);

    const tabs = get(openTabs);
    expect(tabs[0].kind).toBe("composite");
    const composite = tabs[0] as Extract<Tab, { kind: "composite" }>;
    const seg = composite.segments[0] as Extract<
      (typeof composite.segments)[0],
      { type: "terminal" }
    >;
    expect(seg.info.cwd).toBe(mockProject.path);
  });

  it("detaches terminal from composite when cwd moves to unrelated path", () => {
    const terminalInfo: TerminalTabInfo = {
      sessionId: 1,
      title: "BeardGit",
      cwd: mockProject.path,
    };
    openTabs.set([
      {
        kind: "composite",
        project: mockProject,
        segments: [{ type: "terminal", info: terminalInfo }],
        activeSegmentIndex: 0,
      },
    ]);

    onTerminalCwdChanged(1, "/tmp/somewhere-else");

    const tabs = get(openTabs);
    expect(tabs.length).toBe(2);
    expect(tabs[0].kind).toBe("project");
    expect(tabs[1].kind).toBe("terminal");
    const terminal = tabs[1] as Extract<Tab, { kind: "terminal" }>;
    expect(terminal.terminal.cwd).toBe("/tmp/somewhere-else");
    expect(terminal.terminal.title).toBe("somewhere-else");
  });

  it("does nothing for unknown session IDs", () => {
    openTabs.set([{ kind: "project", project: mockProject }]);
    onTerminalCwdChanged(999, "/tmp/unknown");
    const tabs = get(openTabs);
    expect(tabs.length).toBe(1);
    expect(tabs[0].kind).toBe("project");
  });
});
