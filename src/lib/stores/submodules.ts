/**
 * Submodules store — manages submodule list and operations.
 *
 * Fetches the submodule list on demand and after repo-changed events.
 * Init and deinit are synchronous operations that refresh the list.
 * Update operations return TaskIds for background tracking.
 */

import { writable } from "svelte/store";
import type { SubmoduleInfo, TaskId } from "../types";
import {
  listSubmodules as apiList,
  initSubmodule as apiInit,
  updateSubmodule as apiUpdate,
  updateAllSubmodules as apiUpdateAll,
  deinitSubmodule as apiDeinit,
  submoduleAbsPath as apiAbsPath,
} from "../api/tauri";

/** List of submodules in the active repository. */
export const submodules = writable<SubmoduleInfo[]>([]);

/** Whether the submodule list is currently loading. */
export const submodulesLoading = writable(false);

/** Fetch the submodule list from the backend. */
export async function refreshSubmodules() {
  submodulesLoading.set(true);
  try {
    const list = await apiList();
    submodules.set(list);
  } catch {
    submodules.set([]);
  } finally {
    submodulesLoading.set(false);
  }
}

/** Initialize a submodule and refresh the list. */
export async function initSubmodule(path: string): Promise<void> {
  await apiInit(path);
  await refreshSubmodules();
}

/** Update a single submodule (background task). */
export async function updateSubmodule(path: string): Promise<TaskId> {
  return apiUpdate(path);
}

/** Update all submodules (background task). */
export async function updateAllSubmodules(): Promise<TaskId> {
  return apiUpdateAll();
}

/** Deinitialize a submodule and refresh the list. */
export async function deinitSubmodule(path: string, force: boolean): Promise<void> {
  await apiDeinit(path, force);
  await refreshSubmodules();
}

/** Get the absolute path of a submodule for opening in a tab. */
export async function getSubmoduleAbsPath(submodulePath: string): Promise<string> {
  return apiAbsPath(submodulePath);
}
