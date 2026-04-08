# Git Config Viewer/Editor

## Overview

New section in the Settings view for viewing and editing git configuration. Two-column layout showing local (repo) and global (user) config side by side. Structured key-value table with inline editing, searchable, with dropdown selectors for known enum-type keys and free text for everything else. System config shown as read-only.

## Rust Changes

### git-engine crate (`crates/git-engine/src/config.rs`)

New module for config operations.

**`ConfigEntry` struct:**

| Field | Type | Purpose |
|---|---|---|
| `key` | `String` | Full key, e.g. `user.name`, `core.autocrlf` |
| `value` | `String` | Current value |
| `scope` | `ConfigScope` | `Local`, `Global`, or `System` |

**`ConfigScope` enum:**

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigScope {
    Local,
    Global,
    System,
}
```

**`ConfigValueType` enum (frontend-only, for UI hints):**

Not needed in Rust — the frontend maintains a map of known keys to their valid values.

**Functions:**

```rust
/// List all config entries for a given scope.
/// Uses `git config --list --local/--global/--system` with `--null` separator for safe parsing.
pub fn list_config(repo: &Repository, scope: ConfigScope) -> Result<Vec<ConfigEntry>, GitError>

/// Set a config value at the given scope.
pub fn set_config(
    repo: &Repository,
    scope: ConfigScope,
    key: &str,
    value: &str,
) -> Result<(), GitError>

/// Unset (remove) a config key at the given scope.
pub fn unset_config(
    repo: &Repository,
    scope: ConfigScope,
    key: &str,
) -> Result<(), GitError>

/// Add a new config entry at the given scope.
/// For multi-value keys, uses `git config --add`.
pub fn add_config(
    repo: &Repository,
    scope: ConfigScope,
    key: &str,
    value: &str,
) -> Result<(), GitError>
```

**Implementation:** All operations use system `git config` CLI for consistency with how git itself reads/writes config. Parse `--list --null` output (NUL-separated key=value pairs) for safe handling of values containing newlines.

### app-core commands (`crates/app-core/src/commands.rs`)

Four new commands:

```rust
#[tauri::command]
pub fn list_config(state: State<AppState>, scope: ConfigScope) -> Result<Vec<ConfigEntry>, String>

#[tauri::command]
pub fn set_config(state: State<AppState>, scope: ConfigScope, key: String, value: String) -> Result<(), String>

#[tauri::command]
pub fn unset_config(state: State<AppState>, scope: ConfigScope, key: String) -> Result<(), String>

#[tauri::command]
pub fn add_config(state: State<AppState>, scope: ConfigScope, key: String, value: String) -> Result<(), String>
```

## Frontend Changes

### Types (`src/lib/types/index.ts`)

```typescript
interface ConfigEntry {
  key: string;
  value: string;
  scope: "local" | "global" | "system";
}
```

### IPC (`src/lib/api/tauri.ts`)

```typescript
export async function listConfig(scope: "local" | "global" | "system"): Promise<ConfigEntry[]>
export async function setConfig(scope: "local" | "global", key: string, value: string): Promise<void>
export async function unsetConfig(scope: "local" | "global", key: string): Promise<void>
export async function addConfig(scope: "local" | "global", key: string, value: string): Promise<void>
```

### Known Config Keys Map (`src/lib/utils/git-config-keys.ts`)

Static map of known git config keys with their valid value sets for dropdown rendering:

```typescript
const CONFIG_KEY_OPTIONS: Record<string, string[]> = {
  "core.autocrlf": ["true", "false", "input"],
  "core.ignorecase": ["true", "false"],
  "core.filemode": ["true", "false"],
  "core.symlinks": ["true", "false"],
  "pull.rebase": ["true", "false", "merges", "interactive"],
  "pull.ff": ["true", "false", "only"],
  "push.default": ["nothing", "current", "upstream", "tracking", "simple", "matching"],
  "push.autoSetupRemote": ["true", "false"],
  "merge.ff": ["true", "false", "only"],
  "merge.conflictstyle": ["merge", "diff3", "zdiff3"],
  "rebase.autosquash": ["true", "false"],
  "rebase.autostash": ["true", "false"],
  "fetch.prune": ["true", "false"],
  "init.defaultBranch": ["main", "master"],
  "color.ui": ["auto", "always", "never"],
  "diff.algorithm": ["default", "minimal", "patience", "histogram"],
  // Boolean-type keys get ["true", "false"] automatically
};
```

Keys not in this map render as free text input.

### Settings Section (`src/lib/components/settings/GitConfigSettings.svelte`)

**Layout: Two-Column Table**

```
╔══════════════════════════════════════════════════════════════╗
║  Git Configuration                                          ║
║                                                             ║
║  🔍 Filter keys...                                          ║
║                                                             ║
║  ┌─────────────┬──────────────────┬──────────────────┐      ║
║  │ Key         │ Local (project)  │ Global (user)    │      ║
║  ├─────────────┼──────────────────┼──────────────────┤      ║
║  │ user.name   │ —                │ Adolfo Fuentes   │      ║
║  │ user.email  │ adolfo@work.com  │ adolfo@home.com  │      ║
║  │ core.autocrlf│ —               │ ▼ input          │      ║
║  │ pull.rebase │ ▼ true           │ —                │      ║
║  │ push.default│ —                │ ▼ simple         │      ║
║  │ ...         │                  │                  │      ║
║  └─────────────┴──────────────────┴──────────────────┘      ║
║                                                             ║
║  [+ Add Entry]                                              ║
║                                                             ║
║  ▸ System (read-only)                                       ║
╚══════════════════════════════════════════════════════════════╝
```

**Behavior:**

- **Merged key list:** Union of all keys from local + global, sorted alphabetically by section then key
- **Filter input:** Debounced text filter across key names
- **Local column:** Shows value if key exists in local config, "—" if not. Editable.
- **Global column:** Shows value if key exists in global config, "—" if not. Editable with subtle "affects all repos" warning on hover.
- **Value editing:** Click a cell to edit. Known enum keys show dropdown, others show text input. Press Enter to save, Escape to cancel.
- **Empty cell click:** Clicking "—" in a column lets you set a value at that scope (adds the key)
- **Delete:** Small ✕ button on hover next to a value to unset it at that scope
- **Add Entry:** Button at bottom opens inline row with key input + value inputs for both scopes
- **System section:** Collapsible, read-only, shown at the bottom. Collapsed by default.

**Visual details:**

- Dropdown cells show ▼ indicator
- Edited values flash briefly on save (success feedback)
- Global column has a subtle header badge: "affects all repos"
- System section header: "System (read-only)"

### Settings Page Integration (`SettingsPage.svelte`)

Add new section to the sections array:

```typescript
{ labelKey: () => m.settings_git_config(), id: "git-config" }
```

Wire in the `{#if activeSection === "git-config"}` block to render `GitConfigSettings`.

## i18n Keys

- `settings_git_config` — "Git Config"
- `config_title` — "Git Configuration"
- `config_filter_placeholder` — "Filter keys..."
- `config_key` — "Key"
- `config_local` — "Local (project)"
- `config_global` — "Global (user)"
- `config_system` — "System (read-only)"
- `config_no_value` — "—"
- `config_add_entry` — "Add Entry"
- `config_global_warning` — "This affects all repositories"
- `config_key_placeholder` — "section.key"
- `config_value_placeholder` — "value"

## Scope Boundaries

**In scope:**
- Read local, global, and system config
- Edit local and global config (set, unset, add)
- Two-column layout with merged key list
- Dropdown selectors for known enum-type keys
- Free text input for unknown keys
- Search/filter by key name
- System config read-only, collapsible

**Out of scope:**
- Multi-value key editing UI (e.g. multiple `remote.origin.fetch` entries) — show first value, edit replaces it
- Config file syntax validation beyond what git provides
- Conditional includes (`includeIf`) management
- Creating new config sections/subsections with guided UI
