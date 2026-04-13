/**
 * Unified tab store — manages project, terminal, and composite tabs.
 *
 * Project tabs delegate their lifecycle to the existing projects store
 * (which calls Rust IPC). Terminal tabs are frontend-only state that
 * spawn/kill PTY sessions via tauri.ts.
 *
 * Composite tabs merge a project + linked terminal into one segmented tab.
 */

import { writable, derived, get } from "svelte/store";
import type { Tab, ProjectInfo, TerminalTabInfo } from "../types";
import { terminalSpawn, terminalKill } from "../api/tauri";
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

/** Open a new terminal tab. If cwd matches an open project tab, promote it
 *  to composite in-place. Otherwise create a standalone terminal tab.
 *  Returns the session ID. */
export async function openTerminalTab(
  cwd: string,
  title: string,
): Promise<number> {
  const sessionId = await terminalSpawn(cwd, 80, 24);
  const info: TerminalTabInfo = { sessionId, title, cwd };

  const tabs = get(openTabs);
  // Check if there's a simple project tab matching this cwd — promote to composite
  const projectIdx = tabs.findIndex(
    (t) => t.kind === "project" && t.project.path === cwd,
  );

  if (projectIdx >= 0) {
    const projectTab = tabs[projectIdx] as Extract<Tab, { kind: "project" }>;
    const newTabs = [...tabs];
    newTabs[projectIdx] = {
      kind: "composite",
      project: projectTab.project,
      terminal: info,
      activeSegment: "terminal",
    };
    openTabs.set(newTabs);
    activeTabIndex.set(projectIdx);
    return sessionId;
  }

  // Also check if there's already a composite tab for this project (already has terminal)
  const compositeIdx = tabs.findIndex(
    (t) => t.kind === "composite" && t.project.path === cwd,
  );
  if (compositeIdx >= 0) {
    // Already has a terminal — just switch to its terminal segment
    switchSegment(compositeIdx, "terminal");
    activeTabIndex.set(compositeIdx);
    // Kill the new session since we don't need it
    try { await terminalKill(sessionId); } catch { /* ok */ }
    const existing = tabs[compositeIdx] as Extract<Tab, { kind: "composite" }>;
    return existing.terminal.sessionId;
  }

  // No matching project — standalone terminal tab at end
  const newTabs = [...tabs, { kind: "terminal" as const, terminal: info }];
  openTabs.set(newTabs);
  activeTabIndex.set(newTabs.length - 1);
  return sessionId;
}

/** Switch the active segment of a composite tab. */
export function switchSegment(tabIndex: number, segment: "project" | "terminal"): void {
  openTabs.update((tabs) =>
    tabs.map((tab, i) => {
      if (i !== tabIndex || tab.kind !== "composite") return tab;
      return { ...tab, activeSegment: segment };
    }),
  );
}

/** Close the terminal segment of a composite tab, reverting to simple project tab. */
export async function closeTerminalSegment(tabIndex: number): Promise<void> {
  const tabs = get(openTabs);
  const tab = tabs[tabIndex];
  if (!tab || tab.kind !== "composite") return;

  try {
    await terminalKill(tab.terminal.sessionId);
  } catch { /* already dead */ }
  offTerminalOutput(tab.terminal.sessionId);

  const newTabs = [...tabs];
  newTabs[tabIndex] = { kind: "project", project: tab.project };
  openTabs.set(newTabs);
}

/** Close the project segment of a composite tab, reverting to standalone terminal tab. */
export function closeProjectSegment(tabIndex: number): void {
  const tabs = get(openTabs);
  const tab = tabs[tabIndex];
  if (!tab || tab.kind !== "composite") return;

  const newTabs = [...tabs];
  newTabs[tabIndex] = { kind: "terminal", terminal: tab.terminal };
  openTabs.set(newTabs);
}

/** Close a terminal tab by session ID. Handles both standalone and composite. */
export async function closeTerminalTab(sessionId: number): Promise<void> {
  try {
    await terminalKill(sessionId);
  } catch { /* already dead */ }
  offTerminalOutput(sessionId);

  const tabs = get(openTabs);

  // Check composite tabs — demote to project
  const compositeIdx = tabs.findIndex(
    (t) => t.kind === "composite" && t.terminal.sessionId === sessionId,
  );
  if (compositeIdx >= 0) {
    const tab = tabs[compositeIdx] as Extract<Tab, { kind: "composite" }>;
    const newTabs = [...tabs];
    newTabs[compositeIdx] = { kind: "project", project: tab.project };
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

/** Remove a terminal by session ID without killing (shell already exited). */
export function removeTerminalTabBySession(sessionId: number): void {
  const tabs = get(openTabs);

  // Check composite tabs — demote to project
  const compositeIdx = tabs.findIndex(
    (t) => t.kind === "composite" && t.terminal.sessionId === sessionId,
  );
  if (compositeIdx >= 0) {
    offTerminalOutput(sessionId);
    const tab = tabs[compositeIdx] as Extract<Tab, { kind: "composite" }>;
    const newTabs = [...tabs];
    newTabs[compositeIdx] = { kind: "project", project: tab.project };
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
