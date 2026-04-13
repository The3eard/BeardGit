/**
 * Projects store — project-specific tab management.
 *
 * Delegates tab lifecycle (add/remove/navigate) to the unified tabs store.
 * Owns project-specific Rust IPC (open, close, switch) and state clearing.
 */

import { derived, get } from "svelte/store";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectInfo } from "../types";
import {
  openProject as apiOpenProject,
  closeProject as apiCloseProject,
  switchProject as apiSwitchProject,
  getOpenProjects as apiGetOpenProjects,
  getActiveProjectIndex as apiGetActiveProjectIndex,
  restoreProjects as apiRestoreProjects,
  getBranches as apiGetBranches,
  getStatusSummary as apiGetStatusSummary,
  detectProject,
} from "../api/tauri";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { repoInfo, branches, isLoading, registerWatcher } from "./repo";
import {
  checkStatus as checkProviderStatus,
  stopAllPolling,
  ciRuns,
  selectedCiRun,
  selectedCiRunId,
  jobLog,
  hasMoreCiRuns,
  loadingDetail,
} from "./provider";
import { refreshStatuses } from "./changes";
import { refreshUserEmails, clearGraphState, cacheViewport, restoreCachedViewport } from "./graph";
import * as m from "$lib/paraglide/messages";
import { clearBranchState } from "./branches";
import { clearTagState } from "./tags";
import { clearStashState } from "./stashes";
import { clearBlameState } from "./blame";
import { clearWorktreeState } from "./worktrees";
import { clearMrPrState } from "./mr-pr";
import { clearReflogState } from "./reflog";
import { refreshConflictStatus } from "./conflict";
import {
  openTabs,
  activeTabIndex,
  activeTab,
  activeProjectFromTab,
  projectTabs,
  addProjectTab,
  removeTab,
  syncProjectTabs,
  tabIndexToProjectIndex,
  projectIndexToTabIndex,
  switchToNextTab as tabsNext,
  switchToPrevTab as tabsPrev,
  closeTerminalTab,
  switchSegment,
  closeTerminalSegment,
  closeProjectSegment,
} from "./tabs";
import { writable } from "svelte/store";

// Re-export for backward compatibility with components that read these.
export { openTabs, activeTabIndex };
export { switchSegment, closeTerminalSegment, closeProjectSegment };

/** List of open projects derived from the unified tab array. */
export const openProjects = derived(projectTabs, ($pt) =>
  $pt.map((t) => t.project),
);

/** Index of the active project in the Rust-side projects vec, or -1. */
export const activeProjectIndex = derived(
  [openTabs, activeTabIndex],
  ([$tabs, $idx]) => {
    if ($idx < 0 || $idx >= $tabs.length) return -1;
    if ($tabs[$idx].kind !== "project" && $tabs[$idx].kind !== "composite") return -1;
    let pIdx = 0;
    for (let i = 0; i < $idx; i++) {
      if ($tabs[i].kind === "project" || $tabs[i].kind === "composite") pIdx++;
    }
    return pIdx;
  },
);

/** The currently active project, or null if a terminal tab is active. */
export const activeProject = activeProjectFromTab;

export const addMenuOpen = writable(false);

/**
 * Callback invoked whenever we switch to a different project tab.
 * +page.svelte uses this to reset activeView to "graph" for instant UX.
 */
let projectSwitchCallback: (() => void) | null = null;

/** Register a callback to run on project tab switch. */
export function onProjectSwitch(cb: () => void): void {
  projectSwitchCallback = cb;
}

/** Build a starship-style title: project - branch [↑2 ↓1 +3 !2 ?1 ⚑1] */
async function updateTitleBar(projName: string, branch: string) {
  try {
    const s = await apiGetStatusSummary();
    const parts: string[] = [];
    if (s.ahead > 0) parts.push(`↑${s.ahead}`);
    if (s.behind > 0) parts.push(`↓${s.behind}`);
    if (s.staged > 0) parts.push(`+${s.staged}`);
    if (s.unstaged > 0) parts.push(`!${s.unstaged}`);
    if (s.untracked > 0) parts.push(`?${s.untracked}`);
    if (s.stash_count > 0) parts.push(`⚑${s.stash_count}`);
    const status = parts.length > 0 ? ` [${parts.join(" ")}]` : "";
    getCurrentWindow().setTitle(`${projName} - ${branch}${status}`);
  } catch {
    getCurrentWindow().setTitle(`${projName} - ${branch}`);
  }
}

export async function openProjectTab(path: string) {
  const info = await apiOpenProject(path);

  // Get updated project list from Rust to sync
  const projects = await apiGetOpenProjects();
  syncProjectTabs(projects);

  // If the project already has a tab, switch to it
  const tabs = get(openTabs);
  const existingIdx = tabs.findIndex(
    (t) => t.kind === "project" && t.project.path === path,
  );

  if (existingIdx >= 0) {
    await switchToTab(existingIdx);
  } else {
    // Add new tab
    const tabIdx = addProjectTab(info);
    await switchToTab(tabIdx);
  }
}

/** Switch to a tab by unified index. Handles project-specific loading. */
export async function switchToTab(tabIndex: number) {
  const tabs = get(openTabs);
  if (tabIndex < 0 || tabIndex >= tabs.length) return;

  const prevIdx = get(activeTabIndex);
  const tab = tabs[tabIndex];

  // Set active index immediately for instant tab highlight
  activeTabIndex.set(tabIndex);

  if (tab.kind === "project" || tab.kind === "composite") {
    if (tabIndex !== prevIdx) {
      projectSwitchCallback?.();
    }
    await activateProjectTab(tabIndex);
  } else {
    // Terminal tab — update title bar
    getCurrentWindow().setTitle(`${tab.terminal.title} — BeardGit`);
  }
}

/** Full project activation: Rust switch + state refresh. */
async function activateProjectTab(tabIndex: number) {
  const projectIdx = tabIndexToProjectIndex(tabIndex);
  if (projectIdx < 0) return;

  // Cache the outgoing project's graph viewport for instant restore later
  const prevProject = get(activeProjectFromTab);
  if (prevProject) {
    cacheViewport(prevProject.path);
  }

  stopAllPolling();
  clearGraphState();
  clearBranchState();
  clearTagState();
  clearStashState();
  clearBlameState();
  clearWorktreeState();
  clearMrPrState();
  clearReflogState();

  // Restore cached graph viewport instantly (no loading spinner for graph)
  const tabs = get(openTabs);
  const targetTab = tabs[tabIndex];
  const targetPath = (targetTab?.kind === "project" || targetTab?.kind === "composite") ? targetTab.project.path : null;
  const hasCachedGraph = targetPath ? restoreCachedViewport(targetPath) : false;

  // Only show loading spinner if we have no cached graph to display
  if (!hasCachedGraph) {
    isLoading.set(true);
  }
  try {
    const info = await apiSwitchProject(projectIdx);
    repoInfo.set(info);

    const projects = await apiGetOpenProjects();
    syncProjectTabs(projects);

    const proj = projects[projectIdx];
    const projName = proj?.name ?? info.path.split("/").pop() ?? "";
    const branch = info.head_branch ?? "detached";
    await updateTitleBar(projName, branch);

    const branchList = await apiGetBranches();
    branches.set(branchList);

    await detectProject();
    await checkProviderStatus();

    ciRuns.set([]);
    selectedCiRun.set(null);
    selectedCiRunId.set(null);
    jobLog.set(null);
    hasMoreCiRuns.set(false);
    loadingDetail.set(false);

    await refreshStatuses();
    await refreshUserEmails();
    await refreshConflictStatus();
    await registerWatcher();
  } finally {
    isLoading.set(false);
  }
}

export async function closeTab(tabIndex: number) {
  const tabs = get(openTabs);
  if (tabIndex < 0 || tabIndex >= tabs.length) return;

  const tab = tabs[tabIndex];

  if (tab.kind === "terminal") {
    await closeTerminalTab(tab.terminal.sessionId);
    return;
  }

  if (tab.kind === "composite") {
    // Close terminal first (demotes to project), then close the project
    await closeTerminalTab(tab.terminal.sessionId);
  }

  // Project tab (or demoted composite): close via Rust
  const projectIdx = tabIndexToProjectIndex(tabIndex);
  if (projectIdx < 0) return;

  await apiCloseProject(projectIdx);

  const newActiveIdx = removeTab(tabIndex);

  const projects = await apiGetOpenProjects();
  syncProjectTabs(projects);

  if (get(openTabs).length === 0) {
    activeTabIndex.set(-1);
    getCurrentWindow().setTitle("BeardGit");
    return;
  }

  activeTabIndex.set(newActiveIdx);
  const newTabs = get(openTabs);
  if (newActiveIdx >= 0 && newActiveIdx < newTabs.length &&
      (newTabs[newActiveIdx].kind === "project" || newTabs[newActiveIdx].kind === "composite")) {
    await activateProjectTab(newActiveIdx);
  }
}

export async function openFolderAsProject() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: m.app_open_repo_dialog(),
  });
  if (selected && typeof selected === "string") {
    await openProjectTab(selected);
  }
}

export function toggleAddMenu() {
  addMenuOpen.update((v) => !v);
}

export async function initProjects() {
  const projects = await apiRestoreProjects();

  // Seed the unified tab array with project tabs
  const tabs = projects.map((p) => ({ kind: "project" as const, project: p }));
  openTabs.set(tabs);

  if (projects.length > 0) {
    const activeIdx = await apiGetActiveProjectIndex();
    const rustIdx = activeIdx !== null && activeIdx < projects.length ? activeIdx : 0;
    const tabIdx = projectIndexToTabIndex(rustIdx);
    await switchToTab(tabIdx >= 0 ? tabIdx : 0);
  }
}

export async function switchToNextTab(): Promise<void> {
  const prevTab = get(activeTab);
  tabsNext();
  const newTab = get(activeTab);
  const newIdx = get(activeTabIndex);
  if (newTab && (newTab.kind === "project" || newTab.kind === "composite")) {
    const prevPath = prevTab?.kind === "project" ? prevTab.project.path :
                     prevTab?.kind === "composite" ? prevTab.project.path : null;
    const newPath = newTab.kind === "project" ? newTab.project.path : newTab.project.path;
    if (prevPath !== newPath) {
      await activateProjectTab(newIdx);
    }
  }
}

export async function switchToPrevTab(): Promise<void> {
  const prevTab = get(activeTab);
  tabsPrev();
  const newTab = get(activeTab);
  const newIdx = get(activeTabIndex);
  if (newTab && (newTab.kind === "project" || newTab.kind === "composite")) {
    const prevPath = prevTab?.kind === "project" ? prevTab.project.path :
                     prevTab?.kind === "composite" ? prevTab.project.path : null;
    const newPath = newTab.kind === "project" ? newTab.project.path : newTab.project.path;
    if (prevPath !== newPath) {
      await activateProjectTab(newIdx);
    }
  }
}

export async function closeActiveTab(): Promise<void> {
  const idx = get(activeTabIndex);
  if (idx < 0) return;

  const tabs = get(openTabs);
  const tab = tabs[idx];

  // For composite tabs, Cmd+W closes the active segment only
  if (tab?.kind === "composite") {
    if (tab.activeSegment === "terminal") {
      await closeTerminalSegment(idx);
    } else {
      // Close project segment: demote to standalone terminal, close project in Rust
      const projectIdx = tabIndexToProjectIndex(idx);
      closeProjectSegment(idx);
      if (projectIdx >= 0) {
        await apiCloseProject(projectIdx);
      }
    }
    return;
  }

  await closeTab(idx);
}
