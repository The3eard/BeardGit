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

### 4B — Interactive Terminal Tabs

- [ ] Terminal launcher button with dropdown (plain terminal initially)
- [ ] Multi-tab terminal sessions, each bound to a project tab
- [ ] Split management within same project — cross-project split auto-extracts to new tab
- [ ] Project auto-detection: terminal navigating to another project path re-links to that project tab
- [ ] Visual grouping: terminal tabs linked to their project tab
- [ ] xterm.js features: splits, themes, ligatures (Fira Code), search, clickable URLs

---

## Phase 5: AI Integration

### 5.1 — AiProvider Trait & Detection

- [ ] New `ai-provider` crate with `AiProvider` trait (like `CiProvider`)
- [ ] Trait methods: detect config, list sessions, list worktrees, get commit attribution, get config files
- [ ] Null/empty default implementations for unsupported features per provider
- [ ] Auto-detection: scan repo for `.claude/`, `.codex/`, `.opencode/`, `.aider*`
- [ ] Config file reader: `CLAUDE.md`, `AGENTS.md`, `opencode.json`, `.aider.conf.yml`

### 5.2 — Claude Code (First Provider)

- [ ] Implement `AiProvider` for Claude Code
- [ ] Terminal launcher dropdown gains "Terminal with Claude Code" option
- [ ] Detect `.claude/worktrees/` — list, status (active/orphaned/clean), cleanup
- [ ] Parse `worktree-*` branch naming convention
- [ ] Read `.claude/settings.json`, agents, skills, rules

### 5.3 — AI Worktree Sidebar

- [ ] New sidebar section per project: AI worktrees grouped under their project tab
- [ ] Status badges: active session, changes pending, clean, orphaned
- [ ] Click to navigate to worktree's terminal tab
- [ ] Context menu: open in graph, cleanup worktree + branch, open in new project tab

### 5.4 — AI Commit Attribution

- [ ] Detect AI-authored commits by patterns per provider
  - `Authored-by:` footers, `Co-authored-by:` trailers, `(aider)` in author name
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

### 5.7 — Additional Provider Implementations

- [ ] Codex CLI (`AiProvider` implementation)
- [ ] OpenCode (`AiProvider` implementation)
- [ ] Aider (`AiProvider` implementation)
- [ ] Each wired to terminal launcher dropdown

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

### 6.5 — E2E Testing Infrastructure

- [ ] `tauri-driver` + Playwright/WebdriverIO for webview automation
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
