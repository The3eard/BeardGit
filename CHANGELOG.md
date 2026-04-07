# Changelog

All notable changes to BeardGit are documented here. Format follows [keepachangelog.com](https://keepachangelog.com).

## [0.1.2] - 2026-04-07

**Hunk + Line-Level Staging**

- Stage, unstage, or discard individual hunks or specific lines within a hunk
- StagingDiffEditor with per-hunk and per-line checkboxes, select all/deselect all
- Backend builds unified diff patches from selections and applies via `git apply --cached`
- Discard with confirmation dialog (destructive action)

**Blame + File History**

- Blame view with per-line gutter annotations (author, OID, relative date)
- Commit grouping in gutter — consecutive lines from same commit share annotation block
- Click OID in gutter to reload blame at that commit
- File history panel with `git log --follow` — shows all commits that touched the file
- Rename detection — shows "renamed from" badge when file was moved
- Click any commit in history to view blame at that point in time
- Right-click any file in staging area or commit detail → "Blame" / "File History"

**Rebase**

- Non-interactive rebase from branch context menu ("Rebase onto this branch") and graph context menu ("Rebase current onto here")
- Confirmation dialog before rebase; conflicts route to merge editor automatically

**Interactive Rebase**

- Visual commit list editor from graph context menu ("Interactive rebase from here")
- Per-commit action dropdown: pick, squash, fixup, edit, drop
- Drag-to-reorder commits with color-coded left border per action
- Drop action shows strikethrough with reduced opacity
- Footer legend explaining each action
- Backend uses `GIT_SEQUENCE_EDITOR` to inject pre-built todo list

**3-Way Merge Editor**

- CodeMirror `unifiedMergeView` with ours as editable content, base as reference
- Inline accept/reject controls per changed chunk
- Prev/Next conflict navigation buttons
- "Mark Resolved" writes content to disk and stages the file
- Conflict toolbar now shows expandable clickable file list → opens merge editor
- Activated during any conflict operation (merge, rebase, cherry-pick, revert)
- Backend: `get_conflict_file_contents` reads ours/theirs/base from libgit2 index stages

---

## [0.1.1] - 2026-04-07

**CodeMirror 6 Editor Engine**

- Replaced custom diff viewer with CodeMirror 6 — syntax highlighting for 16 languages (JS, TS, Rust, Python, CSS, HTML, JSON, YAML, Markdown, Java, Go, C/C++, SQL, XML, and more)
- Side-by-side diff view with collapsed unchanged regions via @codemirror/merge
- Line numbers in all editor and diff views
- Language auto-detection from file extension with lazy-loaded grammars

**Core Git Operations**

- Revert commits from graph context menu with confirmation dialog
- Amend last commit via toggle in staging area (pre-fills HEAD message)
- Reset to any commit: soft (keep staged), mixed (unstage), hard (discard all) from graph context menu
- Hard reset shows destructive warning with explicit confirmation

**Worktree Management**

- Sidebar section listing all worktrees with branch name, path, and status badges
- Create new worktrees with auto-suggested path and new/existing branch options
- Open worktree as a tab (reuses multi-project tab system)
- Remove worktrees with confirmation dialog

**Remote Management**

- Settings > Repository section showing configured remotes
- Rename and remove remotes with inline editing and confirmation

**Theme System Improvements**

- Simplified TOML themes: only `[meta]` + `[colors]` required (14 lines instead of 50+)
- Graph, editor, and syntax highlighting colors auto-derived from 12 base colors
- Optional `[graph]` and `[editor]` overrides for fine-tuning
- Syntax token colors derived from theme accent palette (keywords, strings, comments, functions, types, numbers, operators, properties)
- Updated themes README with full documentation for custom theme creators

**UI Improvements**

- UI Scale setting (80%–150%) in Settings > Appearance for font size control
- Ref badges in commit detail and graph rotate through accent colors (hash-based, deterministic)
- Fira Code font explicitly set in all CodeMirror instances

**Performance & Windows Fixes**

- All 22 git CLI-backed commands now run on background threads (async + spawn_blocking) — UI never freezes during git operations
- Added CREATE_NO_WINDOW flag on Windows to prevent CMD console flash when spawning git processes
- Covers: tags, stashes, diffs, conflict operations, remotes, worktrees

**Testing**

- Added vitest coverage configuration (@vitest/coverage-v8)
- 32 new Rust tests (theme derivation, file content, remote operations)
- 23 new frontend tests (diff utils, ref colors, editor theme, language support)
- Shared ref-colors utility extracted from duplicate implementations

---

## [0.1.0] - 2026-04-06

**Git Operations**

- Visual commit graph with canvas rendering (100K+ commits via virtual scroll)
- Staging area with file-level stage/unstage and commit
- Branch management: create, delete, checkout, merge, cherry-pick
- Branch view with folder tree, commit history, context menu, and inline commit detail panel
- Stash management: push, pop, apply, drop, per-file apply, diff preview
- Tag management: paginated list, create (annotated + lightweight), delete, push, inline file diff preview, clickable parent refs
- Side-by-side file diff panel with word-level diff highlighting and vertical resize handle
- Clickable ref badges on merge commits showing changed files
- Fetch, Pull, Push as background tasks with live output streaming
- Auto-refresh graph and branches after remote operations complete

**Conflict Detection**

- Detect MERGING / REBASING / CHERRY-PICKING / REVERTING state
- Amber status bar badge and full-width ConflictToolbar with Abort/Continue
- Conflict marker highlighting in diff viewer (ours/separator/theirs)
- Auto-refresh conflict status on repo-changed events

**Graph Features**

- Lane-segment + merge-curve architecture
- Sync-state-aware line styles: thick (pushed), thin (local-only), dashed (fetched)
- Lane recycling at MAX_LANES=8 cap with arrow indicators
- Configurable columns (author, date) with resize
- Author bold — your commits shown in bold (matches git config name/email + provider identities)
- HEAD branch highlighting — thicker line + subtle background tint
- Lane click selection — click a lane to focus it, everything else dims
- Lane hover feedback — cursor and subtle highlight when hovering over lanes
- Clickable parent OIDs — click parent SHA in commit detail to navigate
- Context menu on commits: copy SHA, copy message, create branch, cherry-pick, checkout

**CI Pipeline Integration**

- Multi-provider support: GitLab REST v4 + GitHub REST API
- Unified pipeline list with real-time polling (15s list, 10s detail, 3s logs)
- Job log viewer with full ANSI color rendering (256-color, true-color, bold/dim/italic/underline)
- CI log preprocessing — strips timestamps, stream codes, section markers; adds line numbers (GitLab + GitHub)
- Server-side filtering by branch, source, and status
- Auto-detect provider from git remote URL

**Multi-Project Tabs**

- Open multiple repos as tabs with lazy loading
- Tab persistence across app restarts
- Starship-style title bar with git status summary (ahead/behind/staged/unstaged/stash)
- "+" dropdown with recent repos and open folder

**Background Task System**

- Task manager with async spawn/cancel and output streaming
- Status bar indicator with running/failed states
- Task popover for quick glance, expandable panel for full log viewer
- rAF-batched output events to reduce GC pressure

**Theme System**

- 4 built-in themes: GitHub Dark, GitHub Light, GitLab Dark, GitLab Light
- Default follows OS light/dark preference with live reactive switching
- User-installable custom themes via TOML files in `~/.config/beardgit/themes/`
- Auto-generated README in themes directory documenting the TOML schema
- Theme selector in Settings with "Follow system theme" toggle
- All UI colors driven by CSS custom properties
- Graph canvas renderer fully themed (lane colors, badges, text, selection)
- CI status colors adapt to active theme

**Internationalization**

- English (en-US) and Spanish (es-ES) via Paraglide.js v2
- Compile-time typed message functions
- Language selector in Settings > Appearance

**Authentication**

- Encrypted credential storage (AES-256-GCM, machine-derived key via HKDF-SHA256)
- PAT validation for GitLab and GitHub
- Multi-provider auto-reconnect on app startup

**UI/UX**

- Custom app icon (BeardGit glasses + beard + git diamond)
- Pill-shaped tabs merged into toolbar bar
- Fira Code monospace font with ligatures
- Symbols Nerd Font Mono for icons throughout the UI
- Responsive viewport-relative layouts with clamp()/min()
- Minimum window size 900x600
- Right-click context menus on files and commits
- Reusable components: SplitView, FileChangeList, CommitDetail, ConfirmDialog, SearchBar, ContextMenu
- Shared CSS for consistent styling across all views
- Filesystem watcher with debounced auto-refresh

**Storage**

- SQLite database with versioned schema and commit cache
- JSON config with provider migration support
- TOML theme system (built-in + user-installed)

**CI/CD**

- GitHub Actions CI: frontend checks + Rust fmt/clippy/tests
- Multi-platform build pipeline: macOS (arm64 + x64), Linux x64, Windows x64
- Release pipeline with draft GitHub releases on version tags
- Weekly security audit (cargo audit + npm audit)
