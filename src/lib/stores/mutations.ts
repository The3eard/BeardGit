/**
 * Mutation event listener + dispatch matrix.
 *
 * One Tauri `project-mutated` event arrives per mutation; this module
 * coalesces per-project flags in a single `requestAnimationFrame`
 * tick and dispatches the minimal refresh set to the downstream
 * stores. Events for inactive projects are buffered until the user
 * switches tabs (see {@link flushPendingForActiveProject}).
 */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { get } from "svelte/store";
import { activeProject, refreshActiveTitleBar } from "./projects";
import { refreshAndReloadGraph } from "./graph";
import { refreshStatuses } from "./changes";
import { refreshStashes } from "./stashes";
import { refreshWorktrees } from "./worktrees";
import { refreshRepoConfig } from "./repoConfig";
import { refreshRemotes } from "./remotes";
import { refreshBranches } from "./branches";
import { saveCurrentSnapshot } from "./project-cache";

/** Shape emitted by `mutation_events::emit_mutation`. */
export interface MutationFlags {
  refs_changed: boolean;
  head_changed: boolean;
  status_changed: boolean;
  stashes_changed: boolean;
  worktrees_changed: boolean;
  remotes_changed: boolean;
}

export interface MutationEvent {
  project_path: string;
  kind: { type: string; source?: string };
  flags: MutationFlags;
}

const pending = new Map<string, MutationFlags>();
let rafScheduled = false;
let unlisten: UnlistenFn | null = null;

function mergeFlags(a: MutationFlags, b: MutationFlags): MutationFlags {
  return {
    refs_changed: a.refs_changed || b.refs_changed,
    head_changed: a.head_changed || b.head_changed,
    status_changed: a.status_changed || b.status_changed,
    stashes_changed: a.stashes_changed || b.stashes_changed,
    worktrees_changed: a.worktrees_changed || b.worktrees_changed,
    remotes_changed: a.remotes_changed || b.remotes_changed,
  };
}

function accumulate(path: string, flags: MutationFlags): void {
  const prev = pending.get(path);
  pending.set(path, prev ? mergeFlags(prev, flags) : flags);
}

/**
 * Dispatch the minimal refresh set implied by `flags` to the stores
 * that care. Exported for {@link flushPendingForActiveProject} and
 * for direct testing.
 *
 * `path` identifies the project the flags belong to and gates the
 * project-cache snapshot save: ahead/behind/staged/etc. live in
 * `ProjectSnapshot` and feed the TabTooltip + window title, both of
 * which would otherwise stay stale until the user switches tabs and
 * back. Saving here covers external mutations (`git push`, CLI commit,
 * stash from terminal) that the watcher pipeline observes but the old
 * `repo-changed` listener used to refresh.
 */
export function dispatchRefresh(flags: MutationFlags, path?: string): void {
  if (flags.refs_changed) {
    // Branch list mirrors `refs/heads/**` and `refs/remotes/**`, so any
    // ref movement (create/delete/rename, fetch/push) needs the
    // sidebar list re-fetched alongside the graph layout. Without
    // this, deleted branches linger as ghost rows until the user
    // manually hits the section's refresh button.
    void refreshAndReloadGraph();
    void refreshBranches();
  }
  if (flags.head_changed || flags.status_changed) {
    void refreshStatuses();
  }
  if (flags.stashes_changed) void refreshStashes();
  if (flags.worktrees_changed) void refreshWorktrees();
  if (flags.remotes_changed) {
    void refreshRepoConfig();
    void refreshRemotes();
  }
  // Persist the per-project snapshot (ahead/behind/staged/etc.) when
  // any flag that maps onto a `ProjectSnapshot` field flipped.
  // `worktrees_changed` and `remotes_changed` don't affect snapshot
  // fields so they're deliberately excluded.
  if (
    path &&
    (flags.refs_changed ||
      flags.head_changed ||
      flags.status_changed ||
      flags.stashes_changed)
  ) {
    void saveCurrentSnapshot(path);
    // The OS window title carries the same ↑/↓/+/!/?/⚑ segment that
    // `saveCurrentSnapshot` rebuilds the snapshot for; keep them in
    // lockstep so both surfaces converge on the same data.
    void refreshActiveTitleBar();
  }
}

function flush(): void {
  rafScheduled = false;
  const active = get(activeProject);
  for (const [path, flags] of Array.from(pending.entries())) {
    if (path === active?.path) {
      dispatchRefresh(flags, path);
      pending.delete(path);
    }
  }
}

function schedule(): void {
  if (rafScheduled) return;
  rafScheduled = true;
  const raf =
    typeof requestAnimationFrame === "function"
      ? requestAnimationFrame
      : (cb: () => void) => setTimeout(cb, 16);
  raf(flush);
}

/** Register the Tauri listener. Idempotent. */
export async function startMutationListener(): Promise<void> {
  if (unlisten) return;
  unlisten = await listen<MutationEvent>("project-mutated", (ev) => {
    accumulate(ev.payload.project_path, ev.payload.flags);
    schedule();
  });
}

/** Flush any buffered flags for `path` — called after a tab switch. */
export function flushPendingForActiveProject(path: string): void {
  const flags = pending.get(path);
  if (flags) {
    dispatchRefresh(flags, path);
    pending.delete(path);
  }
}

/** Unregister the listener. Mostly for teardown symmetry. */
export function stopMutationListener(): void {
  unlisten?.();
  unlisten = null;
  pending.clear();
  rafScheduled = false;
}

/** Test helper — reset module-private state between cases. */
export function __resetForTests(): void {
  unlisten?.();
  unlisten = null;
  pending.clear();
  rafScheduled = false;
}
