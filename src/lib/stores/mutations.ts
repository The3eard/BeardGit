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
import { activeProject } from "./projects";
import { refreshAndReloadGraph } from "./graph";
import { refreshStatuses } from "./changes";
import { refreshStashes } from "./stashes";
import { refreshWorktrees } from "./worktrees";
import { refreshRepoConfig } from "./repoConfig";

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
 */
export function dispatchRefresh(_path: string, flags: MutationFlags): void {
  if (flags.refs_changed) {
    void refreshAndReloadGraph();
  }
  if (flags.head_changed || flags.status_changed) {
    void refreshStatuses();
  }
  if (flags.stashes_changed) void refreshStashes();
  if (flags.worktrees_changed) void refreshWorktrees();
  if (flags.remotes_changed) void refreshRepoConfig();
}

function flush(): void {
  rafScheduled = false;
  const active = get(activeProject);
  for (const [path, flags] of Array.from(pending.entries())) {
    if (path === active?.path) {
      dispatchRefresh(path, flags);
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
    dispatchRefresh(path, flags);
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
