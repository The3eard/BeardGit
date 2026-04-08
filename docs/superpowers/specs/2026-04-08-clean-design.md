# Clean

## Overview

Remove untracked files and directories from the working tree. Two access points: per-file delete from the staging area context menu, and a bulk clean dialog with preview, filter toggles, and per-file selection. Destructive and irreversible — UI emphasizes preview before execution.

## Rust Changes

### git-engine crate (`crates/git-engine/src/clean.rs`)

New module for clean operations.

**`CleanItem` struct:**

| Field | Type | Purpose |
|---|---|---|
| `path` | `String` | Relative path from repo root |
| `is_directory` | `bool` | Whether the item is a directory |
| `is_ignored` | `bool` | Whether the item matches .gitignore |

**Functions:**

```rust
/// Dry-run: list files that would be removed.
/// Flags control what's included.
pub fn clean_dry_run(
    repo: &Repository,
    include_directories: bool,
    include_ignored: bool,
    only_ignored: bool,
) -> Result<Vec<CleanItem>, GitError>

/// Remove specific files/directories from the working tree.
/// Paths must be relative to repo root.
pub fn clean_paths(
    repo: &Repository,
    paths: &[String],
) -> Result<u32, GitError>
```

**Implementation:**

- `clean_dry_run`: Uses `git clean -n` with flags (`-d`, `-x`, `-X`) and parses output lines ("Would remove ..."). System git CLI, consistent with other write operations.
- `clean_paths`: Iterates paths and removes via `std::fs::remove_file` / `std::fs::remove_dir_all`. Uses filesystem directly rather than `git clean` for per-path control. Validates all paths are within the repo working directory before deletion (security check). Returns count of removed items.

### app-core commands (`crates/app-core/src/commands.rs`)

Two new commands:

```rust
#[tauri::command]
pub fn clean_dry_run(
    state: State<AppState>,
    include_directories: bool,
    include_ignored: bool,
    only_ignored: bool,
) -> Result<Vec<CleanItem>, String>

#[tauri::command]
pub fn clean_paths(
    state: State<AppState>,
    paths: Vec<String>,
) -> Result<u32, String>
```

## Frontend Changes

### Types (`src/lib/types/index.ts`)

```typescript
interface CleanItem {
  path: string;
  is_directory: boolean;
  is_ignored: boolean;
}
```

### IPC (`src/lib/api/tauri.ts`)

```typescript
export async function cleanDryRun(
  includeDirectories: boolean,
  includeIgnored: boolean,
  onlyIgnored: boolean
): Promise<CleanItem[]>

export async function cleanPaths(paths: string[]): Promise<number>
```

### Per-File Delete (Context Menu)

In `ChangesList.svelte`, add to the right-click context menu for untracked files:

| Action | Behavior |
|---|---|
| Delete untracked file | Confirmation dialog → `cleanPaths([path])` → refresh statuses |

Uses existing `ConfirmDialog` with destructive warning styling. Message: "This will permanently delete {filename}. This cannot be undone."

### Clean Button (StagingArea.svelte)

Add a "Clean" button in the unstaged section header, visible only when untracked files exist. Opens the `CleanDialog`.

### Clean Dialog (`src/lib/components/changes/CleanDialog.svelte`)

**Layout:**

```
╔══════════════════════════════════════════════╗
║  Clean Working Directory                  ✕  ║
╠══════════════════════════════════════════════╣
║                                              ║
║  ☑ Include directories                       ║
║  ☐ Include ignored files                     ║
║  ☐ Only ignored files                        ║
║                                              ║
║  ┌──────────────────────────────────────┐    ║
║  │ ☑ ☐ Select all (12 files)           │    ║
║  │──────────────────────────────────────│    ║
║  │ ☑  src/temp.ts                      │    ║
║  │ ☑  build/                       DIR │    ║
║  │ ☑  .env.local              IGNORED  │    ║
║  │ ☑  node_modules/.cache/        DIR  │    ║
║  │ ☐  notes.md                         │    ║
║  │ ...                                 │    ║
║  └──────────────────────────────────────┘    ║
║                                              ║
║  ⚠ This will permanently delete 11 files.    ║
║    This cannot be undone.                    ║
║                                              ║
║              [Cancel]  [Delete Selected]     ║
╚══════════════════════════════════════════════╝
```

**Behavior:**

- Toggle filters at top → triggers `cleanDryRun()` with updated flags → refreshes preview list
- Each item has a checkbox, defaults to checked
- "Select all" toggle in list header
- Items tagged: `DIR` badge for directories, `IGNORED` badge for ignored files
- File count and warning message update based on checked items
- "Delete Selected" button in destructive red styling
- On confirm: calls `cleanPaths()` with checked paths → closes dialog → refreshes statuses
- Dismiss: Escape, click outside, Cancel button

**Filter toggle interactions:**

- "Include directories" checked by default
- "Include ignored" unchecked by default
- "Only ignored" unchecked by default, mutually exclusive with "Include ignored" (enabling one disables the other)

### Fixed position dialog

Uses same dialog pattern as `ConfirmDialog` and `TagCreateDialog` — fixed position, centered, backdrop click to dismiss.

## i18n Keys

- `clean_title` — "Clean Working Directory"
- `clean_include_directories` — "Include directories"
- `clean_include_ignored` — "Include ignored files"
- `clean_only_ignored` — "Only ignored files"
- `clean_select_all` — "Select all ({count} files)"
- `clean_warning` — "This will permanently delete {count} files. This cannot be undone."
- `clean_delete_button` — "Delete Selected"
- `clean_button` — "Clean"
- `clean_dir_badge` — "DIR"
- `clean_ignored_badge` — "IGNORED"
- `clean_delete_file` — "Delete untracked file"
- `clean_delete_confirm` — "This will permanently delete {filename}. This cannot be undone."

## Scope Boundaries

**In scope:**
- Dry-run preview with filter toggles
- Per-file checkbox selection in bulk dialog
- Per-file delete from context menu
- Destructive action warnings
- Path validation (security: must be within repo)

**Out of scope:**
- Clean with custom exclude patterns (future)
- Undo/trash instead of permanent delete (OS-dependent, future)
- Clean from paths other than repo root
