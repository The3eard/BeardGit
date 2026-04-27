/**
 * Shared task descriptor consumed by the unified tasks drawer.
 *
 * Produced by three independent bridges in `src/lib/stores/tasks.ts`:
 *
 * 1. Rust `TaskManager` events — streamed over the `task://update` Tauri
 *    channel from `crates/app-core/src/task_events.rs`. The wire payload
 *    (`TaskEvent` on the Rust side) mirrors this interface byte-for-byte
 *    after camelCase remapping (see the aggregator store for the mapper).
 * 2. The `aiBackgroundRuns` store — headless AI runs each projected
 *    through `aiSessionToTaskEntry`.
 * 3. The `autoUpdate` store's `updateTask` derived — the in-app updater's
 *    lifecycle, mapped to a single stable entry with id `"auto-update"`.
 *
 * Kept deliberately narrow so every producer can map its feature-specific
 * state into one stable shape without pulling in feature-specific types.
 */

/**
 * Category of long-running operation the tasks drawer understands.
 *
 * The literals match the snake_case serialization of
 * [`task_events::TaskKind`](../../../crates/app-core/src/task_events.rs) on
 * the Rust side — adding a new kind requires changes in both modules plus
 * the aggregator's icon dispatch.
 */
export type TaskKind =
  | "ai_background"
  | "ai_interactive"
  | "git_fetch"
  | "git_pull"
  | "git_push"
  | "git_clone"
  | "app_update";

/**
 * Lifecycle phase of a task as surfaced to the drawer.
 *
 * Maps 1:1 to `task_events::TaskStatus` — `"success"` represents both
 * zero-exit-code completion and "happy-path finished" semantics for
 * non-exit-coded producers (AI runs, auto-update).
 */
export type TaskStatus = "running" | "success" | "error" | "cancelled";

/**
 * Optional progress metadata attached to a task entry.
 *
 * Producers that only know "I'm working" emit `determinate: false` and
 * leave the numeric fields absent; progress-aware producers fill in what
 * they can (bytes/count for `current`/`total`, 0..100 for `percent`).
 */
export interface TaskProgress {
  /** `true` when `total` is known and `percent` is meaningful. */
  determinate: boolean;
  /** Units already processed (bytes, objects, chunks, …). */
  current?: number;
  /** Total units expected. */
  total?: number;
  /** Percent complete (0..100). */
  percent?: number;
}

/**
 * Declarative action a row can surface for the user.
 *
 * The row component renders one button per action; the drawer's primary
 * keyboard action dispatches the first entry in the list.
 */
export interface TaskAction {
  /** Stable identifier dispatched back to the store. */
  id: "cancel" | "retry" | "dismiss" | "open_output";
  /** Localized label. */
  label: string;
  /** Optional styling hint for the button. */
  variant?: "primary" | "danger" | "neutral";
}

/**
 * Snapshot of a background task displayed by the unified tasks drawer.
 *
 * Producers project their feature-specific state into this shape and the
 * aggregator store coalesces updates by {@link id} into a single flat
 * feed. Entries with `finishedAt` older than 5 minutes are retired from
 * the drawer automatically.
 */
export interface TaskEntry {
  /** Stable identifier (e.g. AI session id, Rust `TaskId`, `"auto-update"`). */
  id: string;
  /** Producer category — drives icon + action dispatch. */
  kind: TaskKind;
  /** Human-readable, translated title. */
  title: string;
  /** Optional muted one-line context (repo / remote / session id). */
  subtitle?: string;
  /** Wall-clock ms since epoch when the task started running. */
  startedAt: number;
  /** Wall-clock ms since epoch when the task transitioned to a terminal state. */
  finishedAt?: number;
  /** Current lifecycle phase. */
  status: TaskStatus;
  /** Optional progress snapshot. */
  progress?: TaskProgress;
  /** Error message when `status === "error"`. Absent otherwise. */
  errorMessage?: string;
  /** Actions the row should render, already ordered by priority. */
  actions: TaskAction[];
}
