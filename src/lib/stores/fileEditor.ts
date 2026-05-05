/**
 * File-editor store — tabs, tree state, and per-project persistence for
 * the in-app mini-editor (PR3).
 *
 * Composition:
 *  - `tabs`         — open buffer list (one per file).
 *  - `activeTabPath` — which tab is currently visible.
 *  - `treeEntries`   — last `list_workdir_tree` result for the file tree.
 *  - `treeLoading` / `treeTruncated` — UI flags for the tree pane.
 *
 * All mutations go through `runMutation` so failures surface a sticky
 * toast with the standard "See details" affordance.
 *
 * The store is **single-project scoped**: callers persist the open-tabs
 * list to localStorage on project switch and re-hydrate it after the
 * new project loads. The backend file-IO commands resolve paths against
 * whatever `app-core` considers the active project, so the store never
 * needs to thread a project handle.
 */
import { get, writable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  createWorkdirPath as apiCreatePath,
  deleteWorkdirPath as apiDeletePath,
  listWorkdirTree as apiListTree,
  readWorkdirFile as apiReadFile,
  renameWorkdirPath as apiRenamePath,
  stageFiles as apiStageFiles,
  writeWorkdirFile as apiWriteFile,
} from "$lib/api/tauri";
import { runMutation } from "$lib/api/runMutation";
import type {
  ReadWorkdirFileResult,
  WorkdirTreeEntry,
} from "$lib/types";
import type { MutationEvent } from "$lib/stores/mutations";

/** Maximum entries we ask the backend to return per tree refresh. */
export const TREE_ENTRY_CAP = 10_000;

/** localStorage key prefix used for per-project tab persistence. */
const STORAGE_PREFIX = "beardgit:editor-tabs:";

/** One open buffer in the editor panel. */
export interface EditorTab {
  /** Repo-relative, forward-slashed path. */
  path: string;
  /** Final segment of `path` — used in the tab label. */
  name: string;
  /** Last content read from disk (baseline for the dirty diff). */
  diskContent: string;
  /** Current buffer content the editor is displaying. */
  bufferContent: string;
  /** True when `bufferContent !== diskContent`. */
  dirty: boolean;
  /**
   * True when a watcher event implies the on-disk version changed since
   * we last read it. The user is given "Reload" / "Keep my version"
   * buttons in the toolbar; we never auto-reload.
   */
  externalChange: boolean;
  /** Loading / load-error state. */
  status: "loading" | "ok" | "binary" | "too_large" | "error";
  /** Bytes — meaningful only when `status === "binary" | "too_large"`. */
  size?: number;
  /** Error message when `status === "error"`. */
  error?: string;
  /**
   * Monotonically increasing counter bumped every time `bufferContent`
   * is replaced from outside the editor (initial load, reload, save —
   * never on user typing). The `EditorPane` threads it through to
   * `CodeEditor` as `revisionId`, which is the only signal the editor
   * uses to swallow a fresh `content` value into its `EditorState`.
   * This keeps typing decoupled from prop reactivity: the buffer can
   * update wildly on every keystroke without the editor ever feeling
   * a remount.
   */
  loadVersion: number;
}

/** Open editor tabs in the order they should be rendered. */
export const tabs = writable<EditorTab[]>([]);
/** Path of the currently active tab, or `null` when no tab is open. */
export const activeTabPath = writable<string | null>(null);
/** Last `list_workdir_tree` result — `[]` until the first refresh. */
export const treeEntries = writable<WorkdirTreeEntry[]>([]);
/** `true` while a tree refresh is in flight. */
export const treeLoading = writable(false);
/** `true` when the last tree result hit the entry cap. */
export const treeTruncated = writable(false);

/**
 * Refresh the file tree for the active project, respecting the user's
 * gitignore preference. Errors are non-fatal — the store keeps the prior
 * entries and clears the loading flag.
 */
export async function refreshTree(respectGitignore: boolean): Promise<void> {
  treeLoading.set(true);
  try {
    const result = await apiListTree(null, TREE_ENTRY_CAP, respectGitignore);
    treeEntries.set(result);
    treeTruncated.set(result.length >= TREE_ENTRY_CAP);
  } catch {
    // Leave existing entries in place; the toolbar reload button is
    // always available so the user can retry manually.
  } finally {
    treeLoading.set(false);
  }
}

/** Apply a `ReadWorkdirFileResult` onto an existing tab in `tabs`. */
function applyReadResult(
  path: string,
  result: ReadWorkdirFileResult,
): void {
  tabs.update((list) => {
    const idx = list.findIndex((t) => t.path === path);
    if (idx < 0) return list;
    const next = [...list];
    const prev = next[idx];
    const bumpedVersion = prev.loadVersion + 1;
    if (result.kind === "text") {
      next[idx] = {
        ...prev,
        diskContent: result.data,
        bufferContent: result.data,
        dirty: false,
        externalChange: false,
        status: "ok",
        size: result.size,
        error: undefined,
        loadVersion: bumpedVersion,
      };
    } else if (result.kind === "binary") {
      next[idx] = {
        ...prev,
        diskContent: "",
        bufferContent: "",
        dirty: false,
        externalChange: false,
        status: "binary",
        size: result.size,
        error: undefined,
        loadVersion: bumpedVersion,
      };
    } else {
      next[idx] = {
        ...prev,
        diskContent: "",
        bufferContent: "",
        dirty: false,
        externalChange: false,
        status: "too_large",
        size: result.size,
        error: undefined,
        loadVersion: bumpedVersion,
      };
    }
    return next;
  });
}

/** Mark a single tab's `status` / `error` after a load failure. */
function markLoadError(path: string, err: unknown): void {
  const message = err instanceof Error ? err.message : String(err);
  tabs.update((list) => {
    const idx = list.findIndex((t) => t.path === path);
    if (idx < 0) return list;
    const next = [...list];
    next[idx] = { ...next[idx], status: "error", error: message };
    return next;
  });
}

function basename(path: string): string {
  const idx = path.lastIndexOf("/");
  return idx >= 0 ? path.slice(idx + 1) : path;
}

/**
 * Open a file in a new tab (or focus an existing tab). The actual read
 * is performed asynchronously; the tab is added immediately with
 * `status: "loading"` so the UI can show a placeholder.
 */
export async function openTab(path: string): Promise<void> {
  const existing = get(tabs).find((t) => t.path === path);
  if (existing) {
    activeTabPath.set(path);
    if (existing.status === "loading") {
      // Already in flight from a previous call.
      return;
    }
    return;
  }

  const placeholder: EditorTab = {
    path,
    name: basename(path),
    diskContent: "",
    bufferContent: "",
    dirty: false,
    externalChange: false,
    status: "loading",
    loadVersion: 0,
  };
  tabs.update((list) => [...list, placeholder]);
  activeTabPath.set(path);

  try {
    const result = await apiReadFile(path);
    applyReadResult(path, result);
  } catch (err) {
    markLoadError(path, err);
  }
}

/** Switch which tab is focused. No-op when `path` isn't in `tabs`. */
export function setActiveTab(path: string): void {
  const exists = get(tabs).some((t) => t.path === path);
  if (!exists) return;
  activeTabPath.set(path);
  // If the tab was flagged as externally changed, reload it so the
  // editor doesn't keep showing the prior disk content.
  const tab = get(tabs).find((t) => t.path === path);
  if (tab && tab.externalChange && !tab.dirty && tab.status === "ok") {
    void reloadActive();
  }
}

/**
 * Close a tab. When the tab has unsaved changes, the caller is
 * expected to confirm beforehand — this function unconditionally
 * removes the entry from the store. The next visible tab (or `null`
 * when none remain) becomes the active one.
 */
export async function closeTab(path: string): Promise<void> {
  const list = get(tabs);
  const idx = list.findIndex((t) => t.path === path);
  if (idx < 0) return;
  const next = list.filter((t) => t.path !== path);
  tabs.set(next);
  if (get(activeTabPath) === path) {
    if (next.length === 0) {
      activeTabPath.set(null);
    } else {
      // Prefer the tab that was at the same index after splice; fall back
      // to the previous one when we just closed the last tab.
      const target = next[Math.min(idx, next.length - 1)];
      activeTabPath.set(target.path);
    }
  }
}

/** Update a tab's buffer content (called on every CodeMirror change). */
export function updateBuffer(path: string, content: string): void {
  tabs.update((list) => {
    const idx = list.findIndex((t) => t.path === path);
    if (idx < 0) return list;
    const next = [...list];
    next[idx] = {
      ...next[idx],
      bufferContent: content,
      dirty: content !== next[idx].diskContent,
    };
    return next;
  });
}

/**
 * Save the active tab's buffer to disk. When `opts.stage` is true, also
 * stage the file via `stageFiles` so a Save+Stage gesture takes a single
 * round-trip. Both go through `runMutation` so failures surface the
 * standard sticky toast.
 */
export async function saveActive(opts?: { stage?: boolean }): Promise<void> {
  const path = get(activeTabPath);
  if (!path) return;
  const tab = get(tabs).find((t) => t.path === path);
  if (!tab || tab.status !== "ok") return;
  const content = tab.bufferContent;
  const stage = opts?.stage === true;

  await runMutation({
    kind: stage ? "editor_save_and_stage" : "editor_save",
    invoke: async () => {
      await apiWriteFile(path, content);
      if (stage) await apiStageFiles([path]);
    },
    failureToastPrefix: stage ? "Save+Stage failed" : "Save failed",
  });

  tabs.update((list) => {
    const idx = list.findIndex((t) => t.path === path);
    if (idx < 0) return list;
    const next = [...list];
    next[idx] = {
      ...next[idx],
      diskContent: content,
      dirty: false,
      externalChange: false,
    };
    return next;
  });
}

/**
 * Re-read the active tab from disk and replace its buffer. Discards any
 * unsaved edits; callers that care about that warn the user beforehand.
 */
export async function reloadActive(): Promise<void> {
  const path = get(activeTabPath);
  if (!path) return;
  try {
    const result = await apiReadFile(path);
    applyReadResult(path, result);
  } catch (err) {
    markLoadError(path, err);
  }
}

/**
 * Clear the external-change flag on the active tab without re-reading.
 * Used by the "Keep my version" toolbar action.
 */
export function clearExternalChange(path: string): void {
  tabs.update((list) => {
    const idx = list.findIndex((t) => t.path === path);
    if (idx < 0) return list;
    const next = [...list];
    next[idx] = { ...next[idx], externalChange: false };
    return next;
  });
}

/**
 * Update the path of an open tab after a rename — keeps the buffer and
 * dirty state intact so renaming a file the user is editing doesn't
 * lose their in-flight edits.
 */
export function renameOpenTab(fromPath: string, toPath: string): void {
  tabs.update((list) => {
    const idx = list.findIndex((t) => t.path === fromPath);
    if (idx < 0) return list;
    const next = [...list];
    next[idx] = { ...next[idx], path: toPath, name: basename(toPath) };
    return next;
  });
  if (get(activeTabPath) === fromPath) {
    activeTabPath.set(toPath);
  }
}

/**
 * Remove every open tab whose path falls under `prefix` (or matches it).
 * Used after a delete: if the deleted entry was a directory, every file
 * we had open below it must close.
 */
export function closeTabsUnder(prefix: string): void {
  const list = get(tabs);
  const isUnder = (p: string) => p === prefix || p.startsWith(`${prefix}/`);
  const next = list.filter((t) => !isUnder(t.path));
  if (next.length === list.length) return;
  tabs.set(next);
  const active = get(activeTabPath);
  if (active && isUnder(active)) {
    activeTabPath.set(next.length > 0 ? next[0].path : null);
  }
}

/**
 * Persist the open-tabs list (paths + active path only) for `projectPath`
 * so re-opening the project later restores the same set of tabs.
 *
 * Buffer content is intentionally not persisted — the tab is re-read
 * from disk on restore, which is the correct behaviour: external tools
 * may have edited the file in between sessions.
 */
export function persistTabsForProject(projectPath: string): void {
  if (typeof localStorage === "undefined") return;
  const list = get(tabs);
  const payload = {
    paths: list.map((t) => t.path),
    activePath: get(activeTabPath),
  };
  try {
    localStorage.setItem(
      STORAGE_PREFIX + projectPath,
      JSON.stringify(payload),
    );
  } catch {
    // Quota / private mode — drop persistence silently.
  }
}

/**
 * Restore tabs from localStorage for `projectPath`. Triggers an async
 * read for each path; tabs that fail to read end up with `status: "error"`.
 */
export async function restoreTabsForProject(
  projectPath: string,
): Promise<void> {
  if (typeof localStorage === "undefined") return;
  let raw: string | null;
  try {
    raw = localStorage.getItem(STORAGE_PREFIX + projectPath);
  } catch {
    raw = null;
  }
  if (!raw) {
    tabs.set([]);
    activeTabPath.set(null);
    return;
  }
  let parsed: { paths?: unknown; activePath?: unknown };
  try {
    parsed = JSON.parse(raw);
  } catch {
    tabs.set([]);
    activeTabPath.set(null);
    return;
  }
  const paths = Array.isArray(parsed.paths)
    ? parsed.paths.filter((p): p is string => typeof p === "string")
    : [];
  const activePath =
    typeof parsed.activePath === "string" ? parsed.activePath : null;

  tabs.set(
    paths.map((p) => ({
      path: p,
      name: basename(p),
      diskContent: "",
      bufferContent: "",
      dirty: false,
      externalChange: false,
      status: "loading" as const,
      loadVersion: 0,
    })),
  );
  activeTabPath.set(
    activePath && paths.includes(activePath)
      ? activePath
      : paths[0] ?? null,
  );

  // Read each tab's content sequentially so the active tab paints first.
  for (const p of paths) {
    try {
      const result = await apiReadFile(p);
      applyReadResult(p, result);
    } catch (err) {
      markLoadError(p, err);
    }
  }
}

/** Reset the in-memory tab list and tree — called on project teardown. */
export function clearAll(): void {
  tabs.set([]);
  activeTabPath.set(null);
  treeEntries.set([]);
  treeTruncated.set(false);
  treeLoading.set(false);
}

let externalListenerUnlisten: UnlistenFn | null = null;

/**
 * Subscribe to `project-mutated` so external file edits flag every
 * non-dirty open tab as `externalChange: true`. Returns a teardown
 * function. Idempotent — repeated calls reuse the existing listener.
 *
 * We over-mark on purpose: the watcher event doesn't carry per-file
 * paths, so any `status_changed` flag re-flags every tab. The user
 * either clicks "Reload" / activates the tab (which lazily re-reads)
 * or "Keep my version" (which just clears the flag).
 */
export function startFileEditorListeners(): () => void {
  if (externalListenerUnlisten) {
    return () => {
      externalListenerUnlisten?.();
      externalListenerUnlisten = null;
    };
  }
  void listen<MutationEvent>("project-mutated", (event) => {
    if (!event.payload.flags.status_changed) return;
    tabs.update((list) =>
      list.map((t) =>
        t.dirty || t.externalChange ? t : { ...t, externalChange: true },
      ),
    );
  }).then((fn) => {
    externalListenerUnlisten = fn;
  });
  return () => {
    externalListenerUnlisten?.();
    externalListenerUnlisten = null;
  };
}

/**
 * Wrapper around `createWorkdirPath` that also refreshes the tree and,
 * for non-directory creates, opens the new file in a tab.
 */
export async function createPath(
  path: string,
  isDirectory: boolean,
  respectGitignore: boolean,
): Promise<void> {
  await runMutation({
    kind: "editor_create_path",
    invoke: () => apiCreatePath(path, isDirectory),
    failureToastPrefix: isDirectory ? "Create folder failed" : "Create file failed",
  });
  await refreshTree(respectGitignore);
  if (!isDirectory) {
    await openTab(path);
  }
}

/**
 * Wrapper around `renameWorkdirPath` that updates any open tab to keep
 * its buffer alive and refreshes the tree.
 */
export async function renamePath(
  fromPath: string,
  toPath: string,
  respectGitignore: boolean,
): Promise<void> {
  await runMutation({
    kind: "editor_rename_path",
    invoke: () => apiRenamePath(fromPath, toPath),
    failureToastPrefix: "Rename failed",
  });
  renameOpenTab(fromPath, toPath);
  await refreshTree(respectGitignore);
}

/**
 * Wrapper around `deleteWorkdirPath` that also closes any open tab
 * under the deleted path (single file or whole subtree) and refreshes
 * the tree.
 */
export async function deletePath(
  path: string,
  respectGitignore: boolean,
): Promise<void> {
  await runMutation({
    kind: "editor_delete_path",
    invoke: () => apiDeletePath(path),
    failureToastPrefix: "Delete failed",
  });
  closeTabsUnder(path);
  await refreshTree(respectGitignore);
}

/**
 * Test helper — reset every store this module owns. Called from
 * `beforeEach` blocks in the unit tests so cases stay isolated.
 */
export function __resetForTests(): void {
  tabs.set([]);
  activeTabPath.set(null);
  treeEntries.set([]);
  treeLoading.set(false);
  treeTruncated.set(false);
  externalListenerUnlisten?.();
  externalListenerUnlisten = null;
}
