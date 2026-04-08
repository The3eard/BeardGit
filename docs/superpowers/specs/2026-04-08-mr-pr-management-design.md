# Merge Request / Pull Request Management

## Overview

Full MR/PR lifecycle management using bundled `gh` and `glab` CLI tools. Create, browse, review (with inline comments), approve, and merge MRs/PRs without leaving BeardGit. CLI-first approach: CLI handles MR/PR operations, CLI auth bootstraps the REST API token for pipelines. Bundled CLIs ship with the app (both MIT licensed).

## Architecture

### CLI Bundling

**Bundled binaries:**
- `gh` (GitHub CLI) — bundled in app resources per platform
- `glab` (GitLab CLI) — bundled in app resources per platform

**Platform matrix:**

| Platform | gh binary | glab binary |
|---|---|---|
| macOS arm64 | `gh_darwin_arm64` | `glab_darwin_arm64` |
| macOS x64 | `gh_darwin_amd64` | `glab_darwin_amd64` |
| Linux x64 | `gh_linux_amd64` | `glab_linux_amd64` |
| Windows x64 | `gh_windows_amd64.exe` | `glab_windows_amd64.exe` |

**Location:** `$APP_RESOURCES/cli/gh` and `$APP_RESOURCES/cli/glab`. Accessed via Tauri's `path::resolve()` for the resource directory.

**License compliance:** Include `gh-LICENSE` and `glab-LICENSE` (MIT) in the app bundle alongside the binaries.

**Future:** When auto-update system is added (tauri-plugin-updater), update gh/glab binaries alongside the app.

### Authentication Flow

**Primary flow (CLI-based):**

1. User clicks "Connect" for GitHub or GitLab in Settings
2. App runs `gh auth login` or `glab auth login` via the bundled CLI (interactive, opens browser for OAuth)
3. On success, app extracts the token: `gh auth token` / `glab config get token`
4. Token is stored in BeardGit's encrypted credential store (same as current PAT flow)
5. Token is used for both CLI operations (MR/PR) and REST API operations (pipelines)

**Fallback flow (manual PAT):**

- Keep existing PAT entry in Settings as fallback for environments where CLI OAuth isn't possible (headless, restricted networks)
- PAT is used for REST API only; CLI operations use `--token` flag

**Token refresh:** CLI handles token refresh automatically for OAuth tokens. PATs don't expire unless revoked.

### Operation Routing

| Operation | Method | Reason |
|---|---|---|
| List MRs/PRs | CLI (`gh pr list`, `glab mr list`) | Consistent with MR/PR workflow |
| Get MR/PR detail | CLI (`gh pr view`, `glab mr view`) | Includes review status, checks |
| Create MR/PR | CLI (`gh pr create`, `glab mr create`) | Handles template, draft, labels |
| Edit MR/PR | CLI (`gh pr edit`, `glab mr edit`) | Title, description, reviewers |
| Merge MR/PR | CLI (`gh pr merge`, `glab mr merge`) | Merge strategy options |
| Close MR/PR | CLI (`gh pr close`, `glab mr close`) | |
| Approve/review | CLI (`gh pr review`, `glab mr approve`) | |
| MR/PR comments | CLI (`gh pr comment`, `glab mr note`) | |
| MR/PR diff | CLI (`gh pr diff`, `glab mr diff`) | Raw diff for CodeMirror |
| List pipelines | REST API (existing) | Already implemented, polling-based |
| Pipeline detail | REST API (existing) | Already implemented |
| Job logs | REST API (existing) | Already implemented |

## Rust Changes

### New crate: `cli-provider` (`crates/cli-provider/`)

New crate for CLI-based provider operations. Separate from existing `gitlab-api`/`github-api` crates since the execution model is different (subprocess vs HTTP).

**`CliProvider` struct:**

```rust
pub struct CliProvider {
    kind: ProviderKind,
    binary_path: PathBuf,
    repo_path: PathBuf,
}
```

**Core types:**

```rust
#[derive(Clone, Debug, Serialize)]
pub struct MrPr {
    pub number: u64,
    pub title: String,
    pub state: MrPrState,
    pub author: String,
    pub source_branch: String,
    pub target_branch: String,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
    pub is_draft: bool,
    pub labels: Vec<String>,
    pub reviewers: Vec<String>,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MrPrState {
    Open,
    Closed,
    Merged,
}

#[derive(Clone, Debug, Serialize)]
pub struct MrPrDetail {
    pub mr_pr: MrPr,
    pub body: String,
    pub comments: Vec<MrPrComment>,
    pub review_status: ReviewStatus,
    pub checks_passing: Option<bool>,
    pub merge_conflicts: bool,
    pub mergeable: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct MrPrComment {
    pub id: u64,
    pub author: String,
    pub body: String,
    pub created_at: String,
    pub path: Option<String>,       // file path for inline comments
    pub line: Option<u32>,          // line number for inline comments
    pub is_system: bool,            // system-generated (merge, label changes)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Approved,
    ChangesRequested,
    Pending,
    NoReviews,
}

#[derive(Clone, Debug, Serialize)]
pub struct MrPrDiffFile {
    pub path: String,
    pub status: String,             // "added", "modified", "deleted", "renamed"
    pub old_path: Option<String>,   // for renames
    pub diff: String,               // raw unified diff
    pub additions: u32,
    pub deletions: u32,
}
```

**Functions:**

```rust
impl CliProvider {
    /// Detect bundled CLI binary path for the given provider.
    pub fn new(kind: ProviderKind, repo_path: &Path, app_handle: &AppHandle) -> Result<Self, CliError>

    /// Check if CLI is available and authenticated.
    pub fn check_auth(&self) -> Result<String, CliError>  // returns username

    /// Run interactive OAuth login. Spawned as a background task.
    pub fn login(&self) -> Result<(), CliError>

    /// Extract auth token from CLI config.
    pub fn get_token(&self) -> Result<String, CliError>

    // -- MR/PR operations --

    /// List MRs/PRs with optional filters.
    pub fn list_mr_pr(&self, state: Option<MrPrState>, limit: u32) -> Result<Vec<MrPr>, CliError>

    /// Get full detail for a single MR/PR.
    pub fn get_mr_pr(&self, number: u64) -> Result<MrPrDetail, CliError>

    /// Get diff for a MR/PR as list of file diffs.
    pub fn get_mr_pr_diff(&self, number: u64) -> Result<Vec<MrPrDiffFile>, CliError>

    /// Create a new MR/PR.
    pub fn create_mr_pr(&self, opts: CreateMrPrOpts) -> Result<MrPr, CliError>

    /// Edit an existing MR/PR.
    pub fn edit_mr_pr(&self, number: u64, opts: EditMrPrOpts) -> Result<(), CliError>

    /// Merge a MR/PR.
    pub fn merge_mr_pr(&self, number: u64, strategy: MergeStrategy) -> Result<(), CliError>

    /// Close a MR/PR without merging.
    pub fn close_mr_pr(&self, number: u64) -> Result<(), CliError>

    /// Approve / request changes / comment on a MR/PR.
    pub fn review_mr_pr(&self, number: u64, action: ReviewAction, body: Option<&str>) -> Result<(), CliError>

    /// Add a comment to a MR/PR (general or inline on a file/line).
    pub fn comment_mr_pr(&self, number: u64, body: &str, path: Option<&str>, line: Option<u32>) -> Result<(), CliError>
}

pub struct CreateMrPrOpts {
    pub title: String,
    pub body: String,
    pub source_branch: String,
    pub target_branch: String,
    pub is_draft: bool,
    pub labels: Vec<String>,
    pub reviewers: Vec<String>,
}

pub struct EditMrPrOpts {
    pub title: Option<String>,
    pub body: Option<String>,
    pub target_branch: Option<String>,
    pub labels: Option<Vec<String>>,
    pub reviewers: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    Merge,
    Squash,
    Rebase,
}

#[derive(Clone, Debug)]
pub enum ReviewAction {
    Approve,
    RequestChanges,
    Comment,
}
```

**CLI execution pattern:**

All CLI calls use `--json` flag (both `gh` and `glab` support JSON output) for structured parsing. Commands are run with the repo path as working directory so the CLI auto-detects the remote.

```rust
fn run_cli(&self, args: &[&str]) -> Result<String, CliError> {
    let output = Command::new(&self.binary_path)
        .args(args)
        .current_dir(&self.repo_path)
        .output()?;
    // parse stdout, check exit code
}
```

### app-core commands (`crates/app-core/src/commands.rs`)

New commands delegating to `CliProvider`:

```rust
#[tauri::command]
pub fn check_cli_auth(state: State<AppState>) -> Result<String, String>

#[tauri::command]
pub fn cli_login(state: State<AppState>, task_manager: State<Arc<TaskManager>>) -> Result<u64, String>

#[tauri::command]
pub fn extract_cli_token(state: State<AppState>) -> Result<String, String>

#[tauri::command]
pub fn list_mr_pr(state: State<AppState>, state_filter: Option<String>, limit: Option<u32>) -> Result<Vec<MrPr>, String>

#[tauri::command]
pub fn get_mr_pr(state: State<AppState>, number: u64) -> Result<MrPrDetail, String>

#[tauri::command]
pub fn get_mr_pr_diff(state: State<AppState>, number: u64) -> Result<Vec<MrPrDiffFile>, String>

#[tauri::command]
pub fn create_mr_pr(state: State<AppState>, opts: CreateMrPrOpts) -> Result<MrPr, String>

#[tauri::command]
pub fn edit_mr_pr(state: State<AppState>, number: u64, opts: EditMrPrOpts) -> Result<(), String>

#[tauri::command]
pub fn merge_mr_pr(state: State<AppState>, number: u64, strategy: String) -> Result<(), String>

#[tauri::command]
pub fn close_mr_pr(state: State<AppState>, number: u64) -> Result<(), String>

#[tauri::command]
pub fn review_mr_pr(state: State<AppState>, number: u64, action: String, body: Option<String>) -> Result<(), String>

#[tauri::command]
pub fn comment_mr_pr(state: State<AppState>, number: u64, body: String, path: Option<String>, line: Option<u32>) -> Result<(), String>
```

### Auth Integration

Modify the existing auth flow in `ProviderSetup.svelte` / `auth` crate:

1. Detect if bundled CLI is present (always true for bundled app)
2. Check if CLI is already authenticated (`check_cli_auth`)
3. If yes: extract token, store in credential store, connect provider
4. If no: offer "Login with GitHub/GitLab" button → runs CLI OAuth → extract token → store → connect
5. Existing PAT entry remains as "Advanced" / fallback option

## Frontend Changes

### Types (`src/lib/types/index.ts`)

Add all MR/PR types matching Rust structs: `MrPr`, `MrPrState`, `MrPrDetail`, `MrPrComment`, `ReviewStatus`, `MrPrDiffFile`, `CreateMrPrOpts`, `EditMrPrOpts`, `MergeStrategy`.

### IPC (`src/lib/api/tauri.ts`)

Add all MR/PR wrapper functions matching the commands above.

### Store (`src/lib/stores/mr-pr.ts`)

```typescript
export const mrPrList = writable<MrPr[]>([]);
export const selectedMrPr = writable<MrPrDetail | null>(null);
export const mrPrDiff = writable<MrPrDiffFile[]>([]);
export const mrPrFilter = writable<"open" | "closed" | "merged" | "all">("open");

export async function loadMrPrList(): Promise<void>
export async function loadMrPrDetail(number: number): Promise<void>
export async function loadMrPrDiff(number: number): Promise<void>
```

Polling: 30s interval for the list when the MR/PR view is active. Detail polling: 15s when viewing a specific MR/PR.

### Sidebar View — MR/PR List (`src/lib/components/mr-pr/MrPrList.svelte`)

**Layout:** List + detail panel (same as Branches pattern).

**List item:**

```
┌──────────────────────────────────────────────┐
│ #142 Fix auth token refresh           OPEN   │
│ feature/auth → main       adolfo   2h ago    │
│ +45 -12 · 3 files · ✓ Approved              │
└──────────────────────────────────────────────┘
```

- **Row 1:** Number + title + state badge (Open=green, Merged=purple, Closed=red)
- **Row 2:** Source → target branch + author + relative time
- **Row 3:** Diff stats + review status indicator
- Draft MRs: title prefixed with `DRAFT` badge, muted styling

**Filter bar:** Tabs for Open / Closed / Merged / All

**Action buttons in header:**
- "Create MR/PR" button → opens create dialog

### Detail Panel (`src/lib/components/mr-pr/MrPrDetail.svelte`)

**Layout:** Scrollable detail with sections.

**Sections:**

1. **Header:** Title (editable inline), state badge, number, author, timestamps
2. **Metadata:** Source → target branch, labels, reviewers (editable), draft toggle
3. **Description:** Markdown-rendered body (editable)
4. **Actions bar:**
   - Approve / Request Changes / Comment buttons
   - Merge button (with strategy dropdown: merge/squash/rebase)
   - Close button
   - Edit button
5. **Changed Files:** File list with diff stats — click to view diff
6. **Comments/Activity:** Thread of comments and activity events (approvals, label changes, pushes)

### Diff Review (`src/lib/components/mr-pr/MrPrDiffView.svelte`)

Reuses the existing `DiffEditor` component with an added comment layer:

**Additions to DiffEditor:**
- Inline comment markers in the gutter — click to add a comment at that line
- Existing comment threads shown as collapsible blocks between diff lines
- Comment input: text area with Submit/Cancel buttons
- Comments show author, timestamp, body

**File navigation:**
- File tree/list on the left (reuse `FileChangeList` pattern)
- Click file to load its diff in the editor
- Badge per file showing comment count

### Graph Integration

**MR/PR badges on commits:**

When a commit is part of an open MR/PR, show a badge in the graph view (similar to ref badges):

- Badge text: `!142` (GitLab) or `#142` (GitHub)
- Badge color: accent-purple
- Click badge → navigate to MR/PR detail view

**Implementation:** When MR/PR list is loaded, build a map of `commit_oid → MR/PR number`. Pass to `renderGraph` as an additional overlay. Lookup is by the source branch HEAD commit.

### Create MR/PR Dialog (`src/lib/components/mr-pr/CreateMrPrDialog.svelte`)

**Layout:**

```
╔══════════════════════════════════════════════════════╗
║  Create Merge Request                            ✕   ║
╠══════════════════════════════════════════════════════╣
║                                                      ║
║  Source branch:  feature/auth  (current)             ║
║  Target branch:  ▼ main                              ║
║                                                      ║
║  Title: ____________________________________         ║
║                                                      ║
║  Description:                                        ║
║  ┌────────────────────────────────────────────┐      ║
║  │                                            │      ║
║  │                                            │      ║
║  └────────────────────────────────────────────┘      ║
║                                                      ║
║  ☐ Draft                                             ║
║  Labels:     ▼ Select labels...                      ║
║  Reviewers:  ▼ Select reviewers...                   ║
║                                                      ║
║                     [Cancel]  [Create]               ║
╚══════════════════════════════════════════════════════╝
```

- Source branch defaults to current HEAD branch
- Target branch dropdown (fetched from remote branches, defaults to main/master)
- Title auto-filled from branch name or last commit message
- Description: multi-line text area
- Draft toggle
- Labels and reviewers: multi-select dropdowns (fetched from project)

### Settings Update (`ProviderSetup.svelte`)

Rework the connection flow:

1. **Primary:** "Login with GitHub" / "Login with GitLab" button → CLI OAuth flow
2. **Status:** Show connected state with username, provider icon
3. **Fallback:** Collapsible "Advanced: Manual PAT" section (existing flow)
4. **Disconnect:** Button to clear stored credentials

## i18n Keys

- `mr_pr_title` — "Merge Requests" / "Pull Requests" (based on provider)
- `mr_pr_create` — "Create {type}"
- `mr_pr_open` — "Open"
- `mr_pr_closed` — "Closed"
- `mr_pr_merged` — "Merged"
- `mr_pr_draft` — "Draft"
- `mr_pr_approve` — "Approve"
- `mr_pr_request_changes` — "Request Changes"
- `mr_pr_merge` — "Merge"
- `mr_pr_close` — "Close"
- `mr_pr_squash_merge` — "Squash and Merge"
- `mr_pr_rebase_merge` — "Rebase and Merge"
- `mr_pr_comment` — "Comment"
- `mr_pr_add_comment` — "Add comment..."
- `mr_pr_reviewers` — "Reviewers"
- `mr_pr_labels` — "Labels"
- `mr_pr_source` — "Source branch"
- `mr_pr_target` — "Target branch"
- `mr_pr_description` — "Description"
- `mr_pr_changes` — "Changed files"
- `mr_pr_activity` — "Activity"
- `mr_pr_no_mrs` — "No merge requests"
- `mr_pr_login` — "Login with {provider}"
- `mr_pr_login_pat` — "Advanced: Manual PAT"
- `mr_pr_files_changed` — "{count} files changed"

## Build System Changes

### CLI Binary Management

**Download during build:** Add a build script or CI step that downloads the correct `gh` and `glab` binaries for each platform target and places them in `src-tauri/resources/cli/`.

**Tauri resource config (`tauri.conf.json`):**

```json
{
  "app": {
    "bundle": {
      "resources": [
        "resources/cli/*"
      ]
    }
  }
}
```

**Binary resolution at runtime:**

```rust
fn bundled_cli_path(app: &AppHandle, cli_name: &str) -> PathBuf {
    app.path().resource_dir()
        .expect("resource dir")
        .join("cli")
        .join(cli_name)
}
```

### CI Pipeline Updates

Add CLI download step to build pipeline for each platform target. Pin specific versions of gh and glab for reproducible builds.

## Scope Boundaries

**In scope:**
- Bundle gh and glab CLI binaries with the app
- CLI-based OAuth login flow
- Token extraction from CLI for REST API auth
- Full MR/PR CRUD: list, view, create, edit, merge, close
- Code review: approve, request changes, inline comments
- MR/PR diff viewing in existing CodeMirror editor with comment layer
- MR/PR badges on graph commits
- Create MR/PR dialog with branch selection, labels, reviewers
- File-level and line-level inline comments

**Out of scope:**
- Auto-update of bundled CLIs (future — with tauri-plugin-updater)
- MR/PR templates (future — read from `.github/` or `.gitlab/` templates)
- MR/PR CI status integration (future — combine with existing pipeline view)
- Conflict resolution within MR/PR view (use existing merge editor after local checkout)
- MR/PR notifications / inbox
- Suggested changes (GitHub's "suggestion" feature in reviews)
- Multi-remote support (assumes single origin)
