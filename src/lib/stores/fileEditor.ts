/**
 * File-editor store — tabs, tree state, and per-project persistence for
 * the in-app mini-editor (PR3).
 *
 * Composition:
 *  - `tabs`         — open buffer list (one per file).
 *  - `activeTabPath` — which tab is currently visible.
 *  - `treeChildren`  — one `list_workdir_tree` result per expanded
 *    directory, keyed by prefix (`""` is the repo root).
 *  - `expandedDirs` / `loadingDirs` / `treeLoading` — tree pane UI state.
 *  - `searchResults` / `searchLoading` — server-side file search, which is
 *    how the filter reaches files no expanded directory contains.
 *
 * All mutations go through `runMutation` so failures surface a sticky
 * toast with the standard "See details" affordance.
 *
 * The live stores hold **one project at a time** — the backend file-IO
 * commands resolve paths against whatever `app-core` considers the active
 * project, so nothing here threads a project handle. Every other open
 * project's editor state (tabs, buffers, tree listings, expanded folders)
 * sits in a session cache keyed by project path; `syncProject` swaps it in
 * and out. localStorage keeps only the tab *paths*, for the next launch.
 */
import { getErrorMessage } from "$lib/api/errors";
import { derived, get, writable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  createWorkdirPath as apiCreatePath,
  deleteWorkdirPath as apiDeletePath,
  listWorkdirTree as apiListTree,
  readWorkdirFile as apiReadFile,
  renameWorkdirPath as apiRenamePath,
  searchWorkdirFiles as apiSearchFiles,
  stageFiles as apiStageFiles,
  writeWorkdirFile as apiWriteFile,
} from "$lib/api/tauri";
import { runMutation } from "$lib/api/runMutation";
import type {
  ReadWorkdirFileResult,
  WorkdirTreeEntry,
} from "$lib/types";
import type { MutationEvent } from "$lib/stores/mutations";

/**
 * Cap on a single directory listing.
 *
 * A guard against one pathological directory, not a budget for the tree.
 * The tree used to ask for the whole working directory at once with a cap
 * of 10,000 — applied to a depth-first walk that stopped wherever it was,
 * so entire folders arrived empty and the ones past the cutoff never
 * arrived at all. Nothing recursive happens on this path any more: opening
 * a folder reads that folder.
 */
export const DIRECTORY_ENTRY_CAP = 5_000;

/** Cap on how many search hits the backend returns for one query. */
export const SEARCH_RESULT_CAP = 300;

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
/**
 * Children of every directory listed so far, keyed by repo-relative
 * prefix. `""` is the repo root, which is loaded on project open; the rest
 * appear as the user expands them.
 */
export const treeChildren = writable<Map<string, WorkdirTreeEntry[]>>(new Map());
/** Prefixes the user has expanded, so the tree can render them open. */
export const expandedDirs = writable<Set<string>>(new Set());
/** Prefixes with a listing in flight — the row shows a spinner. */
export const loadingDirs = writable<Set<string>>(new Set());
/** Prefixes whose last listing failed, so the row can say so. */
export const failedDirs = writable<Set<string>>(new Set());
/** `true` while the root listing is in flight. */
export const treeLoading = writable(false);

/** Matches for the current filter query, or `[]` when not searching. */
export const searchResults = writable<WorkdirTreeEntry[]>([]);
/** `true` while a search is in flight. */
export const searchLoading = writable(false);
/** `true` when the last search came back at {@link SEARCH_RESULT_CAP}. */
export const searchTruncated = writable(false);

/** Flat view of everything listed so far — path → entry. */
export const knownEntries = derived(treeChildren, ($children) => {
  const map = new Map<string, WorkdirTreeEntry>();
  for (const entries of $children.values()) {
    for (const e of entries) map.set(e.path, e);
  }
  return map;
});

function withFlag(set: Set<string>, key: string, on: boolean): Set<string> {
  const next = new Set(set);
  if (on) next.add(key);
  else next.delete(key);
  return next;
}

/**
 * List one directory and store its children. `""` is the repo root.
 *
 * Errors leave the previous children in place and mark the directory in
 * `failedDirs`, so the row can say the listing failed rather than
 * silently reading as an empty folder.
 */
export async function loadDirectory(
  prefix: string,
  respectGitignore: boolean,
): Promise<void> {
  const seq = treeSeq;
  if (prefix === "") treeLoading.set(true);
  loadingDirs.update((s) => withFlag(s, prefix, true));
  try {
    const entries = await apiListTree(
      prefix === "" ? null : prefix,
      DIRECTORY_ENTRY_CAP,
      respectGitignore,
    );
    // The tree may have been reset or refreshed while this was in flight —
    // a project switch, or the gitignore toggle firing a second root load.
    // Writing now would put another repository's paths, or the answer to
    // the opposite question, into the tree the user is looking at.
    if (seq !== treeSeq) return;
    treeChildren.update((map) => new Map(map).set(prefix, entries));
    failedDirs.update((s) => withFlag(s, prefix, false));
  } catch {
    if (seq !== treeSeq) return;
    failedDirs.update((s) => withFlag(s, prefix, true));
  } finally {
    // Unconditionally: this flag records that *this* call is in flight, so
    // a stale answer still has to clear its own. Guarding it left a
    // directory that was collapsed mid-listing marked as loading forever —
    // and since it was no longer expanded, no refresh would ever re-list it
    // and clear the mark.
    loadingDirs.update((s) => withFlag(s, prefix, false));
    if (seq === treeSeq && prefix === "") treeLoading.set(false);
  }
}

/**
 * Expand or collapse a directory, listing it on first expand.
 *
 * Collapsing keeps the children cached: re-opening a folder the user just
 * closed should not go back to the disk. That cache is only valid until
 * the next refresh, which is why {@link refreshTree} drops it rather than
 * skipping over it — see the note there.
 */
export async function toggleDirectory(
  prefix: string,
  respectGitignore: boolean,
): Promise<void> {
  const open = get(expandedDirs).has(prefix);
  expandedDirs.update((s) => withFlag(s, prefix, !open));
  if (!open && !get(treeChildren).has(prefix)) {
    await loadDirectory(prefix, respectGitignore);
  }
}

/**
 * Reload what is on screen and forget the rest.
 *
 * The eviction is the important half. Reloading only the open directories
 * and leaving the collapsed ones cached made content unreachable: create a
 * file inside a folder you had opened and then closed, and the refresh
 * that follows the mutation skips that folder — it is not expanded — while
 * re-expanding it skips the listing, because it is still cached. The file
 * exists, the tree cannot show it, and no button in the UI fixes it.
 * Renaming was worse: the tree kept offering the old name, which opens an
 * error.
 *
 * So a refresh drops every cached listing and re-lists the root plus
 * whatever is currently open. Collapsed folders re-list when the user next
 * opens them. The "don't re-list on collapse and re-open" saving still
 * holds between refreshes, which is where it was worth having.
 */
export async function refreshTree(respectGitignore: boolean): Promise<void> {
  const seq = ++treeSeq;
  const open = [...get(expandedDirs)];
  treeChildren.set(new Map());
  failedDirs.set(new Set());
  await loadDirectory("", respectGitignore);
  // The child listings are launched *after* an await, so without this they
  // would capture whatever `treeSeq` had become and sail through the guard
  // that just discarded this refresh's own root listing. Two refreshes with
  // different `respectGitignore` — the toggle, clicked twice — could then
  // race, and the loser's children could land last.
  if (seq !== treeSeq) return;
  await Promise.all(open.map((p) => loadDirectory(p, respectGitignore)));
}

/**
 * Re-list only the directories a path change can have affected.
 *
 * This is what the CRUD wrappers use instead of {@link refreshTree}. A full
 * refresh empties `treeChildren` before re-listing, and emptying it is a
 * visible blank frame — one the user sees for nothing, because
 * `project-mutated` has usually already refreshed the tree by the time the
 * wrapper's own refresh lands. Re-listing the parent leaves every other
 * directory's cache alone, so there is nothing to blank.
 *
 * `prefixes` are the *parent* directories to re-list; `""` is the root.
 * `removed`, when given, is a path that no longer exists — its cached
 * subtree is dropped, since a partial refresh no longer clears it wholesale.
 * Without that, deleting `src/old/` and later creating a directory of the
 * same name would show the dead listing.
 *
 * Note this does **not** replace the wrappers' need to refresh at all: the
 * `project-mutated` listener filters on `status_changed`, which is blind to
 * a new empty directory, the second file in an untracked directory, and
 * anything gitignored. See `file-editor/CLAUDE.md`.
 */
export async function refreshTreePaths(
  prefixes: string[],
  respectGitignore: boolean,
  removed?: string,
): Promise<void> {
  if (removed !== undefined) {
    const isUnder = (p: string) => p === removed || p.startsWith(`${removed}/`);
    const dropKeys = <T>(map: Map<string, T>) => {
      const next = new Map(map);
      for (const key of map.keys()) if (isUnder(key)) next.delete(key);
      return next;
    };
    const dropFlags = (set: Set<string>) => {
      const next = new Set(set);
      for (const key of set) if (isUnder(key)) next.delete(key);
      return next;
    };
    treeChildren.update(dropKeys);
    expandedDirs.update(dropFlags);
    failedDirs.update(dropFlags);
    // `loadingDirs` is deliberately left alone: an in-flight listing has to
    // clear its own flag in its `finally`, and dropping the key here would
    // leave that write to re-add it with nothing to clear it afterwards.
  }

  const seq = treeSeq;
  // Deduped: a rename within one directory names the same parent twice, and
  // listing it twice would race two writes for the same key.
  await Promise.all(
    [...new Set(prefixes)].map((p) => loadDirectory(p, respectGitignore)),
  );
  // Same guard as `refreshTree`: a project switch or gitignore toggle while
  // these were in flight means the answers belong to a tree that is gone.
  if (seq !== treeSeq) return;
}

/**
 * Parent directory of a repo-relative path, or `""` for a top-level entry.
 *
 * Forward slashes only — paths crossing the IPC boundary are normalised, per
 * `git-engine`'s path contract.
 */
export function parentDir(path: string): string {
  const idx = path.lastIndexOf("/");
  return idx === -1 ? "" : path.slice(0, idx);
}

/**
 * Expand every ancestor of `path` so its row is on screen, listing the
 * ones the tree has not visited yet. Used by the "reveal active file"
 * preference; the scroll itself is the tree view's business.
 */
export async function revealInTree(
  path: string,
  respectGitignore: boolean,
): Promise<void> {
  const ancestors: string[] = [];
  for (let p = parentDir(path); p !== ""; p = parentDir(p)) ancestors.push(p);
  if (ancestors.length === 0) return;
  expandedDirs.update((s) => {
    const next = new Set(s);
    for (const a of ancestors) next.add(a);
    return next;
  });
  const known = get(treeChildren);
  await Promise.all(
    ancestors
      .filter((a) => !known.has(a))
      .map((a) => loadDirectory(a, respectGitignore)),
  );
}

/** Drop all tree state — used when switching to a different project. */
export function resetTree(): void {
  treeSeq++;
  treeChildren.set(new Map());
  expandedDirs.set(new Set());
  loadingDirs.set(new Set());
  failedDirs.set(new Set());
  searchResults.set([]);
  searchTruncated.set(false);
}

/**
 * Monotonic id for tree listings, bumped by every reset and refresh.
 *
 * Same reasoning as {@link searchTree}'s: a listing that was in flight when
 * the user switched project, or when the gitignore toggle fired a second
 * root load, must not land on top of the newer state.
 */
let treeSeq = 0;

/**
 * Monotonic id for search requests. Typing produces overlapping in-flight
 * searches and they do not necessarily come back in order, so a late
 * answer to an earlier query must not overwrite a newer one.
 */
let searchSeq = 0;

/**
 * Search the working directory for `query`, server-side.
 *
 * An empty query clears the results and leaves the tree showing. The old
 * filter ran in the browser over the truncated tree, so a file that was
 * really there but had not survived the walk simply could not be found —
 * and the footer's advice to "refine the filter to see more" asked the
 * backend for nothing at all.
 */
export async function searchTree(
  query: string,
  respectGitignore: boolean,
): Promise<void> {
  const seq = ++searchSeq;
  const trimmed = query.trim();
  if (trimmed === "") {
    searchResults.set([]);
    searchTruncated.set(false);
    searchLoading.set(false);
    return;
  }
  searchLoading.set(true);
  try {
    const hits = await apiSearchFiles(trimmed, SEARCH_RESULT_CAP, respectGitignore);
    if (seq !== searchSeq) return;
    searchResults.set(hits);
    searchTruncated.set(hits.length >= SEARCH_RESULT_CAP);
  } catch {
    if (seq !== searchSeq) return;
    searchResults.set([]);
    // Or the pane reads "No files match." with "narrow the query to see
    // the rest" underneath it.
    searchTruncated.set(false);
  } finally {
    if (seq === searchSeq) searchLoading.set(false);
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
  const message = getErrorMessage(err);
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
  flushBufferEdits();
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
  flushBufferEdits();
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

/** Update a tab's buffer content synchronously. */
export function updateBuffer(path: string, content: string): void {
  tabs.update((list) => {
    const idx = list.findIndex((t) => t.path === path);
    if (idx < 0) return list;
    const next = [...list];
    const disk = next[idx].diskContent;
    next[idx] = {
      ...next[idx],
      bufferContent: content,
      // Length first: almost every edit changes it, and the full compare
      // is O(file) on each keystroke of a large buffer.
      dirty: content.length !== disk.length || content !== disk,
    };
    return next;
  });
}

/**
 * How long typing is allowed to run ahead of the store.
 *
 * Under the threshold where the dirty dot in the tab strip is seen to lag,
 * and long enough that a burst of keys is one store write.
 */
const BUFFER_FLUSH_MS = 100;
let pendingEdit: { path: string; content: string } | null = null;
let pendingEditTimer: ReturnType<typeof setTimeout> | undefined;

/**
 * Coalesce a burst of CodeMirror changes into one store write.
 *
 * `updateBuffer` runs inside CodeMirror's update listener, before the
 * keystroke paints. Writing `tabs` there copies the tab list and re-runs
 * every subscriber — tab strip, toolbar, pane — on each character, which
 * is what made typing feel heavy. The editor owns the live document; the
 * store only needs to catch up before anyone *reads* the buffer, and
 * {@link flushBufferEdits} is called from every such path.
 */
export function updateBufferDebounced(path: string, content: string): void {
  if (pendingEdit && pendingEdit.path !== path) flushBufferEdits();
  pendingEdit = { path, content };
  clearTimeout(pendingEditTimer);
  pendingEditTimer = setTimeout(flushBufferEdits, BUFFER_FLUSH_MS);
}

/** Write any coalesced edit through to the store now. */
export function flushBufferEdits(): void {
  clearTimeout(pendingEditTimer);
  pendingEditTimer = undefined;
  if (!pendingEdit) return;
  const { path, content } = pendingEdit;
  pendingEdit = null;
  updateBuffer(path, content);
}

/**
 * Save the active tab's buffer to disk. When `opts.stage` is true, also
 * stage the file via `stageFiles` so a Save+Stage gesture takes a single
 * round-trip. Both go through `runMutation` so failures surface the
 * standard sticky toast.
 */
export async function saveActive(opts?: { stage?: boolean }): Promise<void> {
  flushBufferEdits();
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
  // The live stores may belong to another project by now — the editor is
  // only synced while its panel is mounted, so two project switches from
  // the graph view leave them holding the project before last. Persisting
  // them under this key would hand one repository's paths to another.
  let payload: { paths: string[]; activePath: string | null };
  if (currentProject === null || currentProject === projectPath) {
    payload = {
      paths: get(tabs).map((t) => t.path),
      activePath: get(activeTabPath),
    };
  } else {
    const cached = sessionCache.get(projectPath);
    if (!cached) return;
    payload = {
      paths: cached.tabs.map((t) => t.path),
      activePath: cached.activeTabPath,
    };
  }
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
  flushBufferEdits();
  tabs.set([]);
  activeTabPath.set(null);
  resetTree();
  treeLoading.set(false);
  searchLoading.set(false);
}

// ---------------------------------------------------------------------------
// Session cache — one editor state per open project
// ---------------------------------------------------------------------------

/** Everything worth keeping about a project's editor while it is not active. */
interface EditorProjectState {
  tabs: EditorTab[];
  activeTabPath: string | null;
  treeChildren: Map<string, WorkdirTreeEntry[]>;
  expandedDirs: Set<string>;
  failedDirs: Set<string>;
}

/**
 * Parked editor state, keyed by project path. RAM only, on purpose: it has
 * to survive leaving the editor view and switching project tabs, not a
 * relaunch — localStorage keeps the tab paths for that.
 */
const sessionCache = new Map<string, EditorProjectState>();

/** Project the live stores currently describe; `null` before the first sync. */
let currentProject: string | null = null;

/** `respectGitignore` the current tree was listed with. */
let currentRespectGitignore: boolean | null = null;

function snapshotCurrent(): EditorProjectState {
  flushBufferEdits();
  return {
    tabs: get(tabs),
    activeTabPath: get(activeTabPath),
    treeChildren: get(treeChildren),
    expandedDirs: get(expandedDirs),
    failedDirs: get(failedDirs),
  };
}

/**
 * Point the live stores at `projectPath`, doing only what has changed.
 *
 * Called by the editor panel whenever it mounts or its inputs change. The
 * panel used to reset the tree on every mount because "which project did I
 * load" lived in the component — so leaving for the graph and coming back
 * collapsed every folder and re-read every tab, dropping unsaved edits.
 * Owning that knowledge here makes a remount for the same project a no-op.
 *
 * - Same project, same gitignore flag: nothing.
 * - Same project, flag flipped: re-list the tree.
 * - Different project: park the current state in the session cache, then
 *   either restore the target's parked state or, first time this session,
 *   list the root and re-hydrate tabs from localStorage.
 *
 * A restored project's listings and clean buffers are re-read in place —
 * the disk may have moved while it was in the background — but nothing is
 * blanked first: the parked state paints, then corrects itself. Dirty
 * buffers are kept as they are; they are the user's work.
 */
export async function syncProject(
  projectPath: string,
  respectGitignore: boolean,
): Promise<void> {
  if (projectPath === currentProject) {
    if (respectGitignore !== currentRespectGitignore) {
      currentRespectGitignore = respectGitignore;
      await refreshTree(respectGitignore);
    }
    return;
  }

  // Tabs opened while the stores were unowned — "Open in editor" from the
  // Changes view before this panel ever mounted, or right after a project
  // switch parked the previous one — belong to the project arriving now.
  // They are re-opened on top of whatever it restores.
  flushBufferEdits();
  const orphans = currentProject === null ? get(tabs) : [];
  const orphanActive = currentProject === null ? get(activeTabPath) : null;

  if (currentProject !== null) {
    sessionCache.set(currentProject, snapshotCurrent());
  }
  currentProject = projectPath;
  currentRespectGitignore = respectGitignore;

  const parked = sessionCache.get(projectPath);
  if (!parked) {
    tabs.set([]);
    activeTabPath.set(null);
    resetTree();
    await Promise.all([
      refreshTree(respectGitignore),
      restoreTabsForProject(projectPath),
    ]);
    await adoptOrphans(orphans, orphanActive);
    return;
  }

  // Listings still in flight belong to the project we just left.
  treeSeq++;
  loadingDirs.set(new Set());
  treeLoading.set(false);
  searchResults.set([]);
  searchTruncated.set(false);
  treeChildren.set(parked.treeChildren);
  expandedDirs.set(parked.expandedDirs);
  failedDirs.set(parked.failedDirs);
  tabs.set(parked.tabs);
  activeTabPath.set(parked.activeTabPath);

  const cleanPaths = parked.tabs
    .filter((t) => !t.dirty && t.status !== "loading")
    .map((t) => t.path);
  await Promise.all([
    refreshTreePaths(["", ...parked.expandedDirs], respectGitignore),
    ...cleanPaths.map(async (p) => {
      try {
        applyReadResult(p, await apiReadFile(p));
      } catch (err) {
        markLoadError(p, err);
      }
    }),
  ]);
  await adoptOrphans(orphans, orphanActive);
}

async function adoptOrphans(
  orphans: EditorTab[],
  orphanActive: string | null,
): Promise<void> {
  if (orphans.length === 0) return;
  for (const o of orphans) await openTab(o.path);
  if (orphanActive) activeTabPath.set(orphanActive);
}

/**
 * Detach the live stores from their project without loading another.
 *
 * Called by the route on every project switch, whether or not the editor
 * is mounted. Without it, two switches made from the graph view leave the
 * stores describing the project before last — and a file opened "in
 * editor" from the new project's Changes lands in the old one's tab list.
 */
export function parkProject(): void {
  if (currentProject === null) return;
  sessionCache.set(currentProject, snapshotCurrent());
  currentProject = null;
  currentRespectGitignore = null;
  treeSeq++;
  tabs.set([]);
  activeTabPath.set(null);
  treeChildren.set(new Map());
  expandedDirs.set(new Set());
  failedDirs.set(new Set());
  loadingDirs.set(new Set());
  treeLoading.set(false);
  searchResults.set([]);
  searchTruncated.set(false);
}

/**
 * Drop a closed project's parked state. Its tab paths are persisted first
 * so re-opening the project later restores them.
 */
export function forgetProject(projectPath: string): void {
  persistTabsForProject(projectPath);
  sessionCache.delete(projectPath);
  if (currentProject === projectPath) {
    currentProject = null;
    currentRespectGitignore = null;
    clearAll();
  }
}

let externalListenerPromise: Promise<UnlistenFn> | null = null;

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
/**
 * How an external change refreshes the tree.
 *
 * A callback rather than a direct `refreshTree(...)` call because the
 * gitignore preference lives in another store, and reaching for it from
 * here would tie the editor store to the settings store for one boolean.
 * `FileEditorPanel` installs it with the value it is already deriving, and
 * clears it on destroy — so when the editor is not mounted this is a no-op
 * rather than a listing nobody is looking at.
 */
let treeRefreshHook: (() => Promise<void>) | null = null;

/** Install (or clear, with `null`) the tree refresh used on external changes. */
export function setTreeRefreshHook(fn: (() => Promise<void>) | null): void {
  treeRefreshHook = fn;
}

/**
 * Called by `mutations.ts` — the single fan-out point for
 * `project-mutated` — when the working tree changed.
 *
 * Deliberately not a second `project-mutated` listener in this module.
 * That is what this started as, and it bypassed the dispatcher's rAF
 * coalescing (so a burst re-listed the tree once per event, blanking the
 * pane each time) and its project scoping (so a mutation in a background
 * tab refreshed the active tab's tree).
 */
export function refreshFileEditorTree(): void {
  if (treeRefreshHook) void treeRefreshHook();
}

export function startFileEditorListeners(): () => void {
  // `??=` is synchronous, so two calls racing before the first `listen()`
  // resolves reuse the SAME promise rather than each registering a listener
  // (the previous guard checked an UnlistenFn only assigned inside `.then`,
  // so the first listener leaked).
  externalListenerPromise ??= listen<MutationEvent>("project-mutated", (event) => {
    if (!event.payload.flags.status_changed) return;
    tabs.update((list) =>
      list.map((t) =>
        t.dirty || t.externalChange ? t : { ...t, externalChange: true },
      ),
    );
  });
  return stopFileEditorListeners;
}

/** Tear down the project-mutated listener (resolving the pending promise). */
function stopFileEditorListeners(): void {
  const pending = externalListenerPromise;
  externalListenerPromise = null;
  void pending?.then((fn) => fn());
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
  await refreshTreePaths([parentDir(path)], respectGitignore);
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
  // Both ends: a move between directories changes two listings. `fromPath`
  // is also passed as `removed` — if it was a directory, its cached subtree
  // is now under the new name and the old keys are dead.
  await refreshTreePaths(
    [parentDir(fromPath), parentDir(toPath)],
    respectGitignore,
    fromPath,
  );
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
  await refreshTreePaths([parentDir(path)], respectGitignore, path);
}

/**
 * Test helper — reset every store this module owns. Called from
 * `beforeEach` blocks in the unit tests so cases stay isolated.
 */
export function __resetForTests(): void {
  clearTimeout(pendingEditTimer);
  pendingEdit = null;
  sessionCache.clear();
  currentProject = null;
  currentRespectGitignore = null;
  tabs.set([]);
  activeTabPath.set(null);
  resetTree();
  treeLoading.set(false);
  searchLoading.set(false);
  stopFileEditorListeners();
}
