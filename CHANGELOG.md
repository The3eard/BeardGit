# Changelog

All notable changes to BeardGit are documented here. Format follows [keepachangelog.com](https://keepachangelog.com).

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
