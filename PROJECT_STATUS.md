# BeardGit — Project Status

## Completed

### MVP (v0.1.0)

Canvas-based git graph (100K+ commits), staging area, branch/tag/stash management, fetch/pull/push as background tasks, GitLab + GitHub CI integration with ANSI log viewer, multi-project tabs with persistence, encrypted credential store, filesystem watcher, i18n (en/es), 4 TOML themes, custom app icon, Fira Code + Nerd Font icons.

### Phase 2: Core Workflows (v0.1.1–v0.1.2)

CodeMirror 6 diff engine with syntax highlighting (16 languages), hunk/line-level staging, blame with gutter annotations, file history with rename detection, revert/amend/reset, worktree management, non-interactive + interactive rebase, 3-way merge editor, resizable graph columns, complementary theme pairing, 14 themes (10 dark, 4 light), UI scale setting, all CLI commands non-blocking.

### Phase 3: Power Features + CLI Integration (v0.1.3–v0.1.4)

Task history popup, keyboard shortcuts with cheat sheet, reflog viewer with recovery actions, clean with preview dialog, git config editor (local/global), gitignore management (context menu + CodeMirror editor), patch create/apply, submodule management with open-as-tab, MR/PR management via cli-provider crate (CRUD, review, comments, graph badges), IntelliJ-style merge editor v3 (custom diff engine, SVG bezier connectors, accept/ignore, undo, conflict navigation), auto-update system with toast notifications, multi-file selection with checkboxes, SplitView migration, performance audit fixes.

---

## Phase 4: Terminal Foundation (xterm.js)

### 4A — Terminal Core + Read-Only Views + Theme Bridge ✅

- [x] Rust PTY manager crate (`portable-pty`, shell spawn, read/write)
- [x] New `terminal` crate (isolated from Tauri, reusable library)
- [x] xterm.js Svelte component (WebGL addon, fit addon, web-links, search addons)
- [x] Shell detection and configuration (zsh, bash, powershell)
- [x] Read-only xterm.js instance pool (2 visible + 1 warm)
- [x] TaskPanel output migrated to xterm.js read-only terminal
- [x] CI JobLog migrated to xterm.js read-only terminal
- [x] Retired `ansi.ts` parser — replaced by native xterm.js rendering
- [x] Theme system redesigned: 18 base colors (bg + fg + 16 ANSI) with derived semantics
- [x] All 14 TOML themes updated with explicit ANSI palettes
- [x] Direct xterm.js ITheme mapping from base colors
- [x] Tauri commands + event bridge for terminal sessions

### 4B — Interactive Terminal Tabs + UI Improvements ✅

- [x] Terminal tabs as first-class tab type in the TabBar (unified tab model: project | terminal | composite)
- [x] Composite segmented tabs: project + linked terminal merge into one pill `[● Repo | ⌨ Terminal]`
- [x] Each segment independently clickable, closeable (hover-only ✕), and middle-click closeable
- [x] Closing a segment reverts composite to simple tab (project-only or terminal-only)
- [x] Terminal opens in-place: promoting project tab to composite, not appending at end
- [x] Terminal label adapts: "Terminal", "Claude", "Codex", "OpenCode" with brand icons and hardcoded colors
- [x] Terminal split button in actions area: left (icon) opens terminal, right (chevron) dropdown with options
- [x] Dropdown: "New terminal in ~", Claude Code (#d97757), Codex (#10a37f), OpenCode (#8b8b8b) with SVG brand logos
- [x] Standalone terminal tabs for "New terminal in ~" (not linked to any project)
- [x] Full interactive xterm.js terminal wired to Rust PTY backend (keyboard input, resize, auto-close on shell exit)
- [x] NerdFont icons render in terminal (NerdFontSymbols added to fontFamily)
- [x] Sidebar collapse toggle (icon-only mode, 44px, smooth CSS transition, Cmd+B shortcut, persisted)
- [x] Cmd+T shortcut to open new terminal tab, Cmd+W closes active segment
- [x] Graph viewport cache: instant tab switching with cached graph data (no loading spinner)
- [x] Auto-navigate to graph on project tab switch (instant, prevents stale pipeline/changes data)
- [x] Fixed: recent projects list empty on first use
- [x] Fixed: unstaged file diff preview not loading after tab switch
- [x] Fixed: icon consistency (close ✕, plus +) and vertical centering in action buttons
- [x] Fixed: + button popup click-outside closing

#### Remaining (future)

- [ ] Split management (multiple terminals per project)
- [ ] Terminal process detection (auto-detect Claude/Codex running, update label dynamically)
- [ ] Project auto-detection: terminal navigating to another project path re-links

---

## Phase 5: AI Integration

Implementation in 3 waves. Wave 1 (5.1 + 5.2) complete.

### Wave 1: AiProvider Trait + Claude Code ✅

### 5.1 — AiProvider Trait & Detection ✅

- [x] New `ai-provider` crate with `AiProvider` trait (7 capability groups, sync, command-building)
- [x] Shared types: `AiProviderKind`, `AiSession`, `AiWorktree`, `AiConfigFile`, `ExecuteOptions`
- [x] Two-phase detection: binary scan on startup (PATH), repo scan on tab switch (`.claude/`, `.codex/`)
- [x] Default implementations return empty/None/NotSupported — providers override what they support
- [x] Trait covers: identity, detection, headless execution, specialized actions, interactive launch, session/worktree introspection, config/attribution

### 5.2 — Claude Code (First Provider) ✅

- [x] New `claude-code` crate implementing `AiProvider` for Claude Code CLI
- [x] Detection: `which claude`, `claude --version`, scan `.claude/` + `CLAUDE.md`
- [x] Headless execution: `claude --print` via TaskManager (commit msg, review, analysis, PR description, PR review)
- [x] Interactive launch: spawn `claude` in terminal tab via TerminalManager
- [x] Worktree support: `claude --worktree`, `git worktree list` cross-ref, `worktree-*` branch convention
- [x] Session introspection: parse `~/.claude/sessions/*.json`, PID liveness checks
- [x] Config discovery: settings.json (user/project/local), agents/*.md, skills/*/SKILL.md, CLAUDE.md hierarchy
- [x] Commit attribution: `Authored-by:` footer, `Co-authored-by:` trailer, author name patterns
- [x] 14 Tauri commands in `app-core/ai_commands.rs` (detection, actions, launch, introspection)
- [x] Frontend: `ai.ts` store, AI action buttons in staging view, terminal launcher wired to `ai_launch_interactive`
- [x] Output via existing task viewer (same UX as git fetch/push)
- [x] Terminal dropdown launches `claude` binary directly (auto-starts Claude Code)
- [x] Brand icons (Claude SVG) in terminal tabs and composite tab segments
- [x] E2E test suite for AI provider store (15 tests)

### Wave 2: UI Views + Attribution

### 5.3 — AI Worktree Sidebar

- [ ] New sidebar section per project: AI worktrees grouped under their project tab
- [ ] Status badges: active session, changes pending, clean, orphaned
- [ ] Click to navigate to worktree's terminal tab
- [ ] Context menu: open in graph, cleanup worktree + branch, open in new project tab

### 5.4 — AI Commit Attribution

- [ ] Detect AI-authored commits by patterns per provider
  - `Authored-by:` footers, `Co-authored-by:` trailers, author name matching
- [ ] AI badge/icon on graph commits (distinct from CI/MR badges)
- [ ] Filter: show only AI-generated or only human-generated commits
- [ ] Stats in project overview: percentage of AI-authored commits in last N commits

### 5.5 — Config Viewer

- [ ] Read-only panel showing AI config files for current repo
- [ ] Tabs per detected tool: CLAUDE.md, AGENTS.md, opencode.json
- [ ] Syntax-highlighted with CodeMirror (reuse existing editor engine)

### 5.6 — Session Dashboard

- [ ] Cross-project view: all active AI sessions across all open project tabs
- [ ] Per session: tool name, worktree, branch, terminal tab link, recent activity
- [ ] Quick actions: focus terminal, open worktree in graph, stop session

### Wave 3: Additional Providers

### 5.7 — Additional Provider Implementations

- [ ] Codex CLI (`AiProvider` implementation — `codex exec`, TOML config, SQLite sessions)
- [ ] OpenCode (`AiProvider` implementation — `-p` flag, JSON config, SQLite sessions)
- [ ] Each wired to terminal launcher dropdown and all AI action buttons

---

## Phase 6: Git Completion & Code Quality

### 6.1 — Bisect (Visual Workflow)

- [ ] Visual bisect UI: mark good/bad commits in graph
- [ ] Step-by-step guided workflow with current test commit highlighted
- [ ] Auto-bisect option (run command per step)
- [ ] Bisect log and reset

### 6.2 — Bundle gh/glab Binaries

- [ ] Download platform-specific gh/glab binaries in CI build pipeline
- [ ] Ship bundled with app (macOS arm64/x64, Linux x64, Windows x64)
- [ ] gh/glab OAuth flows run inside interactive terminal tabs
- [ ] Auto-update bundled CLIs with app updates

### 6.3 — Code Quality

- [ ] Split `commands.rs` (~3148 lines) into feature-based modules
- [ ] Extract generic `<List>` component (~1500 LOC savings across 8 components)
- [ ] Extract shared dialog CSS to `src/lib/styles/dialog.css`
- [ ] Extract store `fetchIntoStore` helper
- [ ] CLI provider JSON parsing deduplication (GitHub/GitLab parsers)

### 6.4 — Infrastructure

- [ ] Crash reporting / telemetry (opt-in)
- [ ] Auto-update scope: app + libgit2 + git + gh + glab (extend existing updater)

### 6.5 — E2E Testing Infrastructure (partial ✅)

- [x] Vitest setup with global IPC mocking (`@tauri-apps/api/core`, events, window, dialog)
- [x] Configurable `mockInvokeResponse()` helper for per-test mock data
- [x] 6 E2E workflow test suites: repo-open, staging-commit, branch-ops, tag-ops, stash-ops, ai-provider
- [x] 149 total frontend tests (74 existing + 75 new E2E), all passing
- [ ] `tauri-driver` + Playwright/WebdriverIO for full webview automation
- [ ] Test harness that launches the app against a fixture repo
- [ ] Golden path tests: open repo, navigate views, stage/commit, run terminal, launch AI session
- [ ] CI integration: headless E2E suite on build pipeline
- [ ] Regression suite: one test per major feature (graph, branches, merge editor, terminal, AI)

---

## Branch Strategy

- `main` — stable releases
- `beta` — development, beta updates
- `feature/*` — new features
- `bugfix/*` — bug fixes
