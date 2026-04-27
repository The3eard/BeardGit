/**
 * Thin façade over `tasks.ts` for the `runMutation` wrapper.
 *
 * `runMutation` needs three hooks — begin a tracked task, complete it,
 * and synthesise an ad-hoc task record when a *non-tracked* mutation
 * fails (so the "See details" toast action has something to open in
 * the Tasks popover). Rather than widen `tasks.ts`'s surface with
 * mutation-specific helpers we keep the façade here so the aggregator
 * stays focused on bridging producer events.
 *
 * The `kind` strings accepted by `begin` / `createAdhoc` are free-form
 * snake labels coming from the call site (e.g. `"stage"`, `"commit"`,
 * `"fetch"`). They are normalised to the `git_*` namespace used by
 * {@link TaskKind} so the drawer's icon dispatch keeps working; any
 * string already prefixed with `git_` or `ai_` is passed through.
 */

import { get } from "svelte/store";
import { tasksStore } from "./tasks";
import type { TaskEntry, TaskKind, TaskStatus } from "../types/tasks";

let nextAdhocId = 0;

/** Map a free-form runMutation kind into a valid `TaskKind`. */
function toTaskKind(kind: string): TaskKind {
  if (
    kind.startsWith("git_") ||
    kind.startsWith("ai_") ||
    kind === "app_update"
  ) {
    return kind as TaskKind;
  }
  // Map known long-op kinds to their Rust-side `git_*` equivalents so
  // the popover's icon dispatch keeps working. Everything else falls
  // back to `git_fetch` — a neutral busy icon.
  switch (kind) {
    case "fetch":
      return "git_fetch";
    case "pull":
      return "git_pull";
    case "push":
      return "git_push";
    case "clone":
      return "git_clone";
    default:
      return "git_fetch";
  }
}

/** Upsert a `TaskEntry` into the aggregator store. */
function upsert(entry: TaskEntry): void {
  tasksStore.update((list) => {
    const idx = list.findIndex((t) => t.id === entry.id);
    if (idx >= 0) {
      const copy = list.slice();
      copy[idx] = entry;
      return copy;
    }
    return [...list, entry];
  });
}

export const taskRunner = {
  /**
   * Create a `running` entry keyed by a synthesised id.
   *
   * Returns the id so the caller can pair it with a later
   * {@link taskRunner.complete} call on success or failure.
   */
  begin(kind: string): string {
    const id = `run:${kind}:${Date.now()}:${++nextAdhocId}`;
    upsert({
      id,
      kind: toTaskKind(kind),
      title: kind,
      startedAt: Date.now(),
      status: "running" as TaskStatus,
      actions: [],
    });
    return id;
  },

  /** Mark a tracked task terminal (success or error). */
  complete(id: string, outcome: { ok: boolean; err?: unknown }): void {
    const list = get(tasksStore);
    const found = list.find((t) => t.id === id);
    if (!found) return;
    upsert({
      ...found,
      status: outcome.ok ? "success" : "error",
      finishedAt: Date.now(),
      errorMessage: outcome.err ? String(outcome.err) : undefined,
    });
  },

  /**
   * Synthesise a terminal-failed entry for a mutation that did not
   * request explicit task tracking. Returns the id so callers can
   * link a toast action to the popover row.
   */
  createAdhoc(kind: string, err: unknown): string {
    const id = `adhoc:${kind}:${++nextAdhocId}`;
    upsert({
      id,
      kind: toTaskKind(kind),
      title: kind,
      startedAt: Date.now(),
      finishedAt: Date.now(),
      status: "error" as TaskStatus,
      errorMessage: err instanceof Error ? err.message : String(err),
      actions: [],
    });
    return id;
  },
};
