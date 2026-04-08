# Submodules

## Overview

New sidebar view for managing git submodules. Each submodule can be opened as a full project tab (reusing the entire existing UI) with a badge indicating it's a submodule. The sidebar list shows status and provides quick actions (init, update, deinit) without needing to open the tab.

## Rust Changes

### git-engine crate (`crates/git-engine/src/submodule.rs`)

New module for submodule operations.

**`SubmoduleInfo` struct:**

| Field | Type | Purpose |
|---|---|---|
| `name` | `String` | Submodule name from `.gitmodules` |
| `path` | `String` | Relative path from repo root |
| `url` | `String` | Remote URL |
| `oid` | `Option<String>` | Current HEAD commit SHA (None if uninitialized) |
| `registered_oid` | `String` | Commit SHA registered in parent repo's index |
| `status` | `SubmoduleStatus` | Current state |

**`SubmoduleStatus` enum:**

```rust
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmoduleStatus {
    /// Registered in .gitmodules but not initialized
    Uninitialized,
    /// Initialized and clean — HEAD matches registered commit
    Clean,
    /// Initialized but HEAD differs from registered commit
    Outdated,
    /// Has uncommitted changes in working tree
    Dirty,
}
```

**Functions:**

```rust
/// List all submodules with their status.
/// Uses libgit2's Submodule API for reading.
pub fn list_submodules(repo: &Repository) -> Result<Vec<SubmoduleInfo>, GitError>

/// Initialize a submodule (git submodule init).
pub fn init_submodule(repo: &Repository, path: &str) -> Result<(), GitError>

/// Update a submodule to the registered commit (git submodule update --init).
/// Runs as a background task since it may fetch from remote.
pub fn update_submodule(repo: &Repository, path: &str) -> Result<(), GitError>

/// Update all submodules (git submodule update --init --recursive).
/// Runs as a background task.
pub fn update_all_submodules(repo: &Repository) -> Result<(), GitError>

/// Deinitialize a submodule (git submodule deinit).
pub fn deinit_submodule(repo: &Repository, path: &str, force: bool) -> Result<(), GitError>

/// Get the absolute path for a submodule, for opening as a project tab.
pub fn submodule_abs_path(repo: &Repository, path: &str) -> Result<String, GitError>
```

**Implementation:**

- `list_submodules`: Uses libgit2's `repo.submodules()` for reading `.gitmodules` and index state. Checks `submodule.head_id()` vs `submodule.index_id()` for outdated detection. Checks `submodule.status()` flags for dirty/uninitialized.
- `init_submodule`, `update_submodule`, `deinit_submodule`: Use system `git submodule` CLI for write operations (consistent with the hybrid approach).
- `update_submodule` and `update_all_submodules`: Should be spawned via `TaskManager` since they involve network fetches.

### app-core commands (`crates/app-core/src/commands.rs`)

```rust
#[tauri::command]
pub fn list_submodules(state: State<AppState>) -> Result<Vec<SubmoduleInfo>, String>

#[tauri::command]
pub fn init_submodule(state: State<AppState>, path: String) -> Result<(), String>

#[tauri::command]
pub fn deinit_submodule(state: State<AppState>, path: String, force: bool) -> Result<(), String>

#[tauri::command]
pub fn submodule_abs_path(state: State<AppState>, path: String) -> Result<String, String>
```

Update and update-all go through the task system:

```rust
#[tauri::command]
pub async fn update_submodule(
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
    path: String,
) -> Result<u64, String>

#[tauri::command]
pub async fn update_all_submodules(
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<u64, String>
```

These return `TaskId` for progress tracking via the task system.

## Frontend Changes

### Types (`src/lib/types/index.ts`)

```typescript
interface SubmoduleInfo {
  name: string;
  path: string;
  url: string;
  oid: string | null;
  registered_oid: string;
  status: "uninitialized" | "clean" | "outdated" | "dirty";
}
```

### IPC (`src/lib/api/tauri.ts`)

```typescript
export async function listSubmodules(): Promise<SubmoduleInfo[]>
export async function initSubmodule(path: string): Promise<void>
export async function updateSubmodule(path: string): Promise<number> // TaskId
export async function updateAllSubmodules(): Promise<number> // TaskId
export async function deinitSubmodule(path: string, force: boolean): Promise<void>
export async function submoduleAbsPath(path: string): Promise<string>
```

### Store (`src/lib/stores/submodules.ts`)

- `submodules: writable<SubmoduleInfo[]>` — the submodule list
- `loadSubmodules()` — fetches from backend, called on view activation
- Auto-refresh via `repo-changed` watcher event

### Sidebar (`Sidebar.svelte`)

Add "Submodules" nav item after "Worktrees":

- Icon: Nerd Font submodule/nested-repo glyph
- ID: `"submodules"`
- Only shown if repo has submodules (check `submodules.length > 0` or always show — decide during implementation)

### View (`src/lib/components/submodules/SubmoduleList.svelte`)

**Layout:** Flat list with action buttons.

**Row layout:**

```
┌──────────────────────────────────────────────────────────┐
│  libs/crypto                                             │
│  https://github.com/org/crypto.git     a1b2c3d  ● Clean │
│                              [Init] [Update] [Open Tab]  │
└──────────────────────────────────────────────────────────┘
```

- **Row 1:** Submodule path (primary text)
- **Row 2:** URL (secondary text) + short SHA + status badge
- **Actions:** Contextual buttons based on status

**Status badges:**

| Status | Badge | Color |
|---|---|---|
| Uninitialized | `UNINIT` | text-secondary |
| Clean | `CLEAN` | accent-green |
| Outdated | `OUTDATED` | accent-orange |
| Dirty | `DIRTY` | accent-red |

**Contextual actions per status:**

| Status | Available actions |
|---|---|
| Uninitialized | Init, Init + Update |
| Clean | Open in Tab, Update, Deinit |
| Outdated | Open in Tab, Update, Deinit |
| Dirty | Open in Tab, Deinit (with force confirmation) |

**Header actions:**

- "Update All" button — runs `updateAllSubmodules()` as background task

### Open as Tab

"Open in Tab" action:

1. Calls `submoduleAbsPath(path)` to get absolute path
2. Calls existing `openProject(absPath)` to add as a project tab
3. The tab opens with full BeardGit functionality (graph, changes, branches, etc.)

### Tab Badge (`ProjectTab.svelte`)

When a project tab's path is a subdirectory of another open project's path and matches a known submodule path, show a small badge on the tab:

- Badge: small `SUB` text or a nested-repo icon
- Color: `accent-purple` to distinguish from regular project tabs
- Tooltip: "Submodule of {parent-project-name}"

**Detection:** When opening a project, check if its path is a submodule of any other open project by comparing paths. Store a `isSubmodule: boolean` flag on the `ProjectSlot` or detect at the frontend level.

### Context Menu

Right-click on a submodule row:

| Action | Behavior |
|---|---|
| Open in Tab | Open submodule as project tab |
| Init | `initSubmodule(path)` (if uninitialized) |
| Update | `updateSubmodule(path)` (background task) |
| Deinit | Confirmation dialog → `deinitSubmodule(path, false)` |
| Force Deinit | Confirmation dialog with destructive warning → `deinitSubmodule(path, true)` |
| Copy Path | Copy submodule path to clipboard |
| Copy URL | Copy remote URL to clipboard |

### Page Integration (`+page.svelte`)

Add `{#if activeView === "submodules"}` block:

```svelte
{:else if activeView === "submodules"}
  <SubmoduleList />
```

No detail panel needed — "Open in Tab" is the detail view.

## i18n Keys

- `sidebar_submodules` — "Submodules"
- `submodules_title` — "Submodules"
- `submodules_empty` — "No submodules in this repository"
- `submodules_update_all` — "Update All"
- `submodules_open_tab` — "Open in Tab"
- `submodules_init` — "Init"
- `submodules_update` — "Update"
- `submodules_deinit` — "Deinit"
- `submodules_force_deinit` — "Force Deinit"
- `submodules_deinit_confirm` — "Deinitialize submodule {name}? This removes the working tree."
- `submodules_force_deinit_confirm` — "Force deinitialize {name}? This discards local changes."
- `submodules_status_uninitialized` — "Uninitialized"
- `submodules_status_clean` — "Clean"
- `submodules_status_outdated` — "Outdated"
- `submodules_status_dirty` — "Dirty"
- `submodules_tab_badge` — "Submodule of {parent}"
- `submodules_copy_path` — "Copy Path"
- `submodules_copy_url` — "Copy URL"

## Scope Boundaries

**In scope:**
- List submodules with status (uninitialized, clean, outdated, dirty)
- Init, update, deinit operations
- Update all submodules as background task
- Open submodule as full project tab with badge
- Context menu with actions
- Auto-refresh on repo changes

**Out of scope:**
- Adding new submodules (`git submodule add`) — use terminal for now
- Removing submodules from `.gitmodules` (complex multi-step process)
- Nested submodule tree view (flat list sufficient)
- Submodule-specific graph integration (e.g. showing parent's registered commit in submodule graph)
- Recursive submodule status (submodules within submodules)
