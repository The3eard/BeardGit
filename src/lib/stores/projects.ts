/**
 * Projects store — multi-project tab management.
 *
 * Manages opening, closing, and switching between project tabs.
 * Each tab is a separate git repository. Only the active tab has a
 * fully loaded repo/graph/watcher on the Rust side; background tabs
 * keep lightweight metadata only.
 */

import { writable, derived, get } from "svelte/store";
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
import { refreshUserEmails, clearGraphState } from "./graph";
import * as m from "$lib/paraglide/messages";
import { clearBranchState } from "./branches";
import { clearTagState } from "./tags";
import { clearStashState } from "./stashes";
import { clearBlameState } from "./blame";
import { clearWorktreeState } from "./worktrees";
import { clearMrPrState } from "./mr-pr";
import { clearReflogState } from "./reflog";
import { refreshConflictStatus } from "./conflict";

export const openProjects = writable<ProjectInfo[]>([]);
/** Index of the currently active tab (-1 if none). */
export const activeProjectIndex = writable<number>(-1);
export const addMenuOpen = writable(false);

export const activeProject = derived(
  [openProjects, activeProjectIndex],
  ([$openProjects, $activeProjectIndex]) =>
    $activeProjectIndex >= 0 ? $openProjects[$activeProjectIndex] ?? null : null
);

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

  // Find the index of the newly opened project
  const projects = await apiGetOpenProjects();
  const index = projects.findIndex((p) => p.path === path);

  openProjects.set(projects);

  if (index >= 0) {
    await switchProjectTab(index);
  }
}

export async function closeProjectTab(index: number) {
  await apiCloseProject(index);

  const projects = await apiGetOpenProjects();
  openProjects.set(projects);

  if (projects.length === 0) {
    activeProjectIndex.set(-1);
    getCurrentWindow().setTitle("BeardGit");
    return;
  }

  // If the active tab was closed, switch to the new active
  const activeIdx = await apiGetActiveProjectIndex();
  if (activeIdx !== null && activeIdx >= 0) {
    await switchProjectTab(activeIdx);
  }
}

/// Switch to a project tab by index.
///
/// Calls `switch_project` (which fully loads the repo in Rust) then updates
/// the frontend stores directly — avoiding a redundant second `open_repo` call.
export async function switchProjectTab(index: number) {
  stopAllPolling();

  // Clear stale state from previous project
  clearGraphState();
  clearBranchState();
  clearTagState();
  clearStashState();
  clearBlameState();
  clearWorktreeState();
  clearMrPrState();
  clearReflogState();

  isLoading.set(true);
  try {
    const info = await apiSwitchProject(index);
    activeProjectIndex.set(index);

    // switch_project already loaded the repo on the Rust side; set info directly.
    repoInfo.set(info);

    // Show branch name in the native title bar
    // Update title bar: starship-style status
    const projects = await apiGetOpenProjects();
    const proj = projects[index];
    const projName = proj?.name ?? info.path.split("/").pop() ?? "";
    const branch = info.head_branch ?? "detached";
    await updateTitleBar(projName, branch);

    const branchList = await apiGetBranches();
    branches.set(branchList);

    await detectProject();
    await checkProviderStatus();

    // Clear stale CI data from previous project
    ciRuns.set([]);
    selectedCiRun.set(null);
    selectedCiRunId.set(null);
    jobLog.set(null);
    hasMoreCiRuns.set(false);
    loadingDetail.set(false);

    await refreshStatuses();
    await refreshUserEmails();
    await refreshConflictStatus();

    // Re-register the watcher listener for the newly active repo.
    await registerWatcher();
  } finally {
    isLoading.set(false);
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
  // Restore persisted projects from config (lightweight metadata only)
  const projects = await apiRestoreProjects();
  openProjects.set(projects);

  if (projects.length > 0) {
    const activeIdx = await apiGetActiveProjectIndex();
    const idx = activeIdx !== null && activeIdx < projects.length ? activeIdx : 0;
    await switchProjectTab(idx);
  }
}

/** Switch to the next project tab (wraps around). */
export async function switchToNextTab(): Promise<void> {
  const list = get(openProjects);
  const idx = get(activeProjectIndex);
  if (list.length <= 1 || idx < 0) return;
  const next = (idx + 1) % list.length;
  await switchProjectTab(next);
}

/** Switch to the previous project tab (wraps around). */
export async function switchToPrevTab(): Promise<void> {
  const list = get(openProjects);
  const idx = get(activeProjectIndex);
  if (list.length <= 1 || idx < 0) return;
  const prev = (idx - 1 + list.length) % list.length;
  await switchProjectTab(prev);
}

/** Close the currently active tab. */
export async function closeActiveTab(): Promise<void> {
  const idx = get(activeProjectIndex);
  if (idx < 0) return;
  await closeProjectTab(idx);
}
