# BeardGit — Project Status

## Phase 3: Power Features + CLI Integration — Complete (v0.1.3)

### Task History Popup

- [x] Enriched TaskInfo with command, started_at_ms, exit_code
- [x] Always-clickable status bar task area (history icon when idle)
- [x] Two-line card popup with colored status bars, sorted running-first

### Keyboard Shortcuts

- [x] Central shortcut registry with platform-aware modifiers (⌘/Ctrl)
- [x] ~20 shortcuts: view navigation, tab management, git ops, graph nav
- [x] Cheat sheet overlay via `?` key

### Reflog Viewer

- [x] New sidebar view with action-specific icons
- [x] Detail panel reusing CommitDetail + Show in Graph
- [x] Context menu with recovery actions (checkout, branch, reset)

### Clean

- [x] Dry-run preview with filter toggles (dirs, ignored, only-ignored)
- [x] Per-file checkboxes with select all, destructive warnings
- [x] Per-file delete from untracked file context menu

### Git Config Editor

- [x] Two-column table (Local + Global) in Settings
- [x] Dropdown selectors for known enum keys, free text for others
- [x] Add/unset entries, filter by key, collapsible system section

### Gitignore Management

- [x] Quick "Add to .gitignore" from context menu with smart patterns
- [x] Full CodeMirror editor in Settings with save/revert

### Patch Management

- [x] Create patches from commits (graph) and working tree (changes)
- [x] Apply with dry-run preview + three-way merge fallback

### Submodules

- [x] Sidebar view with status badges (uninit/clean/outdated/dirty)
- [x] Init, update (background task), deinit operations
- [x] Open submodule as full project tab

### MR/PR Management

- [x] New cli-provider crate wrapping bundled gh/glab CLIs (MIT)
- [x] CLI OAuth primary auth flow + PAT fallback
- [x] Full CRUD: list, view, create, edit, merge, close
- [x] Code review: approve, request changes, inline comments
- [x] MR/PR badges on graph commits
- [ ] Download and bundle gh/glab binaries in CI pipeline
- [ ] Auto-update bundled CLIs with app updates

### Audit Fixes

- [x] All CLI commands async with spawn_blocking
- [x] Canvas draw batched with requestAnimationFrame
- [x] Shared utilities: shortOid, configure_no_window, run_blocking
- [x] GitError::CliError variant, enum-typed MR/PR params
- [x] Error handling in CleanDialog and GitConfigSettings

---

## Phase 2: Core Workflows — Complete

### v0.1.2 (Plans 3–4 + Graph Columns + Fixes)

**Hunk + Line-Level Staging**

- [x] Stage/unstage/discard individual hunks or specific lines
- [x] StagingDiffEditor with per-hunk and per-line checkboxes
- [x] Backend patch builder → `git apply --cached`

**Blame + File History**

- [x] Blame view with per-line gutter annotations (author, OID, relative date)
- [x] File history panel with `git log --follow` and rename detection
- [x] Right-click any file → "Blame" / "File History"

**Rebase**

- [x] Non-interactive rebase from branch + graph context menus
- [x] Interactive rebase: visual commit editor with drag-to-reorder + actions (pick/squash/fixup/edit/drop)
- [x] GIT_SEQUENCE_EDITOR injection for todo list

**3-Way Merge Editor**

- [x] CodeMirror unifiedMergeView with inline accept/reject per chunk
- [x] Conflict toolbar shows clickable file list → opens merge editor
- [x] get_conflict_file_contents reads ours/theirs/base from libgit2 index
- [x] write_resolved_file writes + stages + removes conflict entries

**Graph Columns**

- [x] Resizable columns via drag (min 50px, persisted to settings.json)
- [x] Email column (hidden by default)
- [x] SHA column hidden by default
- [x] Column visibility + widths persisted across sessions

**Theme Improvements**

- [x] 10 new built-in themes (14 total: 10 dark, 4 light)
- [x] Complementary theme pairing for OS auto-switch

**Bug Fixes**

- [x] Fixed stale detail panels on repo tab switch (graph, branches, tags, stashes, blame, worktrees)
- [x] Conflict status refreshed on repo tab switch
- [x] Removed Repository settings section (deferred to gh/glab CLI integration)

**CI/CD**

- [x] Release pipeline auto-syncs version from git tag
- [x] Windows MSI: strips non-numeric pre-release suffixes

### v0.1.1 (Plans 1–2)

**CodeMirror 6 Editor Engine**

- [x] Replaced custom diff viewer with CodeMirror 6 (syntax highlighting, 16 languages)
- [x] Side-by-side diff with collapsed unchanged regions (@codemirror/merge)
- [x] Line numbers in all editor and diff views
- [x] Language auto-detection from file extension with lazy-loaded grammars
- [x] Theme bridge: TOML themes → CodeMirror extensions (chrome + syntax tokens)

**Core Git Operations**

- [x] Revert commits from graph context menu with confirmation
- [x] Amend last commit via toggle in staging area (pre-fills HEAD message)
- [x] Reset (soft/mixed/hard) from graph context menu with mode-specific warnings
- [x] Worktree management: list, create, remove with sidebar + tab integration

**Theme System Improvements**

- [x] Simplified TOML themes: only [meta] + [colors] required (14 lines)
- [x] Graph, editor, syntax colors auto-derived from 12 base colors
- [x] Optional [graph] and [editor] overrides for fine-tuning

**UI Improvements**

- [x] UI Scale setting (80%–150%) in Settings > Appearance
- [x] Ref badges rotate through accent colors (hash-based)

**Performance & Windows Fixes**

- [x] All 22 CLI-backed commands non-blocking (async + spawn_blocking)
- [x] CREATE_NO_WINDOW flag on Windows (no CMD flash)

---

## MVP (v0.1.0) — Complete

All Phase 1 features are implemented and tested.

### Completed Features

**Git Graph**

- [x] Visual commit graph with canvas rendering (100K+ commits via virtual scroll)
- [x] Lane-segment + merge-curve architecture
- [x] Sync-state-aware line styles: thick (pushed), thin (local-only), dashed (fetched)
- [x] Lane recycling at MAX_LANES=8 cap with arrow indicators
- [x] Author bold — your commits shown in bold
- [x] HEAD branch highlighting — thicker line + subtle background tint
- [x] Lane click selection and hover feedback
- [x] Clickable parent OIDs in commit detail
- [x] Collapsible linear chains (runs of 3+ linear commits)
- [x] Configurable columns (author, date) with resize

**Git Operations**

- [x] Staging area with file-level stage/unstage and commit
- [x] Branch management: create, delete, checkout, merge, cherry-pick
- [x] Branch view with folder tree, commit history, context menu, and inline commit detail panel
- [x] Stash management: push, pop, apply, drop, per-file apply, diff preview
- [x] Tag management: paginated list, create (annotated + lightweight), delete, push
- [x] Fetch, Pull, Push as background tasks with live output streaming
- [x] Auto-refresh graph and branches after remote operations complete
- [x] Conflict detection: MERGING / REBASING / CHERRY-PICKING / REVERTING state
- [x] ConflictToolbar with Abort/Continue actions and conflict marker highlighting

**Diff Viewer**

- [x] Side-by-side file diff panel with word-level diff highlighting and vertical resize handle
- [x] Clickable ref badges on merge commits showing changed files
- [x] Right-click context menu on changed files (stage, unstage, copy path)
- [x] Reusable CommitDetail component (used in graph, branch, and tag views)

**CI Pipeline Integration**

- [x] Multi-provider support: GitLab REST v4 + GitHub REST API via CiProvider trait
- [x] Unified pipeline list with real-time polling (15s list, 10s detail, 3s job logs)
- [x] Job log viewer with full ANSI color rendering (256-color, true-color, bold/dim/italic/underline)
- [x] CI log preprocessing — strips timestamps, stream codes, section markers; adds line numbers
- [x] Server-side filtering by branch, source, and status
- [x] Auto-detect provider from git remote URL

**Multi-Project Tabs**

- [x] Open multiple repos as tabs with lazy loading
- [x] Tab persistence across app restarts
- [x] Starship-style title bar with git status summary (project - branch [↑↓+!?])
- [x] Lightweight metadata for background tabs (branch, change count)

**Background Task System**

- [x] task-runner crate with async spawn/cancel and output streaming via TaskEventSink trait
- [x] Status bar indicator with running/failed states
- [x] Task popover for quick glance, expandable panel for full log viewer
- [x] Filesystem watcher with debounced auto-refresh on repo-changed events

**Authentication**

- [x] Encrypted credential storage (AES-256-GCM, machine-derived key)
- [x] PAT validation for GitLab and GitHub
- [x] Multi-provider auto-reconnect on app startup

**Internationalization**

- [x] English (en-US) and Spanish (es-ES) via Paraglide.js v2
- [x] Compile-time typed message functions
- [x] Language selector in Settings > Appearance

**Storage**

- [x] SQLite database with versioned schema
- [x] JSON config with multi-provider migration support
- [x] TOML theme system (built-in + user-installed)

**UI/UX**

- [x] Custom app icon (BeardGit glasses + beard + git diamond)
- [x] Pill-shaped tabs merged into toolbar bar
- [x] Fira Code monospace font with ligatures
- [x] Symbols Nerd Font Mono for icons throughout the UI
- [x] Responsive viewport-relative layouts with clamp()/min()
- [x] Minimum window size 900x600
- [x] Shared components: SplitView, FileChangeList, CommitDetail, ConfirmDialog, SearchBar, ContextMenu

---

## Next Steps

### Phase 3 Remaining

- Bisect (visual bisect workflow)
- Bundle gh/glab binaries in CI build pipeline per platform
- Auto-update bundled CLIs with app updates

### Phase 4: Developer Experience

- Embedded terminal (libghostty)
- Full theme system (CodeMirror + libghostty theme support)

### Infrastructure

- Auto-update system (tauri-plugin-updater) — update app, libgit, git, gh, glab
- Crash reporting / telemetry (opt-in)

### Code Quality (Deferred from Audit)

- Split `commands.rs` (3148 lines) into feature-based modules
- Extract generic `<List>` component (saves ~1500 LOC across 8 components)
- Extract shared dialog CSS to `src/lib/styles/dialog.css`
- Extract store `fetchIntoStore` helper
- CLI provider JSON parsing deduplication (GitHub/GitLab parsers)

---

## Branch Strategy

- `main` — stable releases
- `beta` — development, beta updates
- `feature/*` — new features
- `bugfix/*` — bug fixes
