# Task History Popup

## Overview

Replace the current "spinner only when running" status bar behavior with an always-clickable task area that opens a history popup showing all session tasks. The popup is an overview index — click any task to open the full output panel.

Session-scoped, in-memory only. No persistence across restarts.

## Rust Changes

### task-runner crate (`crates/task-runner/src/types.rs`)

**`TaskHandle` additions:**

| Field | Type | Purpose |
|---|---|---|
| `command` | `String` | Full CLI command, e.g. `git pull origin main` |
| `started_at_ms` | `Option<u64>` | Unix epoch milliseconds at spawn time (replaces `Instant`-based `started_at`) |
| `finished_at_ms` | `Option<u64>` | Unix epoch milliseconds at completion |
| `exit_code` | `Option<i32>` | Process exit code, populated on completion/failure |

**`TaskInfo` additions** (serialized to frontend):

| Field | Type | Purpose |
|---|---|---|
| `command` | `String` | Passed through from `TaskHandle` |
| `started_at_ms` | `Option<u64>` | For frontend to compute relative time ("2m ago") |
| `exit_code` | `Option<i32>` | Shown as "exit N" on failed tasks |

Keep existing `elapsed_secs` — computed from the new ms timestamps.

**`started_at: Option<Instant>` stays** for internal elapsed computation but is no longer the source of truth for frontend timestamps. `started_at_ms` uses `SystemTime::now().duration_since(UNIX_EPOCH)` at spawn.

### task-runner crate (`crates/task-runner/src/manager.rs`)

**`TaskManager::spawn` changes:**

- Build the command string from program + args before spawning: `format!("{} {}", program, args.join(" "))`
- Record `started_at_ms` using `SystemTime::now()` converted to epoch ms
- On process exit: capture `ExitStatus::code()` into `exit_code`, record `finished_at_ms`

### app-core commands

No new IPC commands needed. Existing `get_tasks()` and `get_task_output()` return enriched `TaskInfo` automatically.

## Frontend Changes

### Types (`src/lib/types/index.ts`)

Update `TaskInfo` interface:

```typescript
interface TaskInfo {
  id: number;
  label: string;
  status: TaskStatus;
  cancellable: boolean;
  elapsed_secs: number | null;
  command: string;            // new
  started_at_ms: number | null; // new
  exit_code: number | null;     // new
}
```

### Store (`src/lib/stores/tasks.ts`)

Add a derived store for display-sorted tasks:

```typescript
export const sortedTasks = derived(tasks, ($tasks) => {
  const running = $tasks.filter(t => t.status.state === "running");
  const finished = $tasks
    .filter(t => t.status.state !== "running")
    .sort((a, b) => (b.started_at_ms ?? 0) - (a.started_at_ms ?? 0));
  return [...running, ...finished];
});
```

### Status Bar (`src/lib/components/layout/StatusBar.svelte`)

The task indicator area becomes always-clickable:

| State | Display | Click action |
|---|---|---|
| Tasks running | Spinner + count (current behavior) | Open popup |
| Recent failure (5s) | Failure icon + count (current behavior) | Open popup |
| Idle, has history | Subtle clock/history icon | Open popup |
| Idle, no history | Nothing shown | — |

The icon uses a Nerd Font glyph (history/clock icon). Same `togglePopover()` action as now.

### Task Popup (`src/lib/components/tasks/TaskPopover.svelte`)

**Layout: Two-Line Cards**

Each task row:
```
┌─────────────────────────────────────────────┐
│ ▌ Fetch origin                      running │
│ ▌ git fetch origin                     3.2s │
├─────────────────────────────────────────────┤
│ ▌ Pull origin/main                     1.4s │
│ ▌ git pull origin main               2m ago │
├─────────────────────────────────────────────┤
│ ▌ Push origin/feature               exit 1  │
│ ▌ git push origin feature/auth       5m ago │
└─────────────────────────────────────────────┘
```

- **Left border:** 3px colored bar — orange (running), green (completed), red (failed), gray (cancelled)
- **Row 1:** Label (left) + status indicator (right): "running" / duration / "exit N" / "cancelled"
- **Row 2:** Command in secondary text (left) + relative time (right)
- **Running tasks** shown at top with subtle orange background tint
- **Click action:** Close popup, open full task panel (`expandPanel()`), select clicked task (`selectTask(id)`)
- **Container:** `max-height: min(400px, 50vh)`, scrollable, positioned above status bar
- **Header:** "Tasks" + count badge + "Session history" label

### Existing Components

- **`TaskPanel.svelte`** — No changes needed. Already shows full output for `selectedTaskId`.
- **`TaskList.svelte`** — May be replaced by the new popup card layout, or kept for the panel view. Evaluate during implementation.

## Status Bar Idle Icon

When no tasks are running and no recent failures, show a clock/history Nerd Font icon only if tasks exist in the session. This provides a visual affordance that history is available.

When no tasks have ever run in the session, show nothing (current behavior for idle state).

## Future: Terminal Integration

The popup is designed as an index/launcher pattern. When libghostty is integrated in Phase 4:

- Task rows could open output in an embedded terminal view instead of the current panel
- The `command` field supports re-running tasks in a terminal
- The data model (command string, output stream, exit code) maps directly to terminal session metadata

No code accommodations needed now — the architecture naturally supports this evolution.

## Scope Boundaries

**In scope:**
- Enriched `TaskInfo` with command, timestamps, exit code
- Always-clickable status bar task area
- Two-line card popup layout
- Click-to-open full panel

**Out of scope:**
- Persistent history across restarts (future)
- Task re-run from popup (future, with terminal)
- Filtering/searching task history (unnecessary at session scale)
- Task grouping by repo/type (unnecessary at current volume)
