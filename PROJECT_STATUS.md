# BeardGit — Project Status

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

### Phase 2: Core Workflows
- Rebase & Revert
- Amend commits
- 3-way merge editor (CodeMirror 6)

### Phase 3: Power Features
- Blame / file history
- Hunk-level staging
- Interactive rebase

### Phase 4: CLI Integration
- glab / gh CLI for MR/PR operations

### Phase 5: Developer Experience
- Embedded terminal (libghostty)
- Full theme system

### Infrastructure
- Auto-update system (tauri-plugin-updater)
- Keyboard shortcuts
- Crash reporting / telemetry (opt-in)

---

## Branch Strategy
- `main` — stable releases
- `beta` — development, beta updates
- `feature/*` — new features
- `bugfix/*` — bug fixes
