/**
 * `runMutation` — single façade for every UI-initiated mutation call.
 *
 * Wraps the underlying Tauri `invoke` with the cross-cutting concerns
 * every mutation call site used to hand-roll:
 *
 *   - Success toast (auto-dismiss after 5 s; suppressed for silent-set
 *     ops like stage / unstage / discard).
 *   - Failure toast (sticky — `duration: null`) carrying only the
 *     first line of a multi-line Tauri error.
 *   - Optional task-record lifecycle via {@link taskRunner} for
 *     long-running / output-bearing ops (fetch, pull, push, rebase,
 *     publish).
 *   - Ad-hoc task creation on failure for *non-tracked* mutations so
 *     the "See details" toast action has a row to open in the Tasks
 *     popover.
 *
 * Refresh is **not** driven from here — the mutation-events crate on
 * the Rust side emits a `project-mutated` event after every command
 * and the `mutations.ts` listener fans that out to the right stores.
 * Callers migrated to `runMutation` therefore delete their trailing
 * `refreshStatuses()` / `reloadGraph()` calls.
 *
 * Generics are preserved so the wrapped invoke's return type is
 * visible to the caller — important for commands that return an OID
 * or a `TaskId`.
 */
import { addToast } from "$lib/stores/toast";
import { taskRunner } from "$lib/stores/taskRunner";
import { openTasksPopover } from "$lib/stores/tasksPopover";
import { firstErrorLine, getErrorCode, errorCodeMessage } from "$lib/api/errors";

/** Options passed to {@link runMutation}. */
export interface MutationOpts<T> {
  /** Short snake label used for logging, task title, and ad-hoc task kind. */
  kind: string;
  /** The underlying Tauri invoke (or any async function). */
  invoke: () => Promise<T>;
  /**
   * Optional success-toast renderer. Receives the invoke's result so
   * the copy can interpolate it (commit OID, branch name, count, …).
   * Omit for silent-set ops where the UI already updates visually.
   */
  successToast?: (result: T) => string;
  /** Prefix used when the invoke rejects (e.g. `"Commit failed"`). */
  failureToastPrefix: string;
  /**
   * When true, records the mutation in the Tasks popover via
   * {@link taskRunner.begin} / {@link taskRunner.complete}. Use for
   * long-running or output-bearing ops — the Rust-side `TaskManager`
   * already emits its own entries for those, so this flag is mostly
   * for TS-only mutations (e.g. future headless AI commands).
   */
  trackAsTask?: boolean;
}

/** Run a mutation with the standard toast + task policy. */
export async function runMutation<T>(opts: MutationOpts<T>): Promise<T> {
  const taskId = opts.trackAsTask ? taskRunner.begin(opts.kind) : null;
  try {
    const result = await opts.invoke();
    if (opts.successToast) {
      addToast({
        type: "success",
        message: opts.successToast(result),
        duration: 5000,
      });
    }
    if (taskId) taskRunner.complete(taskId, { ok: true });
    return result;
  } catch (err) {
    // Always resolve an id the "See details" action can open: tracked
    // tasks reuse their `taskId`; non-tracked mutations synthesise an
    // ad-hoc error entry so the failure row is reachable from the
    // Tasks popover even for one-shot ops like stage / commit.
    let detailId: string;
    if (taskId) {
      taskRunner.complete(taskId, { ok: false, err });
      detailId = taskId;
    } else {
      detailId = taskRunner.createAdhoc(opts.kind, err);
    }
    // Prefer a code-keyed label when the error carries a known `code`
    // (structured IpcError); otherwise fall back to the raw first line. The
    // code→message map is deliberately tiny (see errors.ts) — full per-code
    // i18n is deferred to the codegen follow-up.
    const code = getErrorCode(err);
    const detail = (code && errorCodeMessage(code)) || firstErrorLine(err);
    addToast({
      type: "error",
      message: `${opts.failureToastPrefix} — ${detail}`,
      duration: null,
      actions: [
        {
          label: "See details",
          onclick: () => openTasksPopover(detailId),
        },
      ],
    });
    throw err;
  }
}
