# Reflog Viewer

## Overview

New sidebar view showing the reflog — every HEAD movement in the repo (commits, checkouts, rebases, resets, merges). Primary use case: understanding what happened and recovering from mistakes. Supports inline commit detail, "Show in Graph" navigation, and recovery actions via context menu.

## Rust Changes

### git-engine crate (`crates/git-engine/src/reflog.rs`)

New module for reflog operations.

**`ReflogEntry` struct:**

| Field | Type | Purpose |
|---|---|---|
| `oid` | `String` | Commit OID after the action |
| `prev_oid` | `String` | Commit OID before the action |
| `action` | `String` | Action type: "commit", "checkout", "rebase", "reset", "merge", "pull", "cherry-pick", etc. |
| `summary` | `String` | Full reflog message, e.g. "checkout: moving from main to feature/x" |
| `author` | `String` | Who performed the action |
| `email` | `String` | Author email |
| `timestamp` | `i64` | Unix timestamp |

**Functions:**

```rust
/// Read reflog entries for HEAD, most recent first.
/// Uses libgit2's `Reference::log()` for reading.
pub fn get_reflog(repo: &Repository, limit: usize) -> Result<Vec<ReflogEntry>, GitError>
```

Default limit: 100 entries. Pagination via offset if needed later.

**Parsing:** libgit2's `Reflog` API provides structured access to each entry (id_new, id_old, committer, message). Parse the message prefix to extract the action type (text before the first colon).

### app-core commands (`crates/app-core/src/commands.rs`)

One new command:

```rust
#[tauri::command]
pub fn get_reflog(state: State<AppState>, limit: Option<usize>) -> Result<Vec<ReflogEntry>, String>
```

Delegates to `git-engine::reflog::get_reflog()` via `with_active_repo`.

## Frontend Changes

### Types (`src/lib/types/index.ts`)

```typescript
interface ReflogEntry {
  oid: string;
  prev_oid: string;
  action: string;
  summary: string;
  author: string;
  email: string;
  timestamp: number;
}
```

### IPC (`src/lib/api/tauri.ts`)

```typescript
export async function getReflog(limit?: number): Promise<ReflogEntry[]>
```

### Store (`src/lib/stores/reflog.ts`)

- `reflogEntries: writable<ReflogEntry[]>` — the entry list
- `selectedReflogEntry: writable<ReflogEntry | null>` — currently selected entry
- `loadReflog()` — fetches entries from backend, called on view activation and after mutations
- Auto-refresh via `repo-changed` watcher event

### Sidebar (`Sidebar.svelte`)

Add "Reflog" nav item after "Worktrees":

- Icon: Nerd Font history/clock glyph
- ID: `"reflog"`
- Wired to `onNavigate` callback

### View (`src/lib/components/reflog/ReflogList.svelte`)

**Layout:** Same pattern as BranchList — list on the left, detail panel on the right.

**Entry row:**
```
┌──────────────────────────────────────────────┐
│ 🔄 checkout: moving from main to feature/x  │
│    a1b2c3d → e4f5g6h              2m ago     │
└──────────────────────────────────────────────┘
```

- **Row 1:** Action icon + summary message
- **Row 2:** Short OIDs (prev → new) + relative timestamp
- Action icons by type: commit (●), checkout (⇄), rebase (↻), reset (↩), merge (⑂), pull (↓), other (○)
- Click to select → shows CommitDetail in side panel
- Right-click → context menu with recovery actions

**Action icons mapping:**

| Action | Icon | Color |
|---|---|---|
| commit | ● | accent-green |
| checkout | branch glyph | accent-blue |
| rebase | ↻ | accent-purple |
| reset | ↩ | accent-orange |
| merge | merge glyph | accent-blue |
| pull | ↓ | accent-blue |
| cherry-pick | cherry glyph | accent-green |
| other | ○ | text-secondary |

### Detail Panel (`src/lib/components/reflog/ReflogDetail.svelte`)

Reuses existing `CommitDetail` component for the selected entry's `oid`. Adds a "Show in Graph" button at the top that:

1. Switches `activeView` to `"graph"`
2. Navigates the graph viewport to the entry's OID
3. Selects the commit in the graph

### Context Menu

Right-click on a reflog entry shows:

| Action | Behavior |
|---|---|
| Checkout this commit | `checkoutBranch(entry.oid)` — detached HEAD |
| Create branch here | Prompt for name, `createBranch(name, entry.oid)` |
| Reset to here | Reset submenu: soft/mixed/hard with confirmation |
| Copy SHA | Copy `entry.oid` to clipboard |
| Show in Graph | Navigate to commit in graph view |

These reuse the same API wrappers already used by the graph context menu.

### Page Integration (`+page.svelte`)

Add `{#if activeView === "reflog"}` block with the split layout:

```svelte
{:else if activeView === "reflog"}
  <div class="reflog-layout">
    <ReflogList />
    {#if $selectedReflogEntry}
      <div class="reflog-detail">
        <ReflogDetail />
      </div>
    {/if}
  </div>
```

## i18n Keys

- `sidebar_reflog` — "Reflog"
- `reflog_title` — "Reflog"
- `reflog_empty` — "No reflog entries"
- `reflog_show_in_graph` — "Show in Graph"
- `reflog_action_commit` — "commit"
- `reflog_action_checkout` — "checkout"
- `reflog_action_rebase` — "rebase"
- `reflog_action_reset` — "reset"
- `reflog_action_merge` — "merge"
- `reflog_action_pull` — "pull"
- Context menu labels reuse existing keys (`graph_ctx_checkout`, `graph_ctx_create_branch`, etc.)

## Scope Boundaries

**In scope:**
- Read reflog via libgit2
- Sidebar view with entry list + commit detail
- Action icons by reflog entry type
- Context menu with recovery actions (checkout, branch, reset)
- "Show in Graph" navigation
- Auto-refresh on repo changes

**Out of scope:**
- Reflog for branches other than HEAD (future — add branch selector)
- Reflog entry diffing (comparing prev_oid to oid — future enhancement)
- Reflog search/filtering (unnecessary at 100-entry scale)
- Reflog expiry management (`git reflog expire`)
