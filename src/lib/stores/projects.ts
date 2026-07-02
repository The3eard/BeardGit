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
import { getErrorCode, getErrorMessage } from "../api/errors";
import { requestOpenInitRepoDialog } from "./initRepoDialog";
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
  selectedJobId,
  jobLogUnavailable,
  loadingJobLog,
} from "./provider";
import { refreshStatuses, clearChangesState } from "./changes";
import { loadProjectSnapshot, saveCurrentSnapshot, restorePersistedViewport } from "./project-cache";
import { refreshUserEmails, clearGraphState, resetGraphViewScope, cacheViewport, restoreCachedViewport } from "./graph";
import * as m from "$lib/paraglide/messages";
import { clearBranchState, cacheBranchesForProject, restoreCachedBranches } from "./branches";
import { clearTagState } from "./tags";
import { clearStashState } from "./stashes";
import { clearBlameState } from "./blame";
import { clearWorktreeState } from "./worktrees";
import { clearMrPrState } from "./mr-pr";
import { clearIssueState } from "./issues";
import { clearReleaseState } from "./releases";
import { clearReflogState } from "./reflog";
import { refreshConflictStatus } from "./conflict";
import { flushPendingForActiveProject } from "./mutations";
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
  closeSegment,
  closeProjectSegment,
  getCompositeTerminals,
} from "./tabs";
import { writable } from "svelte/store";
import { refreshRemotes } from "./remotes";

// Re-export for backward compatibility with components that read these.
export { openTabs, activeTabIndex };
export { switchSegment, closeSegment, closeProjectSegment };

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

/** Live git summary for the ACTIVE project, shown in the status bar. */
export interface ActiveRepoStatus {
  branch: string;
  ahead: number;
  behind: number;
  staged: number;
  unstaged: number;
  untracked: number;
  stash_count: number;
}

/**
 * Status-bar mirror of the (now hidden) native window title's git
 * segment. Written by the same paths that keep the OS title fresh —
 * `updateTitleBar` (mutation-driven) and the cached-snapshot preview on
 * tab switch — and cleared when a terminal tab or the welcome screen
 * is active.
 */
export const activeRepoStatus = writable<ActiveRepoStatus | null>(null);

/**
 * Callback invoked whenever we switch to a different project tab.
 * +page.svelte uses this to reset activeView to "graph" for instant UX.
 */
let projectSwitchCallback: (() => void) | null = null;

/** Register a callback to run on project tab switch. */
export function onProjectSwitch(cb: () => void): void {
  projectSwitchCallback = cb;
}

/**
 * Refresh the OS window title for the *active* project tab.
 *
 * Watcher-driven: `mutations.ts:dispatchRefresh` calls this whenever a
 * mutation flag implies the title's status segment may have moved
 * (`refs_changed | head_changed | status_changed | stashes_changed`).
 * No-op when the active tab isn't a project (terminal tabs own their
 * own title via `switchToTab`).
 */
export async function refreshActiveTitleBar() {
  const tab = get(openTabs)[get(activeTabIndex)];
  if (!tab || (tab.kind !== "project" && tab.kind !== "composite")) return;
  const projName = tab.project.name;
  const branch = get(repoInfo)?.head_branch ?? "detached";
  await updateTitleBar(projName, branch);
}

/** Build a starship-style title: project - branch [↑2 ↓1 +3 !2 ?1 ⚑1] */
async function updateTitleBar(projName: string, branch: string) {
  try {
    const s = await apiGetStatusSummary();
    activeRepoStatus.set({
      branch,
      ahead: s.ahead,
      behind: s.behind,
      staged: s.staged,
      unstaged: s.unstaged,
      untracked: s.untracked,
      stash_count: s.stash_count,
    });
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
    activeRepoStatus.set({
      branch,
      ahead: 0,
      behind: 0,
      staged: 0,
      unstaged: 0,
      untracked: 0,
      stash_count: 0,
    });
    getCurrentWindow().setTitle(`${projName} - ${branch}`);
  }
}

export async function openProjectTab(path: string) {
  let info: ProjectInfo;
  try {
    info = await apiOpenProject(path);
  } catch (err) {
    // `open_project` rejects with an IpcError; `not_a_repo` carries the
    // attempted path in `message` so we can seed the init dialog with it.
    if (getErrorCode(err) === "not_a_repo") {
      requestOpenInitRepoDialog(getErrorMessage(err));
      return;
    }
    throw err;
  }

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

  // Cache outgoing project's graph + branch list BEFORE changing
  // activeTabIndex (activeProjectFromTab derives from activeTabIndex,
  // so must read first). The branch cache mirrors the viewport
  // pattern: on the next switch back to this project we paint the
  // last-seen list immediately and reconcile when the fresh
  // `apiBranches` IPC resolves, instead of flashing an empty list.
  const prevProject = get(activeProjectFromTab);
  if (prevProject) {
    cacheViewport(prevProject.path);
    cacheBranchesForProject(prevProject.path);
  }

  // Set active index immediately for instant tab highlight
  activeTabIndex.set(tabIndex);

  if (tab.kind === "project" || tab.kind === "composite") {
    if (tabIndex !== prevIdx) {
      projectSwitchCallback?.();
    }
    await activateProjectTab(tabIndex);
  } else if (tab.kind === "terminal") {
    // Terminal tab — update title bar; no repo context in the status bar.
    activeRepoStatus.set(null);
    getCurrentWindow().setTitle(`${tab.terminal.title} — BeardGit`);
  }
}

/** Full project activation: Rust switch + state refresh. */
async function activateProjectTab(tabIndex: number) {
  const projectIdx = tabIndexToProjectIndex(tabIndex);
  if (projectIdx < 0) return;

  stopAllPolling();
  clearGraphState();
    resetGraphViewScope();
  clearBranchState();
  clearTagState();
  clearStashState();
  clearBlameState();
  clearWorktreeState();
  clearMrPrState();
  clearIssueState();
  clearReleaseState();
  clearReflogState();
  clearChangesState();

  // Restore cached graph viewport instantly (no loading spinner for graph).
  //
  // Two-tier lookup: the in-memory tab cache is populated on
  // `cacheViewport` (outgoing tab) and holds the full lane geometry;
  // the disk-backed slice in `ProjectSnapshot.graph_viewport_cache` is
  // populated by `saveCurrentSnapshot` and survives app restarts. When
  // the tab cache misses we fall through to the persisted slice so
  // cold starts still paint the graph synchronously (Phase 8).
  const tabs = get(openTabs);
  const targetTab = tabs[tabIndex];
  const targetPath = (targetTab?.kind === "project" || targetTab?.kind === "composite") ? targetTab.project.path : null;
  // Restore branch list from cache so the panel paints instantly on
  // tab switch. Falls back to clearing when no cache entry exists
  // (cold tab), keeping the existing empty-and-loading UX. The fresh
  // list lands later when `refreshBranches` resolves below.
  if (targetPath !== null) restoreCachedBranches(targetPath);
  const hasCachedGraph = targetPath
    ? (restoreCachedViewport(targetPath) || restorePersistedViewport(targetPath))
    : false;

  // (Buffered mutation-event flags get replayed AFTER apiSwitchProject
  //  resolves — see below. Flushing here would call saveCurrentSnapshot
  //  with the previous project's repoInfo + getStatusSummary still
  //  loaded, writing wrong data under the new project's key.)

  // Set clean titlebar immediately (no stale counts from previous project)
  const projName = (targetTab?.kind === "project" || targetTab?.kind === "composite")
    ? targetTab.project.name
    : "";
  activeRepoStatus.set(null);
  getCurrentWindow().setTitle(`${projName} — BeardGit`);

  // Load cached snapshot for instant titlebar with real data
  if (targetPath) {
    loadProjectSnapshot(targetPath).then((snapshot) => {
      if (snapshot) {
        const branch = snapshot.head_branch ?? "detached";
        activeRepoStatus.set({
          branch,
          ahead: snapshot.ahead,
          behind: snapshot.behind,
          staged: snapshot.staged,
          unstaged: snapshot.unstaged,
          untracked: snapshot.untracked,
          stash_count: snapshot.stash_count,
        });
        const parts: string[] = [];
        if (snapshot.ahead > 0) parts.push(`↑${snapshot.ahead}`);
        if (snapshot.behind > 0) parts.push(`↓${snapshot.behind}`);
        if (snapshot.staged > 0) parts.push(`+${snapshot.staged}`);
        if (snapshot.unstaged > 0) parts.push(`!${snapshot.unstaged}`);
        if (snapshot.untracked > 0) parts.push(`?${snapshot.untracked}`);
        if (snapshot.stash_count > 0) parts.push(`⚑${snapshot.stash_count}`);
        const status = parts.length > 0 ? ` [${parts.join(" ")}]` : "";
        getCurrentWindow().setTitle(`${projName} - ${branch}${status}`);
      }
    });
  }

  // Only show loading spinner if we have no cached graph to display
  if (!hasCachedGraph) {
    isLoading.set(true);
  }
  try {
    const info = await apiSwitchProject(projectIdx);
    repoInfo.set(info);

    // Remotes feed `projectProvider` (status-bar forge pill, provider
    // heuristics) — refresh on activation, since the mutation pipeline
    // only updates them on `remotes_changed`.
    void refreshRemotes();

    // Replay any mutation-event flags buffered for this project while
    // it was in the background. Must run AFTER repoInfo.set so the
    // saveCurrentSnapshot inside the dispatch reads the new project's
    // data, not the previous one's.
    if (targetPath) flushPendingForActiveProject(targetPath);

    // Title bar — fire-and-forget with real data
    const realProjName = (targetTab?.kind === "project" || targetTab?.kind === "composite")
      ? targetTab.project.name
      : info.path.split("/").pop() ?? "";
    const branch = info.head_branch ?? "detached";
    updateTitleBar(realProjName, branch);

    // Independent fetches in parallel
    const [projects, branchList] = await Promise.all([
      apiGetOpenProjects(),
      apiGetBranches(),
    ]);
    syncProjectTabs(projects);
    branches.set(branchList);

    // Reset CI state
    ciRuns.set([]);
    selectedCiRun.set(null);
    selectedCiRunId.set(null);
    jobLog.set(null);
    hasMoreCiRuns.set(false);
    loadingDetail.set(false);
    // Job-detail state also lives in the provider store — reset it too, or a
    // stale selectedJobId from the previous project leaves the job-steps view
    // blank with no signal.
    selectedJobId.set(null);
    jobLogUnavailable.set(false);
    loadingJobLog.set(false);

    // Provider + status refreshes in parallel
    await Promise.all([
      detectProject().then(() => checkProviderStatus()),
      refreshStatuses(),
      refreshUserEmails(),
      refreshConflictStatus(),
      registerWatcher(),
    ]);

    // Persist snapshot for next switch
    if (targetPath) {
      saveCurrentSnapshot(targetPath);
    }
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
    // Close all terminal segments first (demotes to project), then close the project
    const terminals = getCompositeTerminals(tab);
    for (const t of terminals) {
      await closeTerminalTab(t.sessionId);
    }
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
    stopAllPolling();
    clearGraphState();
    resetGraphViewScope();
    clearBranchState();
    clearTagState();
    clearStashState();
    clearBlameState();
    clearWorktreeState();
    clearMrPrState();
    clearIssueState();
    clearReleaseState();
    clearReflogState();
    clearChangesState();
    repoInfo.set(null);
    branches.set([]);
    activeRepoStatus.set(null);
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

    // Pre-warm the snapshot cache for the about-to-activate project so
    // the first `switchToTab` can hit `restorePersistedViewport`
    // synchronously and skip the skeleton paint on cold start (Phase 8).
    const targetProject = projects[rustIdx];
    if (targetProject) {
      await loadProjectSnapshot(targetProject.path);
    }

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
    if (tab.activeSegmentIndex >= 0) {
      // Close the active linked segment
      await closeSegment(idx, tab.activeSegmentIndex);
    } else {
      // Project segment active: demote to standalone terminal(s), close project in Rust
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
