/**
 * AI Config store — file tree, active editor state, and CRUD actions.
 *
 * Manages the AI config editor view state: which files exist (from
 * ai_get_config_files), which file is open, its content, and dirty state.
 */

import { writable, get } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import * as api from "$lib/api/tauri";
import type { AiConfigFile, AiConfigChangeEvent } from "$lib/types";
import { fetchIntoStore } from "$lib/utils/store-helpers";

// ─── State ───

/** All discovered AI config files (project + user scope). */
export const configFiles = writable<AiConfigFile[]>([]);

/** Path of the currently open file in the editor. */
export const activeFilePath = writable<string | null>(null);

/** Raw content of the currently open file. */
export const activeFileContent = writable<string | null>(null);

/** True when the editor has unsaved changes. */
export const activeFileDirty = writable(false);

/** True while loading the file list. */
export const configLoading = writable(false);

/** True when the active file was modified on disk while the editor has unsaved changes. */
export const configFileChangedOnDisk = writable(false);

// ─── Actions ───

/** Fetch all AI config files from the backend. */
export async function loadConfigFiles(): Promise<void> {
  await fetchIntoStore(configFiles, configLoading, () => api.aiGetConfigFiles(), []);
}

/** Open a file in the editor. Reads content from backend. */
export async function openFile(path: string): Promise<void> {
  const content = await api.aiReadConfigFile(path);
  activeFilePath.set(path);
  activeFileContent.set(content);
  activeFileDirty.set(false);
}

/** Save the current editor content to disk. */
export async function saveFile(content: string): Promise<void> {
  const path = get(activeFilePath);
  if (!path) return;
  await api.aiWriteConfigFile(path, content);
  activeFileContent.set(content);
  activeFileDirty.set(false);
}

/** Create a new config file from template and open it. */
export async function createConfigFile(kind: string, scope: string, name: string): Promise<void> {
  const file = await api.aiCreateConfigFile(kind, scope, name);
  await loadConfigFiles();
  await openFile(file.path);
}

/** Mark editor content as dirty (unsaved changes). */
export function markDirty(): void {
  activeFileDirty.set(true);
}

/** Reset editor state. Called on view switch. */
export function clearConfigState(): void {
  configFiles.set([]);
  activeFilePath.set(null);
  activeFileContent.set(null);
  activeFileDirty.set(false);
  configFileChangedOnDisk.set(false);
}

/** Reload the active file from disk, discarding editor changes. */
export async function reloadActiveFile(): Promise<void> {
  const path = get(activeFilePath);
  if (path) {
    await openFile(path);
    configFileChangedOnDisk.set(false);
  }
}

/** Dismiss the "changed on disk" notification. */
export function dismissDiskChange(): void {
  configFileChangedOnDisk.set(false);
}

// ─── Config File Watcher ───

let unlistenConfigChanged: (() => void) | null = null;

/** Start watching AI config directories. Call on AiConfigEditor mount. */
export async function startConfigWatcher(): Promise<void> {
  // Idempotent: a re-mount without teardown would leak the previous
  // ai-config-changed listener and double-arm the Rust watcher.
  if (unlistenConfigChanged) return;
  await api.aiWatchConfigDirs();

  const unlisten = await listen<AiConfigChangeEvent>("ai-config-changed", (event) => {
    const { path: changedPath } = event.payload;
    const currentPath = get(activeFilePath);

    if (currentPath && changedPath === currentPath) {
      // Active file changed on disk
      const isDirty = get(activeFileDirty);
      if (isDirty) {
        // User has unsaved changes — notify via a store flag
        configFileChangedOnDisk.set(true);
      } else {
        // Editor is clean — auto-reload silently
        openFile(currentPath);
      }
    } else {
      // A different file changed — refresh the file tree
      loadConfigFiles();
    }
  });

  unlistenConfigChanged = unlisten;
}

/** Stop watching AI config directories. Call on AiConfigEditor unmount. */
export async function stopConfigWatcher(): Promise<void> {
  unlistenConfigChanged?.();
  unlistenConfigChanged = null;
  try {
    await api.aiStopConfigWatcher();
  } catch {
    // Ignore — watcher may already be stopped
  }
}
