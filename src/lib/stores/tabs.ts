/**
 * Unified tab store — manages project, terminal, and composite tabs.
 *
 * Project tabs delegate their lifecycle to the existing projects store
 * (which calls Rust IPC). Terminal tabs are frontend-only state that
 * spawn/kill PTY sessions via tauri.ts.
 *
 * Composite tabs merge a project + N linked segments (terminals, AI
 * terminals, worktrees) into one segmented tab. Segments are always
 * sorted: Worktrees → AI Terminals → Regular Terminals.
 */

import { writable, derived, get } from "svelte/store";
import type { Tab, ProjectInfo, TerminalTabInfo, LinkedSegment, AiProviderKind } from "../types";
import { terminalSpawn, terminalKill, aiLaunchInteractive, aiResumeSession } from "../api/tauri";
import { onTerminalOutput, offTerminalOutput } from "./terminal";

export const openTabs = writable<Tab[]>([]);
export const activeTabIndex = writable<number>(-1);

/** The currently active tab (project, terminal, or composite), or null. */
export const activeTab = derived(
  [openTabs, activeTabIndex],
  ([$openTabs, $activeTabIndex]) =>
    $activeTabIndex >= 0 ? $openTabs[$activeTabIndex] ?? null : null,
);

/** The active project if a project or composite tab is focused, otherwise null. */
export const activeProjectFromTab = derived(activeTab, ($tab) => {
  if ($tab?.kind === "project") return $tab.project;
  if ($tab?.kind === "composite") return $tab.project;
  return null;
});

/** All tabs that contain a project (for backward compat with Rust IPC indexing). */
export const projectTabs = derived(openTabs, ($tabs) =>
  $tabs
    .filter((t) => t.kind === "project" || t.kind === "composite")
    .map((t) => ({
      kind: "project" as const,
      project: t.kind === "project" ? t.project : (t as Extract<Tab, { kind: "composite" }>).project,
    })),
);

/**
 * Map a Rust project index (0-based within projects-only) to the unified
 * tab array index.
 */
export function projectIndexToTabIndex(projectIndex: number): number {
  const tabs = get(openTabs);
  let pIdx = 0;
  for (let i = 0; i < tabs.length; i++) {
    if (tabs[i].kind === "project" || tabs[i].kind === "composite") {
      if (pIdx === projectIndex) return i;
      pIdx++;
    }
  }
  return -1;
}

/**
 * Map a unified tab index to a Rust project index.
 * Returns -1 if the tab has no project.
 */
export function tabIndexToProjectIndex(tabIndex: number): number {
  const tabs = get(openTabs);
  if (tabIndex < 0 || tabIndex >= tabs.length) return -1;
  const tab = tabs[tabIndex];
  if (tab.kind !== "project" && tab.kind !== "composite") return -1;
  let pIdx = 0;
  for (let i = 0; i < tabIndex; i++) {
    if (tabs[i].kind === "project" || tabs[i].kind === "composite") pIdx++;
  }
  return pIdx;
}

/**
 * Sort segments in canonical order:
 * 1. Worktrees first
 * 2. AI terminals second (terminal with provider)
 * 3. Regular terminals last
 */
function sortSegments(segments: LinkedSegment[]): LinkedSegment[] {
  return [...segments].sort((a, b) => {
    const order = (s: LinkedSegment) => {
      if (s.type === "worktree") return 0;
      if (s.type === "terminal" && s.info.provider) return 1;
      return 2;
    };
    return order(a) - order(b);
  });
}

/**
 * Find the new index of a segment after sorting, given its identity before sort.
 * Used to update activeSegmentIndex after a sort mutation.
 */
function findSegmentIndex(segments: LinkedSegment[], segment: LinkedSegment): number {
  return segments.findIndex((s) => {
    if (s.type === "terminal" && segment.type === "terminal") {
      return s.info.sessionId === segment.info.sessionId;
    }
    if (s.type === "worktree" && segment.type === "worktree") {
      return s.path === segment.path;
    }
    return false;
  });
}

/** Insert a project tab at the end and return its unified index. */
export function addProjectTab(project: ProjectInfo): number {
  const tabs = get(openTabs);
  const newTab: Tab = { kind: "project", project };
  const newTabs = [...tabs, newTab];
  openTabs.set(newTabs);
  return newTabs.length - 1;
}

/** Remove a tab by unified index. Returns the new suggested active index. */
export function removeTab(tabIndex: number): number {
  const tabs = get(openTabs);
  if (tabIndex < 0 || tabIndex >= tabs.length) return get(activeTabIndex);
  const newTabs = tabs.filter((_, i) => i !== tabIndex);
  openTabs.set(newTabs);

  const currentActive = get(activeTabIndex);
  if (newTabs.length === 0) return -1;
  if (tabIndex < currentActive) return currentActive - 1;
  if (tabIndex === currentActive) return Math.min(tabIndex, newTabs.length - 1);
  return currentActive;
}

/** Update the project info for all matching project tabs (after refresh). */
export function syncProjectTabs(projects: ProjectInfo[]): void {
  openTabs.update((tabs) =>
    tabs.map((tab) => {
      if (tab.kind === "project") {
        const updated = projects.find((p) => p.path === tab.project.path);
        return updated ? { ...tab, project: updated } : tab;
      }
      if (tab.kind === "composite") {
        const updated = projects.find((p) => p.path === tab.project.path);
        return updated ? { ...tab, project: updated } : tab;
      }
      return tab;
    }),
  );
}

/** Open a new terminal tab. If cwd matches an open project tab, promote to
 *  composite in-place. If cwd matches a composite, add a new terminal segment.
 *  Otherwise create a standalone terminal tab.
 *  Returns the session ID. */
export async function openTerminalTab(
  cwd: string,
  title: string,
): Promise<number> {
  const sessionId = await terminalSpawn(cwd, 80, 24);
  const info: TerminalTabInfo = { sessionId, title, cwd };
  const segment: LinkedSegment = { type: "terminal", info };

  const tabs = get(openTabs);
  // Check if there's a simple project tab matching this cwd — promote to composite
  const projectIdx = tabs.findIndex(
    (t) => t.kind === "project" && t.project.path === cwd,
  );

  if (projectIdx >= 0) {
    const projectTab = tabs[projectIdx] as Extract<Tab, { kind: "project" }>;
    const segments = sortSegments([segment]);
    const newTabs = [...tabs];
    newTabs[projectIdx] = {
      kind: "composite",
      project: projectTab.project,
      segments,
      activeSegmentIndex: findSegmentIndex(segments, segment),
    };
    openTabs.set(newTabs);
    activeTabIndex.set(projectIdx);
    return sessionId;
  }

  // Check if there's already a composite tab for this project — add a new segment
  const compositeIdx = tabs.findIndex(
    (t) => t.kind === "composite" && t.project.path === cwd,
  );
  if (compositeIdx >= 0) {
    const composite = tabs[compositeIdx] as Extract<Tab, { kind: "composite" }>;
    const newSegments = sortSegments([...composite.segments, segment]);
    const newTabs = [...tabs];
    newTabs[compositeIdx] = {
      ...composite,
      segments: newSegments,
      activeSegmentIndex: findSegmentIndex(newSegments, segment),
    };
    openTabs.set(newTabs);
    activeTabIndex.set(compositeIdx);
    return sessionId;
  }

  // No matching project — standalone terminal tab at end
  const newTabs = [...tabs, { kind: "terminal" as const, terminal: info }];
  openTabs.set(newTabs);
  activeTabIndex.set(newTabs.length - 1);
  return sessionId;
}

/** Open a standalone terminal tab, always creating a new tab regardless of existing project tabs. */
export async function openStandaloneTerminal(
  cwd: string,
  title: string,
): Promise<number> {
  const sessionId = await terminalSpawn(cwd, 80, 24);
  const info: TerminalTabInfo = { sessionId, title, cwd };
  const tabs = get(openTabs);
  const newTabs = [...tabs, { kind: "terminal" as const, terminal: info }];
  openTabs.set(newTabs);
  activeTabIndex.set(newTabs.length - 1);
  return sessionId;
}

/**
 * Launch an AI provider in a terminal tab via `ai_launch_interactive`.
 * If cwd matches an open project tab, promotes it to composite in-place.
 * If cwd matches a composite, adds a new AI terminal segment.
 * Otherwise creates a standalone terminal tab. The tab stores the provider
 * kind so the UI can show the brand icon.
 */
export async function openAiTerminalTab(
  cwd: string,
  title: string,
  provider: AiProviderKind,
): Promise<number> {
  const sessionId = await aiLaunchInteractive(provider);
  const info: TerminalTabInfo = { sessionId, title, cwd, provider };
  const segment: LinkedSegment = { type: "terminal", info };

  const tabs = get(openTabs);
  // Check if there's a simple project tab matching this cwd — promote to composite
  const projectIdx = tabs.findIndex(
    (t) => t.kind === "project" && t.project.path === cwd,
  );

  if (projectIdx >= 0) {
    const projectTab = tabs[projectIdx] as Extract<Tab, { kind: "project" }>;
    const segments = sortSegments([segment]);
    const newTabs = [...tabs];
    newTabs[projectIdx] = {
      kind: "composite",
      project: projectTab.project,
      segments,
      activeSegmentIndex: findSegmentIndex(segments, segment),
    };
    openTabs.set(newTabs);
    activeTabIndex.set(projectIdx);
    return sessionId;
  }

  // Check if there's already a composite tab for this project — add a new segment
  const compositeIdx = tabs.findIndex(
    (t) => t.kind === "composite" && t.project.path === cwd,
  );
  if (compositeIdx >= 0) {
    const composite = tabs[compositeIdx] as Extract<Tab, { kind: "composite" }>;
    const newSegments = sortSegments([...composite.segments, segment]);
    const newTabs = [...tabs];
    newTabs[compositeIdx] = {
      ...composite,
      segments: newSegments,
      activeSegmentIndex: findSegmentIndex(newSegments, segment),
    };
    openTabs.set(newTabs);
    activeTabIndex.set(compositeIdx);
    return sessionId;
  }

  // Standalone terminal tab at end
  const newTabs = [...tabs, { kind: "terminal" as const, terminal: info }];
  openTabs.set(newTabs);
  activeTabIndex.set(newTabs.length - 1);
  return sessionId;
}

/**
 * Resume an existing AI session in a terminal tab.
 *
 * The Rust side has already spawned the PTY via `ai_resume_session`; this
 * helper attaches the returned session id to the UI using the same
 * promote/segment/standalone rules as `openAiTerminalTab`. Returns `true`
 * on success, `false` when the provider doesn't advertise a resume command.
 */
export async function resumeAiSessionTab(
  cwd: string,
  title: string,
  provider: AiProviderKind,
  aiSessionId: string,
): Promise<boolean> {
  const sessionId = await aiResumeSession(provider, aiSessionId);
  if (sessionId === null) return false;

  const info: TerminalTabInfo = { sessionId, title, cwd, provider };
  const segment: LinkedSegment = { type: "terminal", info };

  const tabs = get(openTabs);

  const projectIdx = tabs.findIndex(
    (t) => t.kind === "project" && t.project.path === cwd,
  );
  if (projectIdx >= 0) {
    const projectTab = tabs[projectIdx] as Extract<Tab, { kind: "project" }>;
    const segments = sortSegments([segment]);
    const newTabs = [...tabs];
    newTabs[projectIdx] = {
      kind: "composite",
      project: projectTab.project,
      segments,
      activeSegmentIndex: findSegmentIndex(segments, segment),
    };
    openTabs.set(newTabs);
    activeTabIndex.set(projectIdx);
    return true;
  }

  const compositeIdx = tabs.findIndex(
    (t) => t.kind === "composite" && t.project.path === cwd,
  );
  if (compositeIdx >= 0) {
    const composite = tabs[compositeIdx] as Extract<Tab, { kind: "composite" }>;
    const newSegments = sortSegments([...composite.segments, segment]);
    const newTabs = [...tabs];
    newTabs[compositeIdx] = {
      ...composite,
      segments: newSegments,
      activeSegmentIndex: findSegmentIndex(newSegments, segment),
    };
    openTabs.set(newTabs);
    activeTabIndex.set(compositeIdx);
    return true;
  }

  const newTabs = [...tabs, { kind: "terminal" as const, terminal: info }];
  openTabs.set(newTabs);
  activeTabIndex.set(newTabs.length - 1);
  return true;
}

/**
 * Add a worktree segment to the project tab matching `projectPath`.
 * Promotes to composite if it's a simple project tab.
 */
export function addWorktreeSegment(
  projectPath: string,
  worktreePath: string,
  branch: string,
): void {
  const segment: LinkedSegment = { type: "worktree", path: worktreePath, branch };
  const tabs = get(openTabs);

  // Check composite first
  const compositeIdx = tabs.findIndex(
    (t) => t.kind === "composite" && t.project.path === projectPath,
  );
  if (compositeIdx >= 0) {
    const composite = tabs[compositeIdx] as Extract<Tab, { kind: "composite" }>;
    const newSegments = sortSegments([...composite.segments, segment]);
    const newTabs = [...tabs];
    newTabs[compositeIdx] = {
      ...composite,
      segments: newSegments,
      activeSegmentIndex: findSegmentIndex(newSegments, segment),
    };
    openTabs.set(newTabs);
    activeTabIndex.set(compositeIdx);
    return;
  }

  // Check simple project tab
  const projectIdx = tabs.findIndex(
    (t) => t.kind === "project" && t.project.path === projectPath,
  );
  if (projectIdx >= 0) {
    const projectTab = tabs[projectIdx] as Extract<Tab, { kind: "project" }>;
    const segments = sortSegments([segment]);
    const newTabs = [...tabs];
    newTabs[projectIdx] = {
      kind: "composite",
      project: projectTab.project,
      segments,
      activeSegmentIndex: findSegmentIndex(segments, segment),
    };
    openTabs.set(newTabs);
    activeTabIndex.set(projectIdx);
  }
}

/**
 * Switch the active segment of a composite tab.
 * -1 = project segment, 0..N = index into segments array.
 */
export function switchSegment(tabIndex: number, segmentIndex: number): void {
  openTabs.update((tabs) =>
    tabs.map((tab, i) => {
      if (i !== tabIndex || tab.kind !== "composite") return tab;
      return { ...tab, activeSegmentIndex: segmentIndex };
    }),
  );
}

/**
 * Close a segment at the given index from a composite tab.
 * Kills the terminal session if it's a terminal segment.
 * If no segments remain, demotes to simple project tab.
 */
export async function closeSegment(tabIndex: number, segmentIndex: number): Promise<void> {
  const tabs = get(openTabs);
  const tab = tabs[tabIndex];
  if (!tab || tab.kind !== "composite") return;
  if (segmentIndex < 0 || segmentIndex >= tab.segments.length) return;

  const segment = tab.segments[segmentIndex];

  // Kill terminal session if applicable
  if (segment.type === "terminal") {
    try {
      await terminalKill(segment.info.sessionId);
    } catch { /* already dead */ }
    offTerminalOutput(segment.info.sessionId);
  }

  const newSegments = tab.segments.filter((_, idx) => idx !== segmentIndex);
  const newTabs = [...tabs];

  if (newSegments.length === 0) {
    // Demote to simple project tab
    newTabs[tabIndex] = { kind: "project", project: tab.project };
  } else {
    // Adjust activeSegmentIndex
    let newActiveIdx = tab.activeSegmentIndex;
    if (tab.activeSegmentIndex === segmentIndex) {
      // Closed the active segment — switch to project
      newActiveIdx = -1;
    } else if (tab.activeSegmentIndex > segmentIndex) {
      // Active segment shifted left
      newActiveIdx = tab.activeSegmentIndex - 1;
    }
    newTabs[tabIndex] = {
      ...tab,
      segments: newSegments,
      activeSegmentIndex: newActiveIdx,
    };
  }

  openTabs.set(newTabs);
}

/**
 * Close the project segment of a composite tab, reverting to standalone terminal tab(s).
 * If the composite has exactly one terminal segment, demotes to a standalone terminal tab.
 * If it has multiple segments, only terminals are kept as standalone tabs.
 */
export function closeProjectSegment(tabIndex: number): void {
  const tabs = get(openTabs);
  const tab = tabs[tabIndex];
  if (!tab || tab.kind !== "composite") return;

  // Collect all terminal segments — worktrees are dropped when project closes
  const terminalSegments = tab.segments.filter(
    (s): s is Extract<LinkedSegment, { type: "terminal" }> => s.type === "terminal",
  );

  const newTabs = [...tabs];

  if (terminalSegments.length === 1) {
    // Single terminal — demote to standalone terminal tab
    newTabs[tabIndex] = { kind: "terminal", terminal: terminalSegments[0].info };
  } else if (terminalSegments.length > 1) {
    // Multiple terminals — replace with first terminal, add rest as standalone tabs
    newTabs[tabIndex] = { kind: "terminal", terminal: terminalSegments[0].info };
    for (let j = 1; j < terminalSegments.length; j++) {
      newTabs.splice(tabIndex + j, 0, { kind: "terminal", terminal: terminalSegments[j].info });
    }
  } else {
    // No terminals — just remove the composite, leaving nothing (remove the tab)
    newTabs.splice(tabIndex, 1);
  }

  openTabs.set(newTabs);
}

/** Close a terminal tab by session ID. Handles both standalone and composite. */
export async function closeTerminalTab(sessionId: number): Promise<void> {
  try {
    await terminalKill(sessionId);
  } catch { /* already dead */ }
  offTerminalOutput(sessionId);

  const tabs = get(openTabs);

  // Check composite tabs — find the segment with this session and close it
  for (let i = 0; i < tabs.length; i++) {
    const tab = tabs[i];
    if (tab.kind !== "composite") continue;
    const segIdx = tab.segments.findIndex(
      (s) => s.type === "terminal" && s.info.sessionId === sessionId,
    );
    if (segIdx < 0) continue;

    const newSegments = tab.segments.filter((_, idx) => idx !== segIdx);
    const newTabs = [...tabs];

    if (newSegments.length === 0) {
      // Demote to project
      newTabs[i] = { kind: "project", project: tab.project };
    } else {
      let newActiveIdx = tab.activeSegmentIndex;
      if (tab.activeSegmentIndex === segIdx) {
        newActiveIdx = -1;
      } else if (tab.activeSegmentIndex > segIdx) {
        newActiveIdx = tab.activeSegmentIndex - 1;
      }
      newTabs[i] = {
        ...tab,
        segments: newSegments,
        activeSegmentIndex: newActiveIdx,
      };
    }
    openTabs.set(newTabs);
    return;
  }

  // Standalone terminal — remove entirely
  const tabIndex = tabs.findIndex(
    (t) => t.kind === "terminal" && t.terminal.sessionId === sessionId,
  );
  if (tabIndex < 0) return;

  const newActiveIdx = removeTab(tabIndex);
  activeTabIndex.set(newActiveIdx);
}

/** Process name to AiProviderKind mapping for foreground-process detection. */
const PROCESS_TO_PROVIDER: Record<string, AiProviderKind> = {
  claude: "claude_code",
  codex: "codex",
  opencode: "open_code",
};

/**
 * Handle a terminal cwd change (from OSC 7 detection).
 * Updates the terminal's cwd and title, and auto-links standalone terminals
 * to matching project tabs (promotes to composite). If the cwd inside a
 * composite moves outside the project root, the segment detaches and either
 * attaches to another project tab or becomes a standalone terminal.
 */
export function onTerminalCwdChanged(sessionId: number, cwd: string): void {
  const tabs = get(openTabs);
  const dirName = cwd.split("/").filter(Boolean).pop() ?? cwd;

  // ── Case 1: standalone terminal tab ────────────────────────────────────
  const standaloneIdx = tabs.findIndex(
    (t) => t.kind === "terminal" && t.terminal.sessionId === sessionId,
  );
  if (standaloneIdx >= 0) {
    const tab = tabs[standaloneIdx] as Extract<Tab, { kind: "terminal" }>;
    const updatedInfo: TerminalTabInfo = {
      ...tab.terminal,
      cwd,
      title: dirName,
    };

    // a) cwd matches an open simple project — promote that project to composite
    const projectIdx = tabs.findIndex(
      (t) => t.kind === "project" && t.project.path === cwd,
    );
    if (projectIdx >= 0) {
      const projectTab = tabs[projectIdx] as Extract<Tab, { kind: "project" }>;
      const segment: LinkedSegment = { type: "terminal", info: updatedInfo };
      const segments = sortSegments([segment]);
      const newTabs = [...tabs];
      newTabs.splice(standaloneIdx, 1);
      const adjustedProjectIdx =
        projectIdx > standaloneIdx ? projectIdx - 1 : projectIdx;
      newTabs[adjustedProjectIdx] = {
        kind: "composite",
        project: projectTab.project,
        segments,
        activeSegmentIndex: findSegmentIndex(segments, segment),
      };
      openTabs.set(newTabs);
      activeTabIndex.set(adjustedProjectIdx);
      return;
    }

    // b) cwd matches an open composite — append the terminal as a segment
    const compositeIdx = tabs.findIndex(
      (t) => t.kind === "composite" && t.project.path === cwd,
    );
    if (compositeIdx >= 0 && compositeIdx !== standaloneIdx) {
      const composite = tabs[compositeIdx] as Extract<
        Tab,
        { kind: "composite" }
      >;
      const segment: LinkedSegment = { type: "terminal", info: updatedInfo };
      const newSegments = sortSegments([...composite.segments, segment]);
      const newTabs = [...tabs];
      newTabs.splice(standaloneIdx, 1);
      const adjustedCompositeIdx =
        compositeIdx > standaloneIdx ? compositeIdx - 1 : compositeIdx;
      newTabs[adjustedCompositeIdx] = {
        ...composite,
        segments: newSegments,
        activeSegmentIndex: findSegmentIndex(newSegments, segment),
      };
      openTabs.set(newTabs);
      activeTabIndex.set(adjustedCompositeIdx);
      return;
    }

    // c) no matching project — just update cwd + title
    const newTabs = [...tabs];
    newTabs[standaloneIdx] = { kind: "terminal", terminal: updatedInfo };
    openTabs.set(newTabs);
    return;
  }

  // ── Case 2: terminal inside a composite ────────────────────────────────
  for (let i = 0; i < tabs.length; i++) {
    const tab = tabs[i];
    if (tab.kind !== "composite") continue;
    const segIdx = tab.segments.findIndex(
      (s) => s.type === "terminal" && s.info.sessionId === sessionId,
    );
    if (segIdx < 0) continue;

    const seg = tab.segments[segIdx] as Extract<
      LinkedSegment,
      { type: "terminal" }
    >;
    const updatedInfo: TerminalTabInfo = {
      ...seg.info,
      cwd,
      title: dirName,
    };

    // Same project — just update the segment
    if (cwd === tab.project.path) {
      const newSegments = [...tab.segments];
      newSegments[segIdx] = { type: "terminal", info: updatedInfo };
      const newTabs = [...tabs];
      newTabs[i] = { ...tab, segments: newSegments };
      openTabs.set(newTabs);
      return;
    }

    // Moved to a different path — detach from this composite
    const newSegments = tab.segments.filter((_, idx) => idx !== segIdx);
    const newTabs = [...tabs];

    if (newSegments.length === 0) {
      newTabs[i] = { kind: "project", project: tab.project };
    } else {
      let newActiveIdx = tab.activeSegmentIndex;
      if (tab.activeSegmentIndex === segIdx) {
        newActiveIdx = -1;
      } else if (tab.activeSegmentIndex > segIdx) {
        newActiveIdx = tab.activeSegmentIndex - 1;
      }
      newTabs[i] = {
        ...tab,
        segments: newSegments,
        activeSegmentIndex: newActiveIdx,
      };
    }

    // Attach to another matching project (simple or composite) if one exists
    const newProjectIdx = newTabs.findIndex(
      (t) => t.kind === "project" && t.project.path === cwd,
    );
    if (newProjectIdx >= 0) {
      const projectTab = newTabs[newProjectIdx] as Extract<
        Tab,
        { kind: "project" }
      >;
      const segment: LinkedSegment = { type: "terminal", info: updatedInfo };
      const segments = sortSegments([segment]);
      newTabs[newProjectIdx] = {
        kind: "composite",
        project: projectTab.project,
        segments,
        activeSegmentIndex: findSegmentIndex(segments, segment),
      };
      openTabs.set(newTabs);
      activeTabIndex.set(newProjectIdx);
      return;
    }

    const newCompositeIdx = newTabs.findIndex(
      (t) => t.kind === "composite" && t.project.path === cwd,
    );
    if (newCompositeIdx >= 0) {
      const composite = newTabs[newCompositeIdx] as Extract<
        Tab,
        { kind: "composite" }
      >;
      const segment: LinkedSegment = { type: "terminal", info: updatedInfo };
      const segments = sortSegments([...composite.segments, segment]);
      newTabs[newCompositeIdx] = {
        ...composite,
        segments,
        activeSegmentIndex: findSegmentIndex(segments, segment),
      };
      openTabs.set(newTabs);
      activeTabIndex.set(newCompositeIdx);
      return;
    }

    // No matching project — create a standalone terminal tab at end
    newTabs.push({ kind: "terminal", terminal: updatedInfo });
    openTabs.set(newTabs);
    activeTabIndex.set(newTabs.length - 1);
    return;
  }
}

/**
 * Handle a foreground process change detection.
 * Maps known AI CLI binary names to provider kinds and updates the terminal's
 * `provider` field, triggering the brand-icon swap in tab UI.
 */
export function onTerminalProcessChanged(
  sessionId: number,
  processName: string | null,
): void {
  const provider: AiProviderKind | undefined = processName
    ? PROCESS_TO_PROVIDER[processName]
    : undefined;

  const tabs = get(openTabs);

  // Standalone terminal tabs
  const standaloneIdx = tabs.findIndex(
    (t) => t.kind === "terminal" && t.terminal.sessionId === sessionId,
  );
  if (standaloneIdx >= 0) {
    const tab = tabs[standaloneIdx] as Extract<Tab, { kind: "terminal" }>;
    if (tab.terminal.provider === provider) return;
    const newTabs = [...tabs];
    newTabs[standaloneIdx] = {
      kind: "terminal",
      terminal: { ...tab.terminal, provider },
    };
    openTabs.set(newTabs);
    return;
  }

  // Composite tabs — segment-scoped
  for (let i = 0; i < tabs.length; i++) {
    const tab = tabs[i];
    if (tab.kind !== "composite") continue;
    const segIdx = tab.segments.findIndex(
      (s) => s.type === "terminal" && s.info.sessionId === sessionId,
    );
    if (segIdx < 0) continue;

    const seg = tab.segments[segIdx] as Extract<
      LinkedSegment,
      { type: "terminal" }
    >;
    if (seg.info.provider === provider) return;

    const updatedSeg: LinkedSegment = {
      type: "terminal",
      info: { ...seg.info, provider },
    };
    const replaced = [...tab.segments];
    replaced[segIdx] = updatedSeg;

    // Re-sort: provider affects segment order (AI terminals before regular).
    const sorted = sortSegments(replaced);
    // Find where the old active segment ended up after sorting.
    let newActiveIdx = tab.activeSegmentIndex;
    if (tab.activeSegmentIndex >= 0 && tab.activeSegmentIndex < replaced.length) {
      const prevActive = replaced[tab.activeSegmentIndex];
      newActiveIdx = findSegmentIndex(sorted, prevActive);
    }

    const newTabs = [...tabs];
    newTabs[i] = { ...tab, segments: sorted, activeSegmentIndex: newActiveIdx };
    openTabs.set(newTabs);
    return;
  }
}

/** Remove a terminal by session ID without killing (shell already exited). */
export function removeTerminalTabBySession(sessionId: number): void {
  const tabs = get(openTabs);

  // Check composite tabs — remove the segment
  for (let i = 0; i < tabs.length; i++) {
    const tab = tabs[i];
    if (tab.kind !== "composite") continue;
    const segIdx = tab.segments.findIndex(
      (s) => s.type === "terminal" && s.info.sessionId === sessionId,
    );
    if (segIdx < 0) continue;

    offTerminalOutput(sessionId);
    const newSegments = tab.segments.filter((_, idx) => idx !== segIdx);
    const newTabs = [...tabs];

    if (newSegments.length === 0) {
      newTabs[i] = { kind: "project", project: tab.project };
    } else {
      let newActiveIdx = tab.activeSegmentIndex;
      if (tab.activeSegmentIndex === segIdx) {
        newActiveIdx = -1;
      } else if (tab.activeSegmentIndex > segIdx) {
        newActiveIdx = tab.activeSegmentIndex - 1;
      }
      newTabs[i] = {
        ...tab,
        segments: newSegments,
        activeSegmentIndex: newActiveIdx,
      };
    }
    openTabs.set(newTabs);
    return;
  }

  // Standalone terminal
  const tabIndex = tabs.findIndex(
    (t) => t.kind === "terminal" && t.terminal.sessionId === sessionId,
  );
  if (tabIndex < 0) return;

  offTerminalOutput(sessionId);
  const newActiveIdx = removeTab(tabIndex);
  activeTabIndex.set(newActiveIdx);
}

/** Switch to next tab (wraps). */
export function switchToNextTab(): void {
  const tabs = get(openTabs);
  const idx = get(activeTabIndex);
  if (tabs.length <= 1 || idx < 0) return;
  activeTabIndex.set((idx + 1) % tabs.length);
}

/** Switch to previous tab (wraps). */
export function switchToPrevTab(): void {
  const tabs = get(openTabs);
  const idx = get(activeTabIndex);
  if (tabs.length <= 1 || idx < 0) return;
  activeTabIndex.set((idx - 1 + tabs.length) % tabs.length);
}

/** Find the most recently active project tab index. Returns -1 if none. */
export function findLastProjectTabIndex(): number {
  const tabs = get(openTabs);
  const current = get(activeTabIndex);
  for (let i = current - 1; i >= 0; i--) {
    if (tabs[i].kind === "project" || tabs[i].kind === "composite") return i;
  }
  for (let i = tabs.length - 1; i > current; i--) {
    if (tabs[i].kind === "project" || tabs[i].kind === "composite") return i;
  }
  return -1;
}

/**
 * Get all terminal TerminalTabInfo instances from a composite tab's segments.
 * Useful for iterating over terminals when closing a composite.
 */
export function getCompositeTerminals(tab: Extract<Tab, { kind: "composite" }>): TerminalTabInfo[] {
  return tab.segments
    .filter((s): s is Extract<LinkedSegment, { type: "terminal" }> => s.type === "terminal")
    .map((s) => s.info);
}

/**
 * Check if a composite tab's active segment is a terminal.
 * Returns the TerminalTabInfo if so, null otherwise.
 */
export function getActiveTerminalSegment(tab: Extract<Tab, { kind: "composite" }>): TerminalTabInfo | null {
  if (tab.activeSegmentIndex < 0 || tab.activeSegmentIndex >= tab.segments.length) return null;
  const seg = tab.segments[tab.activeSegmentIndex];
  if (seg.type === "terminal") return seg.info;
  return null;
}
