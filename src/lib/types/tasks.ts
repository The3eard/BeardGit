/**
 * Shared task descriptor consumed by the unified tasks drawer (cluster 0.3)
 * and produced by feature stores such as `autoUpdate.ts`.
 *
 * Kept deliberately minimal so every producer — auto-update, background AI
 * runs, long-running git operations — can map its own state into one stable
 * shape without pulling in feature-specific types. The tasks drawer
 * subscribes to a `Readable<TaskEntry | null>` per producer and renders
 * them side-by-side.
 */

/** Lifecycle phase a task is currently in. */
export type TaskEntryStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

/**
 * Snapshot of a background task for the tasks drawer.
 *
 * Producers map their feature-specific state into this shape. Drawer
 * consumers subscribe via a `Readable<TaskEntry | null>` — `null` means
 * the producer has no active task to surface right now.
 */
export interface TaskEntry {
  /** Stable identifier for the task (e.g. `"auto-update"`). */
  id: string;
  /** Producer category for grouping in the drawer. */
  kind: "update" | "ai-background" | "git" | "other";
  /** Human-readable title, localized. */
  title: string;
  /** Optional one-line secondary text (release notes, error message, …). */
  subtitle?: string;
  /** Lifecycle phase. */
  status: TaskEntryStatus;
  /** 0–1 fraction when known; omit for indeterminate tasks. */
  progress?: number;
  /** Ms since epoch when this task started. */
  startedAt?: number;
  /** Optional error message when `status === "failed"`. */
  error?: string;
}
