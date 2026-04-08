# Gitignore Management

## Overview

Two access points for managing `.gitignore`: a quick "Add to .gitignore" action on untracked files in the Changes view, and a full CodeMirror editor in Settings for viewing and editing all patterns.

## Rust Changes

### git-engine crate (`crates/git-engine/src/gitignore.rs`)

New module for gitignore operations.

**Functions:**

```rust
/// Read the contents of .gitignore at repo root.
/// Returns empty string if file doesn't exist.
pub fn read_gitignore(repo: &Repository) -> Result<String, GitError>

/// Write contents to .gitignore at repo root.
/// Creates the file if it doesn't exist.
pub fn write_gitignore(repo: &Repository, content: &str) -> Result<(), GitError>

/// Append a pattern to .gitignore.
/// Adds a newline before the pattern if file doesn't end with one.
/// Does nothing if the exact pattern already exists.
pub fn add_gitignore_pattern(repo: &Repository, pattern: &str) -> Result<(), GitError>
```

**Implementation:** Direct filesystem operations on `{repo_root}/.gitignore`. No git CLI needed — `.gitignore` is a regular file. `add_gitignore_pattern` reads the file first to check for duplicates, then appends.

### app-core commands (`crates/app-core/src/commands.rs`)

Three new commands:

```rust
#[tauri::command]
pub fn read_gitignore(state: State<AppState>) -> Result<String, String>

#[tauri::command]
pub fn write_gitignore(state: State<AppState>, content: String) -> Result<(), String>

#[tauri::command]
pub fn add_gitignore_pattern(state: State<AppState>, pattern: String) -> Result<(), String>
```

## Frontend Changes

### IPC (`src/lib/api/tauri.ts`)

```typescript
export async function readGitignore(): Promise<string>
export async function writeGitignore(content: string): Promise<void>
export async function addGitignorePattern(pattern: string): Promise<void>
```

### Quick Action: Add to .gitignore (ChangesList.svelte)

Add to the right-click context menu for untracked files:

| Action | Behavior |
|---|---|
| Add to .gitignore | Submenu with pattern options → `addGitignorePattern()` → refresh statuses |

**Submenu options** for a file like `src/temp/debug.log`:

- `debug.log` — ignore by filename anywhere
- `*.log` — ignore by extension
- `src/temp/debug.log` — ignore exact path
- `src/temp/` — ignore entire directory

The submenu is built dynamically from the file path:

```typescript
function gitignorePatterns(path: string): string[] {
  const parts = path.split("/");
  const filename = parts[parts.length - 1];
  const ext = filename.includes(".") ? "*." + filename.split(".").pop() : null;
  const dir = parts.length > 1 ? parts.slice(0, -1).join("/") + "/" : null;

  const patterns = [filename, path];
  if (ext) patterns.splice(1, 0, ext);
  if (dir) patterns.push(dir);
  return patterns;
}
```

After adding, auto-refreshes file statuses so the file disappears from the untracked list.

### Full Editor: Settings Section (`src/lib/components/settings/GitignoreSettings.svelte`)

**Layout:**

```
╔══════════════════════════════════════════════════════════╗
║  .gitignore                                              ║
║                                                          ║
║  ┌────────────────────────────────────────────────────┐  ║
║  │ # Dependencies                                     │  ║
║  │ node_modules/                                      │  ║
║  │ target/                                            │  ║
║  │                                                    │  ║
║  │ # Build output                                     │  ║
║  │ dist/                                              │  ║
║  │ build/                                             │  ║
║  │ *.o                                                │  ║
║  │                                                    │  ║
║  │ # Environment                                      │  ║
║  │ .env                                               │  ║
║  │ .env.local                                         │  ║
║  └────────────────────────────────────────────────────┘  ║
║                                                          ║
║  ⚠ Unsaved changes              [Revert]  [Save]        ║
╚══════════════════════════════════════════════════════════╝
```

**Behavior:**

- CodeMirror editor with gitignore syntax highlighting (comment lines starting with `#`, negation patterns starting with `!`)
- Loads content via `readGitignore()` on section activation
- Tracks dirty state by comparing current editor content with loaded content
- "Unsaved changes" warning appears when dirty
- **Save** button: calls `writeGitignore(content)` → refreshes file statuses → clears dirty state
- **Revert** button: reloads from disk, discards editor changes
- If `.gitignore` doesn't exist, shows empty editor. Saving creates the file.
- Editor height: fills available space in settings content area

**CodeMirror configuration:**

- Reuse existing `codemirror-theme.ts` for theme integration
- Line numbers enabled
- Basic syntax: comments (`#` lines) styled as `theme.comment`, negation (`!` prefix) styled as `accent-orange`
- No complex gitignore grammar needed — comment highlighting is sufficient

### Settings Page Integration (`SettingsPage.svelte`)

Add new section to the sections array:

```typescript
{ labelKey: () => m.settings_gitignore(), id: "gitignore" }
```

Wire in the `{#if activeSection === "gitignore"}` block to render `GitignoreSettings`.

## i18n Keys

- `settings_gitignore` — ".gitignore"
- `gitignore_title` — ".gitignore"
- `gitignore_unsaved` — "Unsaved changes"
- `gitignore_save` — "Save"
- `gitignore_revert` — "Revert"
- `gitignore_empty` — "No .gitignore file. Save to create one."
- `gitignore_add_to` — "Add to .gitignore"

## Scope Boundaries

**In scope:**
- Read/write `.gitignore` at repo root
- Quick "Add to .gitignore" from untracked file context menu with smart pattern suggestions
- Full CodeMirror editor in Settings with save/revert
- Basic syntax highlighting (comments, negation)
- Dirty state tracking

**Out of scope:**
- Nested `.gitignore` files in subdirectories (only repo root)
- Global gitignore (`~/.gitignore_global`) editing
- `.git/info/exclude` editing
- Pattern testing/validation ("would this pattern match X?")
- Template suggestions from gitignore.io or GitHub templates
