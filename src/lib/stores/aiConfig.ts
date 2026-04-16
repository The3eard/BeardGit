/**
 * AI Config store — file tree, active editor state, and CRUD actions.
 *
 * Manages the AI config editor view state: which files exist (from
 * ai_get_config_files), which file is open, its content, and dirty state.
 */

import { writable, get } from "svelte/store";
import * as api from "$lib/api/tauri";
import type { AiConfigFile } from "$lib/types";

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

// ─── Actions ───

/** Fetch all AI config files from the backend. */
export async function loadConfigFiles(): Promise<void> {
  configLoading.set(true);
  try {
    const files = await api.aiGetConfigFiles();
    configFiles.set(files);
  } catch {
    configFiles.set([]);
  } finally {
    configLoading.set(false);
  }
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
}
