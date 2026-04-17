# BeardGit — Project Status

## Completed

### MVP (v0.1.0)

Canvas-based git graph (100K+ commits), staging area, branch/tag/stash management, fetch/pull/push as background tasks, GitLab + GitHub CI integration with ANSI log viewer, multi-project tabs with persistence, encrypted credential store, filesystem watcher, i18n (en/es), 4 TOML themes, custom app icon, Fira Code + Nerd Font icons.

### Phase 2: Core Workflows (v0.1.1–v0.1.2)

CodeMirror 6 diff engine with syntax highlighting (16 languages), hunk/line-level staging, blame with gutter annotations, file history with rename detection, revert/amend/reset, worktree management, non-interactive + interactive rebase, 3-way merge editor, resizable graph columns, complementary theme pairing, 14 themes (10 dark, 4 light), UI scale setting, all CLI commands non-blocking.

### Phase 3: Power Features + CLI Integration (v0.1.3–v0.1.4)

Task history popup, keyboard shortcuts with cheat sheet, reflog viewer with recovery actions, clean with preview dialog, git config editor (local/global), gitignore management (context menu + CodeMirror editor), patch create/apply, submodule management with open-as-tab, MR/PR management via cli-provider crate (CRUD, review, comments, graph badges), IntelliJ-style merge editor v3 (custom diff engine, SVG bezier connectors, accept/ignore, undo, conflict navigation), auto-update system with toast notifications, multi-file selection with checkboxes, SplitView migration, performance audit fixes.

### Phase 4: Terminal Foundation (xterm.js) ✅

**4A — Terminal Core + Read-Only Views + Theme Bridge:** Rust PTY manager crate, xterm.js Svelte component (WebGL, fit, web-links, search addons), shell detection (zsh/bash/powershell), read-only instance pool, TaskPanel + CI JobLog migrated to xterm.js, theme system redesigned (18 base colors + 16 ANSI), 14 TOML themes updated, Tauri commands + event bridge.

**4B — Interactive Terminal Tabs + UI Improvements:** Unified tab model (project | terminal | composite), composite segmented tabs, terminal split button + dropdown with AI provider entries (Claude/Codex/OpenCode brand icons), standalone terminals, NerdFont in terminal, sidebar collapse (Cmd+B), Cmd+T/Cmd+W shortcuts, graph viewport cache, auto-navigate on tab switch.

### Phase 5: AI Integration ✅

**Wave 1 — AiProvider Trait + Claude Code (v0.1.7):** `AiProvider` trait (17 methods, 7 capability groups), `claude-code` crate, 16 Tauri commands, AI provider settings UI, AI button validation, Changes section redesign, reflog overhaul, submodule add/remove, tab tooltips, E2E test infrastructure (6 suites, 149 tests).

**Wave 2 — UI Views (v0.1.8):** Worktree section enriched with AI badges + context menu (EnrichedWorktree join), AI Config Editor (dual file tree project/user, editable CodeMirror, create dialog for agents/skills/prompts, 3 new Tauri commands with path validation), AI Sessions view (project-scoped session list, file watcher on ~/.claude/sessions/, auto-refresh via events).

**Wave 3 — Additional Providers (v0.1.8):** `codex` crate (detection, commands, TOML config, attribution), `opencode` crate (detection, commands, JSON config, attribution), both wired into make_provider factory, dynamic terminal dropdown (only detected providers), Codex brand color #ffffff.

### Phase 6: Git Completion & Code Quality (v0.1.8) ✅

**6.3 — Code Quality:** Split `commands.rs` (3267 lines, 139 commands) into 24 feature-based modules under `commands/`, shared dialog CSS extracted to `dialog.css`, `fetchIntoStore` utility extracted from repeated store patterns.

**6.1 — Bisect (Visual Workflow):** `git-engine` bisect module (8 operations via system git CLI), 8 Tauri commands, `bisect.ts` store, BisectWorkflow component (good/bad/skip controls, auto-bisect with test command, bisect log display), AutoBisectDialog.

**6.2 — CLI Auth + Integration:** CLI auth status detection for gh/glab, 3 Tauri commands, CliAuthSection settings component with authenticate/logout buttons via interactive terminal, unified settings page (Token Auth + CLI Auth).

**6.4 — Error Logging + Debug UX:** `tracing`-based structured file logging with daily rotation, platform-specific log directories, debug info collection, ErrorDialog component with copy-error and open-log actions, 3 Tauri commands.

**6.5 — E2E Testing Infrastructure:** WebdriverIO + tauri-driver config, 3 fixture repo setup scripts, sidebar/graph page objects, data-testid attributes, 2 initial test specs.

**Composite Tab Upgrade:** Multi-segment composite tabs (N terminals + worktrees per project), fixed segment ordering (Project → Worktrees → AI Terminals → Terminals), terminal button always adds to composite, `+` button and dropdown for adding segments.

### Phase 8: Forge Integration ✅

**8.1 — ForgeProvider trait refactor:** New `forge-provider` crate exporting the `ForgeProvider` trait + shared types (`MrPr`, `Issue`, `Release`, `Label`, `User`, `Milestone`, `Comment`, …) and `ForgeError`. `cli-provider` split into `GitHubCli` and `GitLabCli` structs, each implementing the trait. `build_forge_provider(AppState) → Arc<dyn ForgeProvider>`. No user-visible change; foundation for 8.2–8.5.

**8.2 — MR/PR Enhancements:** 11 new trait methods (add/remove labels, add/remove reviewers, mark ready/draft, reopen, resolve/unresolve discussions, checkout MR/PR locally, list repo labels). MrPrDetail UI gets `LabelPicker`, `ReviewerPicker`, draft toggle, reopen button, per-comment resolve on GitLab, and "Checkout locally" streaming via TaskManager.

**8.3 — Issues Vertical:** 13 new trait methods (list/get/create/edit/close/reopen/comment, labels/milestones/assignees). New `commands/issues.rs`, new store `stores/issues.ts`, new `components/issues/` (IssueView, IssueList, IssueDetail, CreateIssueDialog, AssigneePicker, MilestonePicker), shared `common/LabelPicker.svelte` + `common/Xrefs.svelte`, new `utils/xrefs.ts` cross-ref parser. Sidebar entry "Issues".

**8.4 — CI/CD Control:** 6 new `CiProvider` methods (trigger_workflow, retry_run, retry_failed_jobs, retry_job, cancel_run, list_workflows) implemented in `gitlab-api` and `github-api` via reqwest; tests use `mockito`. New `TriggerWorkflowDialog`; `PipelineList` gains a "Run Workflow" button + row context menu; `PipelineDetail` gains retry/cancel actions + per-job retry. GitHub PAT `workflow` scope hint in ProviderSetup.

**8.5 — Releases Vertical:** 9 new trait methods (list/get/create/edit/delete/publish + 3 asset ops). Asset upload streams via TaskManager for non-blocking progress. New `commands/releases.rs`, store `stores/releases.ts`, `components/releases/` (ReleaseView, ReleaseList, ReleaseDetail, CreateReleaseDialog, AssetUploadProgress). `xrefs.ts` extended to recognize release tags against a loaded tag cache. `create_tag_and_release` flow atomically pushes the tag and creates the release via TaskManager.

---

## Phase 7: Polish, Performance & Remaining Items

All undone items from previous phases consolidated here.

### 7.1 — Terminal Enhancements

- [ ] Terminal cwd auto-detection: OSC 7 shell integration, terminal navigating to a project path auto-links to composite tab
- [ ] Terminal process detection: auto-detect Claude/Codex running in terminal, update label dynamically
- [ ] Split terminal panes: multiple xterm.js instances in split layout within one tab segment

### 7.2 — CLI Binary Bundling

- [ ] Download platform-specific gh/glab binaries in CI build pipeline
- [ ] Ship bundled with app (macOS arm64/x64, Linux x64, Windows x64)
- [ ] Auto-update bundled CLIs with app updates
- [ ] Fallback to system binaries when bundled not available

### 7.3 — Code Quality (Remaining)

- [ ] Extract generic `<List>` component (~1500 LOC savings across 11 components)
- [ ] Extract store `fetchIntoStore` to remaining stores (branches, tags, stashes, reflog, mr-pr)
- [ ] CLI provider JSON parsing deduplication (GitHub/GitLab shared parser)

### 7.4 — E2E Testing Expansion

- [ ] `tauri-driver` + WebdriverIO full integration (headless app launch)
- [ ] Golden path tests: open repo → navigate → stage/commit → terminal → AI session
- [ ] CI integration: headless E2E suite on build pipeline (xvfb-run on Linux)
- [ ] Regression suite: one test per major feature (graph, branches, merge editor, terminal, AI)

### 7.5 — Infrastructure

- [ ] Auto-update scope: extend to bundled gh/glab binaries (after 7.2)
- [ ] Error logging: add `tracing` instrumentation to git write operations (push/pull/fetch/commit/merge/rebase)
- [ ] Log rotation cleanup: auto-purge logs older than 7 days

### 7.6 — UI Polish & Bug Fixes

- [ ] AI Sessions "Focus" button: wire to actual terminal tab when session has a linked BeardGit terminal
- [ ] Bisect graph integration: highlight good/bad/current commits with colored overlays in canvas graph
- [ ] Bisect context menu: right-click commit in graph → "Mark as good" / "Mark as bad"
- [ ] AI Config Editor: file watcher for live reload when external editor modifies config files
- [ ] Worktree "Open in graph": navigate graph view to worktree's branch
- [ ] Worktree "Lock/Unlock": wire the context menu stub actions

### 7.7 — Performance

- [ ] Profile and optimize large repo graph rendering (100K+ commits)
- [ ] Terminal instance pooling: recycle xterm.js instances for closed terminals
- [ ] Lazy-load CodeMirror languages (only load grammar when file type first encountered)

---

## Phase 9: Provider Architecture Cleanup

Goal: make the forge/CI provider layer easier to read, test, and extend. Pure refactor, no user-visible behaviour change. Phases 8.2–8.5 grew `cli-provider/src/github.rs` and `gitlab.rs` to ~800 LOC each with every feature piled into one big `impl` block — this phase splits them before they get larger and draws a harder line between trait crates (contracts) and implementation crates (logic).

### 9.1 — `provider` crate split

- [ ] Split `crates/provider/src/lib.rs` (883 lines) — move `CiProvider` trait into `traits.rs`, `CiStatus`/`CiRun`/`CiJob`/`CiStage`/`CiRunDetail` into `types.rs` (joining the post-8.4 types already there), `ProviderKind` + `parse_remote_url` into `kind.rs`, `ProviderError` into `error.rs`
- [ ] `lib.rs` becomes re-exports only — no definitions
- [ ] Audit: the `provider` crate must never gain an HTTP dep. Impls belong in `gitlab-api`/`github-api`

### 9.2 — `cli-provider` per-vertical split

- [ ] Convert `cli-provider/src/github.rs` and `gitlab.rs` into module directories: `github/mod.rs` declaring the struct + `impl ForgeProvider`, with methods grouped into submodules `mr_pr.rs`, `issues.rs`, `releases.rs`, `labels.rs`, `reviewers.rs`, `checkout.rs`, `discussions.rs`
- [ ] Each submodule contains only `impl GitHubCli { pub(super) fn … }` methods that the main `impl ForgeProvider for GitHubCli` delegates to. Keep args-builders + parse helpers colocated with their feature.
- [ ] Same structure for `gitlab/`. Target: no file >400 LOC.

### 9.3 — Trait / implementation discipline

- [ ] Add CI guard (grep-based lint or `deny` attr) enforcing that `forge-provider` and `provider` never import `reqwest`, `tokio`, `tauri`, or `std::process::Command`. They are contract-only crates.
- [ ] Promote `forge-provider::mock` behind a proper `mock` feature flag with richer fakes so `app-core` tests can exercise Tauri commands without shelling out to real CLIs
- [ ] Same treatment for `provider::CiProvider` — a `MockCiProvider` that returns canned pipelines/runs for frontend-integration tests

### 9.4 — Cross-cutting helpers

- [ ] Extract shared `run` / `run_json` / `run_with_stdin` from `cli-provider` into a small internal helper module reused across all CLI-based forge impls (and any future AI provider wrappers that shell out)
- [ ] Audit `gitlab-api` and `github-api` for shared retry / rate-limit / error-mapping logic — factor common primitives into a single place so bug fixes don't need two copies
- [ ] Deduplicate JSON parsing helpers already listed in Phase 7.3 (GitHub/GitLab shared parser) — bring that item into 9.4's scope since both touch the same files

### 9.5 — Documentation pass

- [ ] Update `crates/CLAUDE.md` with the post-9.1/9.2 layout so future contributors know where each piece lives
- [ ] Add a short "Adding a new forge capability" how-to that walks through: trait method on `ForgeProvider` → default `NotSupported` → GitHub/GitLab submodule impl → Tauri command → frontend store → i18n. Keeps the Phase 8 muscle memory captured.

---

## Future Considerations (Post-1.0)

### CLI-only authentication (retire PAT + REST API path)

**Idea:** Currently BeardGit uses two parallel paths — (1) CLI wrappers (`gh`/`glab`) via the `cli-provider` crate for MR/PR operations, and (2) REST API clients (`gitlab-api`/`github-api`) with PAT auth via the `auth` crate for CI pipeline integration. Long-term, a CLI-only path would simplify the auth surface and remove duplicated logic.

**Pros:**
- Single auth flow (`gh auth login` / `glab auth login`); no PAT screen
- OAuth short-lived tokens > long-lived PATs (better security)
- Drop `reqwest`/`rustls-tls` deps, simplify or remove `gitlab-api`/`github-api`/`auth` crates
- CLIs expose full feature parity with REST (runs, logs, triggers, releases, issues)

**Cons:**
- Hard dependency on CLI binaries being available (Phase 7.2 bundling must be rock-solid)
- Subprocess overhead on CI polling (~6 spawns/min per active project) vs direct HTTPS
- Log streaming needs rework (CLI returns complete output; current REST integration is byte-level)
- Rewriting `CiProvider` trait implementations as CLI wrappers is non-trivial
- User migration friction (existing PAT users need to re-auth via CLI OAuth)

**Plan:** Deferred until after Phase 8. Revisit when:
1. Phase 7.2 bundling confirmed stable across all platforms
2. Real-world polling performance measured on bundled CLIs
3. `ForgeProvider` trait (Phase 8.1) has proven the CLI abstraction

When done, this becomes a dedicated phase (likely Phase 11 or 12) focused on CI REST-to-CLI migration, followed by removal of the PAT auth path.

---

## Branch Strategy

- `main` — stable releases
- `beta` — development, beta updates
- `feature/*` — new features
- `bugfix/*` — bug fixes
