# Changelog

All notable changes to BeardGit are documented here. Format follows [keepachangelog.com](https://keepachangelog.com).

## [0.1.4] - 2026-04-09 — UI Polish, Layout Consistency & Bug Fixes

**3-Way Merge Editor (IntelliJ-style)**

- Full 3-panel layout: Theirs (Incoming) | Result | Ours (Current)
- Custom 3-way diff engine with LCS-based line alignment and chunk classification
- Non-conflicting changes auto-applied to the result on open
- Conflict placeholder lines in center with accept/ignore buttons on each side
- SVG bezier connector curves between panels linking conflict regions visually
- Hybrid curves: filled bezier when sparse, thin connector lines when dense (> 4 conflicts)
- Dynamic connector gap width (24px normal, 40px for many conflicts)
- Color scheme: green (added), purple (conflict), blue (center placeholder), active highlight (brighter)
- Chunk-aware scroll sync: center drives side panels based on line mapping, not proportional
- Side panel wheel events redirected to center for consistent behavior
- Smooth scroll animations on side panels during sync
- SVG connectors update on scroll, accept/ignore, undo, and window resize
- Undo support: Cmd/Ctrl+Z undoes accept/ignore operations, toolbar undo button
- Toggle line numbers button (# icon) for all three panels
- Prev/Next conflict navigation scrolls all panels aligned with active highlight
- Mark Resolved button: grey when disabled, green when all conflicts resolved
- Warning popup when resolving with conflict markers still present
- Cancel button with red destructive styling
- Syntax highlighting in all panels (language-aware via filename)

**Merge Request / Pull Request Improvements**

- List layout aligned with pipeline section pattern (3-column horizontal rows with state icon, title, time)
- Removed filter tabs (Open/Closed/Merged/All), replaced with SearchBar state filter (default: state:open)
- Added search/filter bar with state, author, branch, and label filters
- Markdown rendering in descriptions and comments (snarkdown + allowlist-based XSS sanitizer, links open externally)
- Redesigned merge action buttons: split-button with dropdown menu for merge strategy (merge/squash/rebase)
- Added refresh button and "no provider" empty state
- Provider readiness guard prevents empty list on startup

**Layout Consistency**

- Migrated Reflog view to SplitView (resizable sidebar, consistent with Tags/Stash/Branches/MR)
- Migrated Pipelines view to SplitView (replaces custom resize logic in +page.svelte)
- Pipeline job log pane is now resizable with a drag handle and has a close button
- Standardized icon-only buttons across all views: 14px, no border, color-only hover (refresh, close, nav buttons)
- Worktree, CommitDetail, DiffEditor, StagingDiffEditor, BlameView buttons all aligned
- Tag push button uses green hover (from theme --accent-green) consistently in both list and detail
- Worktree delete button: same color as others by default, red highlight on hover only
- Graph header separator line now reaches full width (border on container, not SearchBar)

**Git Config Editor**

- Empty values show italic "empty" label in light grey instead of em dash
- Clicking an empty field and typing nothing no longer saves an empty value
- Tooltips use i18n keys instead of hardcoded English

**Task System**

- Task popover now appears correctly (fixed position + click-outside race condition)
- Task output loaded from backend on selection (fixes empty output for completed tasks)
- Panel output shows executed command at top ($ git fetch origin)
- Three distinct empty states: "Select a task", "No output", output content
- Correct NerdFont icons for expand/collapse/close buttons
- Removed output preview from popover (kept in full panel only)

**Authentication**

- GitHub/GitLab CLI OAuth login disabled until ghostty terminal integration (PAT-only for now)
- OAuth errors now shown to user instead of silent fallback to PAT

**Keyboard Shortcuts**

- `?` shortcut now works globally (even when editor is focused)
- Fixed shift-key matching for shortcuts like `?` that inherently need Shift
- Help overlay: Escape key closes the popup, larger fonts, bigger close button
- Sidebar highlight syncs with keyboard navigation (Cmd+1-6)

**Other Fixes**

- Reflog empty-state message properly centered
- Hardcoded hex colors replaced with theme variables (--accent-green, --accent-red) in tag buttons
- Markdown sanitizer uses allowlist approach; links get target="_blank"
- MR merge dropdown closes on click-outside
- Conflict marker regex handles Windows \r\n line endings
- MR filtered-empty state shows "No results match your filter" instead of generic message
- MR and reflog state cleared on project switch (prevents stale data)
- CreateMrPrDialog backdrop changed to button for a11y compliance
- All svelte-check warnings resolved (0 errors, 0 warnings)

## [0.1.3] - 2026-04-08 — Phase 3: Power Features + CLI Integration

**Task History Popup**

- Enriched TaskInfo with command string, start timestamp, and exit code
- Always-clickable status bar task area — visible even when no tasks are running
- Two-line card popup: colored status bar (green/red/orange/gray), label, command, duration, relative time
- Click any task to open full output panel

**Keyboard Shortcuts**

- Central shortcut registry with platform-aware modifiers (⌘ on macOS, Ctrl on Windows/Linux)
- Cmd+1-6 for view navigation, Cmd+Tab/Shift+Tab for tabs, Cmd+W to close tab
- Cmd+Shift+F/L/P for Fetch/Pull/Push, Cmd+Shift+S/U for Stage/Unstage all
- J/K for graph commit navigation, Home/End for first/last commit, / for search
- `?` opens cheat sheet overlay with all shortcuts grouped by category

**Reflog Viewer**

- New sidebar view showing HEAD reflog entries with action-specific icons (commit, checkout, rebase, reset, merge, pull)
- Detail panel reuses CommitDetail with "Show in Graph" navigation button
- Context menu: checkout commit, create branch, reset (soft/mixed/hard), copy SHA

**Clean (Untracked File Removal)**

- "Clean" button in staging area when untracked files exist
- Dialog with filter toggles: include directories, include ignored, only ignored
- Per-file checkboxes with select/deselect all
- Per-file "Delete untracked file" from right-click context menu
- Destructive action warnings on all delete operations

**Git Config Editor**

- New Settings section: two-column table showing Local (project) and Global (user) config side by side
- Dropdown selectors for known enum-type keys (core.autocrlf, pull.rebase, push.default, etc.)
- Free text input for all other keys
- Inline editing with Enter to save, Escape to cancel
- Add new entries, unset existing keys, filter by key name
- Collapsible read-only System config section

**Gitignore Management**

- Quick "Add to .gitignore" from untracked file context menu with smart pattern suggestions (filename, *.ext, exact path, directory/)
- Full CodeMirror editor in Settings with save/revert and dirty state tracking
- Basic syntax highlighting for comments and negation patterns

**Patch Management**

- Create patches from commits (graph context menu → native save dialog)
- Create patches from working tree changes (staged or unstaged)
- Apply patches with dry-run preview showing per-file stats
- Three-way merge fallback for conflicting patches — integrates with existing merge editor

**Submodules**

- New sidebar view listing all submodules with status badges (Uninitialized, Clean, Outdated, Dirty)
- Init, update (background task), deinit operations
- "Open in Tab" — opens submodule as a full project tab with all BeardGit features
- Context menu with all operations + copy path/URL
- "Update All" header button for batch update

**MR/PR Management (GitHub + GitLab)**

- New `cli-provider` crate wrapping bundled `gh` and `glab` CLIs (both MIT licensed)
- CLI OAuth as primary auth flow — opens browser, extracts token, stores in encrypted credential store
- PAT entry remains as fallback for restricted environments
- Full CRUD: list, view, create, edit, merge (merge/squash/rebase), close
- Code review: approve, request changes, general + inline comments
- MR/PR badges on graph commits for branches with open MR/PRs (purple pills)
- Create dialog with source/target branch, title, description, draft toggle, labels, reviewers
- Filter tabs: Open / Closed / Merged / All

**Windows DPI & Zoom Fixes**

- Replaced CSS `zoom` with Tauri native `webview.setZoom()` — fixes blurry fonts and layout overflow at >100% UI scale on Windows
- Added `-webkit-font-smoothing: antialiased` and `text-rendering: optimizeLegibility` for crisper text rendering
- Canvas graph detects DPI changes when moving between screens and re-renders at correct resolution
- Fixed canvas subpixel blurriness at fractional DPR values

**Graph UX Improvements**

- Row hover highlight (subtle transparent overlay)
- Standard cursor instead of pointer hand on graph rows
- Increased canvas font sizes by 1px across all text elements
- Fixed column resize hit zone misaligned with separator lines

**Tauri Native Migration**

- Replaced `-webkit-app-region: drag` CSS with `data-tauri-drag-region` HTML attribute
- Added `core:webview:allow-set-webview-zoom` capability permission

**Performance & Code Quality**

- All MR/PR CLI commands run on `spawn_blocking` — never block the Tauri async runtime
- Canvas draw batched with `requestAnimationFrame` via `scheduleDraw()` helper
- Keyboard shortcut handler uses `get()` instead of subscribe/unsubscribe per keydown
- Reflog auto-refresh debounced (300ms) on repo-changed events
- Extracted shared utilities: `shortOid()`, `configure_no_window()`, `run_blocking()` helper
- Added `GitError::CliError` variant — CLI failures no longer misuse `RepoNotFound`
- Stringly-typed MR/PR params replaced with proper enum types (`MrPrState`, `MergeStrategy`)
- Error handling added to CleanDialog and GitConfigSettings (visible error messages)
- `formatRelativeTimeMs` delegates to `formatRelativeTimeUnix` instead of duplicating

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

**Graph Columns**

- Resizable columns — drag column separators to adjust width (min 50px), persisted across sessions
- New Email column (hidden by default) showing commit author email
- SHA column now hidden by default (toggleable from Columns dropdown)
- Column visibility and widths persisted to settings.json via new Tauri commands

**10 New Built-in Themes + Complementary Pairing**

- Dracula, One Dark Pro, Catppuccin Mocha, Catppuccin Latte, Nord, Tokyo Night, Solarized Dark, Solarized Light, Gruvbox Dark, Monokai Pro
- Total: 14 built-in themes (10 dark, 4 light)
- Complementary theme pairing for OS auto-switch — each theme maps to a light/dark counterpart so toggling OS appearance picks the right pair (e.g., Catppuccin Mocha ↔ Catppuccin Latte)

**Performance**

- `Arc<str>` for commit OIDs in graph-builder — eliminates ~10 String clones per commit in the 100K+ commit hot path
- GitLab stage grouping optimized from O(n²) to O(n) via HashMap index

**Code Quality & Deduplication**

- Replaced 30+ hardcoded CSS color values with theme variables across 5 components
- Added `--overlay-accent-*` CSS variables for consistent overlay theming
- Consolidated 3 inline date formatters into shared `formatDate()`/`formatDateTime()` utilities
- Replaced manual debounce in TagList with shared `debounce()` utility
- Deduplicated `normalize_github_url` (auth crate now imports from github-api)

**Bug Fixes**

- Fixed stale detail panels when switching repository tabs — graph, branch, tag, stash, blame, and worktree state now fully cleared on tab switch
- Conflict status now refreshed on repo tab switch
- Branch commit list not taking full available width (missing `min-width: 0`)
- Diff close button hidden when file path is too long (added overflow handling + `flex-shrink: 0`)
- npm security audit: resolved high-severity vite vulnerability, overrode cookie to ^0.7.0 (0 vulnerabilities)

**Settings**

- Removed Repository section (remote management) — will return with gh/glab CLI integration

**CI/CD**

- Release pipeline auto-syncs version from git tag — no manual version bumps needed
- Strips non-numeric pre-release suffixes for Windows MSI compatibility (e.g., `v0.1.2-beta` → `0.1.2`)

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
