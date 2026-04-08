# Patch Management

## Overview

Create and apply patches. Two creation paths: commit-based patches from the graph context menu, and working tree patches from the changes view. Import via native file dialog with dry-run preview and three-way fallback for conflicts.

## Rust Changes

### git-engine crate (`crates/git-engine/src/patch.rs`)

New module for patch operations.

**`PatchStat` struct:**

| Field | Type | Purpose |
|---|---|---|
| `path` | `String` | File path affected |
| `insertions` | `u32` | Lines added |
| `deletions` | `u32` | Lines removed |

**`PatchPreview` struct:**

| Field | Type | Purpose |
|---|---|---|
| `applies_cleanly` | `bool` | Whether patch applies without conflicts |
| `stats` | `Vec<PatchStat>` | Per-file change summary |
| `total_files` | `u32` | Number of files affected |
| `total_insertions` | `u32` | Total lines added |
| `total_deletions` | `u32` | Total lines removed |

**Functions:**

```rust
/// Create patch file(s) from one or more commits.
/// Uses `git format-patch` which produces one .patch file per commit.
/// Returns the list of created file paths.
pub fn create_commit_patches(
    repo: &Repository,
    oids: &[String],
    output_dir: &str,
) -> Result<Vec<String>, GitError>

/// Create a patch from the current working tree diff.
/// If staged_only is true, uses `git diff --cached`.
/// Otherwise uses `git diff` (unstaged changes).
/// Returns the patch content as a string.
pub fn create_working_tree_patch(
    repo: &Repository,
    staged_only: bool,
) -> Result<String, GitError>

/// Preview a patch: dry-run + stat summary.
/// Checks if it applies cleanly via `git apply --check`.
/// Gets stats via `git apply --stat`.
pub fn preview_patch(
    repo: &Repository,
    patch_path: &str,
) -> Result<PatchPreview, GitError>

/// Apply a patch file.
/// If three_way is true, uses `git apply --3way` to create merge
/// conflicts instead of failing on conflict.
pub fn apply_patch(
    repo: &Repository,
    patch_path: &str,
    three_way: bool,
) -> Result<(), GitError>
```

**Implementation:**

- `create_commit_patches`: Uses `git format-patch <oid> -1 -o <dir>` for each OID. Returns the generated filenames.
- `create_working_tree_patch`: Uses `git diff` or `git diff --cached`, returns stdout as string.
- `preview_patch`: Runs `git apply --stat <path>` to get file stats, then `git apply --check <path>` to test if it applies. Parses stat output for per-file numbers.
- `apply_patch`: Uses `git apply <path>` or `git apply --3way <path>`. On three-way conflict, the repo enters a conflict state that the existing merge editor can handle.

### app-core commands (`crates/app-core/src/commands.rs`)

Four new commands:

```rust
#[tauri::command]
pub fn create_commit_patches(
    state: State<AppState>,
    oids: Vec<String>,
    output_dir: String,
) -> Result<Vec<String>, String>

#[tauri::command]
pub fn create_working_tree_patch(
    state: State<AppState>,
    staged_only: bool,
) -> Result<String, String>

#[tauri::command]
pub fn preview_patch(
    state: State<AppState>,
    patch_path: String,
) -> Result<PatchPreview, String>

#[tauri::command]
pub fn apply_patch(
    state: State<AppState>,
    patch_path: String,
    three_way: bool,
) -> Result<(), String>
```

## Frontend Changes

### Types (`src/lib/types/index.ts`)

```typescript
interface PatchStat {
  path: string;
  insertions: number;
  deletions: number;
}

interface PatchPreview {
  applies_cleanly: boolean;
  stats: PatchStat[];
  total_files: number;
  total_insertions: number;
  total_deletions: number;
}
```

### IPC (`src/lib/api/tauri.ts`)

```typescript
export async function createCommitPatches(oids: string[], outputDir: string): Promise<string[]>
export async function createWorkingTreePatch(stagedOnly: boolean): Promise<string>
export async function previewPatch(patchPath: string): Promise<PatchPreview>
export async function applyPatch(patchPath: string, threeWay: boolean): Promise<void>
```

### Create Patch — Graph Context Menu (GitGraph.svelte)

Add to the existing commit right-click context menu:

| Action | Behavior |
|---|---|
| Create patch | Native save dialog → `createCommitPatches([oid], outputDir)` |

Uses Tauri's `dialog.save()` to pick output location. Default filename: `{short_oid}-{sanitized-summary}.patch`.

### Create Patch — Changes View (StagingArea.svelte)

Add a "Create Patch" button visible when there are staged or unstaged changes. Opens a small dialog:

```
╔═══════════════════════════════════════╗
║  Create Patch                     ✕   ║
╠═══════════════════════════════════════╣
║                                       ║
║  Source:                              ║
║  ○ Staged changes                     ║
║  ○ Unstaged changes                   ║
║                                       ║
║            [Cancel]  [Save Patch]     ║
╚═══════════════════════════════════════╝
```

- Radio buttons for staged vs unstaged
- "Save Patch" opens native save dialog → writes the patch content string to the selected file path
- If no changes in selected source, button is disabled with tooltip

### Apply Patch — Dialog (`src/lib/components/patch/ApplyPatchDialog.svelte`)

Accessible from a toolbar button or keyboard shortcut. Flow:

1. **File selection:** Native open dialog filtered to `.patch`, `.diff` files
2. **Preview:** Show dry-run results in a dialog

```
╔══════════════════════════════════════════════════════╗
║  Apply Patch                                     ✕   ║
╠══════════════════════════════════════════════════════╣
║                                                      ║
║  📄 fix-auth-bug.patch                               ║
║                                                      ║
║  ✓ Applies cleanly                                   ║
║                                                      ║
║  ┌────────────────────────────────────────────────┐  ║
║  │ File               │ Insertions │ Deletions    │  ║
║  ├────────────────────┼────────────┼──────────────┤  ║
║  │ src/lib/auth.ts    │ +12        │ -3           │  ║
║  │ src/lib/api.ts     │ +4         │ -1           │  ║
║  │ tests/auth.test.ts │ +25        │ -0           │  ║
║  └────────────────────┴────────────┴──────────────┘  ║
║                                                      ║
║  3 files changed, +41 insertions, -4 deletions       ║
║                                                      ║
║                          [Cancel]  [Apply Patch]     ║
╚══════════════════════════════════════════════════════╝
```

If patch doesn't apply cleanly:

```
║  ⚠ Patch does not apply cleanly                      ║
║                                                      ║
║  ... (same stats table) ...                          ║
║                                                      ║
║       [Cancel]  [Apply with 3-way merge]             ║
```

3. **Apply:** calls `applyPatch(path, false)` for clean apply, or `applyPatch(path, true)` for three-way
4. **Post-apply:** Refresh statuses. If three-way created conflicts, the existing ConflictToolbar activates automatically via the repo watcher.

### Access Point

Add "Apply Patch" to the toolbar or as a menu item accessible from the Changes view. Also available via keyboard shortcut (can be added to shortcuts spec later).

## i18n Keys

- `patch_create` — "Create Patch"
- `patch_create_from_commit` — "Create patch"
- `patch_source_staged` — "Staged changes"
- `patch_source_unstaged` — "Unstaged changes"
- `patch_save` — "Save Patch"
- `patch_apply` — "Apply Patch"
- `patch_applies_cleanly` — "Applies cleanly"
- `patch_does_not_apply` — "Patch does not apply cleanly"
- `patch_apply_3way` — "Apply with 3-way merge"
- `patch_files_changed` — "{count} files changed, +{insertions} insertions, -{deletions} deletions"
- `patch_file` — "File"
- `patch_insertions` — "Insertions"
- `patch_deletions` — "Deletions"

## Scope Boundaries

**In scope:**
- Create patches from commits (graph context menu)
- Create patches from working tree (staged or unstaged)
- Apply patches with dry-run preview
- Three-way merge fallback for conflicting patches
- Native file dialogs for save/open

**Out of scope:**
- `git am` for mailbox-format patch series (future)
- Patch editing before apply
- Drag and drop patch import (future)
- Creating patches from commit ranges (multi-select in graph — future)
- Patch queue management
