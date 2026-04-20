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

### Phase 7: Polish, Performance & Remaining Items ✅

**7.1 — Terminal Enhancements:** OSC 7 shell integration for cwd auto-detection (terminal navigating to a project path auto-links to its composite tab); foreground process polling auto-detects Claude / Codex / OpenCode running in a terminal and updates the tab label + icon dynamically. Split terminal panes deferred to a future phase (not in the plan's scope).

**7.2 — CLI Binary Bundling:** `gh` v2.62.0 and `glab` v1.46.1 bundled as Tauri sidecars on macOS arm64, Linux x64, Windows x64 via `scripts/download-cli-binaries.js` + `cli-versions.json`. `resolve_cli_binary()` checks sidecar location first, falls back to system PATH. Build + Release pipelines download matrix-specific binaries before `tauri-action`. Validated end-to-end across all 3 platforms.

**7.3 — Code Quality:** Generic `<List>` component with 10 consumers (Branch / Tag / Stash / Reflog / MrPr / Worktree / Submodule / Release / Issue / AiSession lists). `fetchIntoStore` / `fetchListIntoStore` / `fetchPageIntoStore` helpers consumed by 10 stores. CLI provider JSON parser dedup landed via 8.1 (`GITHUB_FIELDS`/`GITLAB_FIELDS` pattern) + 9.4 cleanup.

**7.4 — E2E Testing Expansion:** 9 spec files / 53 tests (app-launch, navigation, golden-path, regression/{graph, branches, staging, terminal, bisect, settings}). 6 new page objects + 2 expanded; data-testid attributes across graph, branches, changes, terminal, settings, bisect, dialogs. New `e2e-tests` CI job (ubuntu-22.04, xvfb + tauri-driver) scoped to main/beta pushes and PRs targeting them; uploads junit.xml + failure screenshots.

**7.5 — Infrastructure:** `storage::logging::purge_old_logs()` auto-removes log files older than 7 days via `async_runtime::spawn` + `spawn_blocking` on startup. 41 `#[instrument]` spans on git-engine write operations (bisect, operations, conflict, reset, clean, remote, worktree, submodule, interactive_rebase) with sensitive fields excluded. 80 `#[instrument(name = "cmd::…")]` spans on 19 Tauri command modules for hierarchical log grepping. Auto-update for bundled CLIs is implicit (Tauri updater replaces the full app bundle).

**7.6 — UI Polish:** Bisect graph overlays + right-click context menu (Mark good/bad/skip). Worktree lock/unlock wired through git-engine + context menu; "Open in graph" navigates to worktree's branch. AI Config Editor live-reload via new `watcher::ai_config` module. AI Sessions "Focus" focuses linked terminal tab or launches `claude --resume <sessionId>`.

**7.7 — Performance:** Graph render profiling via `graph-perf.ts` — 6 `performance.mark` pairs around the render loop, dev-only FPS overlay toggled with `Ctrl+Shift+P`. Interactive xterm.js instance pool (3 deep) recycles terminals across tab open/close to reduce GC pressure. CodeMirror language cache short-circuits repeated dynamic imports per file extension.

### Phase 10: AI Background Worktree ✅

One-shot, headless AI runs in an isolated worktree — no terminal tab required.
- **Spec / plan:** `docs/superpowers/specs/2026-04-17-ai-background-worktree.md` + `docs/superpowers/plans/2026-04-17-phase10-ai-background-worktree.md`
- **Dialog (3 entry points):** tab-bar button, `AI Sessions` header "+ New run" button, and `Cmd+Shift+A` global shortcut all open the same `CreateBackgroundRunDialog.svelte`. Dialog supports free-text prompts, saved prompts from `.claude/prompts/`, and skill invocations from `.claude/skills/`.
- **Headless coordinator:** `app-core::ai_background::AiBackgroundCoordinator` creates a worktree at `<repo>/.beardgit/ai-worktrees/<slug>` on a new branch `ai/<provider>/<slug>`, inlines any saved-prompt/skill content into the prompt, spawns the provider via `TaskManager` (with stdin piping for Claude Code's JSON stream mode), and registers the session in a live registry. Configurable concurrency cap queues excess runs.
- **Providers:** all three (`claude-code` / `codex` / `opencode`) implement the new `AiProvider::launch_background` method — Claude uses `--print --output-format stream-json --verbose` with stdin; Codex uses `codex exec -p`; OpenCode uses `opencode run -p`. `--dangerously-skip-permissions` (and its Codex equivalent) is gated behind a new `ai_prompt_auto_accept` setting, default off.
- **UI:** new `BackgroundRunStatusBadge`, `BackgroundRunTranscript`, and `AiSessionDetail` components. AI Sessions sidebar merges background runs (sorted first, with status pill) ahead of provider-reported sessions. Session detail exposes `Open terminal here` (disabled with tooltip while running), `Switch to worktree tab`, `Discard worktree`, and a transcript viewer with copy-to-clipboard.
- **Settings:** `Settings → AI Provider` gains worktree-root override, concurrency-cap slider, and the auto-accept toggle. All settings persist via two new Tauri commands.
- **Tests:** 13 new Rust tests (coordinator lifecycle, concurrency cap, queue cancellation, slug derivation, worktree-root resolution, stdin piping, AiBackground TaskKind); 7 vitest cases for the store. All 266 frontend tests + full Rust workspace pass.

### Phase 9: Provider Architecture Cleanup ✅

Pure refactor, zero user-visible change.
- **9.1** `provider/lib.rs` (883 LOC) split into `traits.rs` / `types.rs` / `kind.rs` / `error.rs` / `http_helpers.rs` / `mock.rs`; `lib.rs` now 43 LOC of re-exports
- **9.2** `cli-provider/src/github.rs` and `gitlab.rs` (~800 LOC each) converted to directory modules with per-vertical submodules (mr_pr, labels, reviewers, lifecycle, discussions, checkout, issues, releases). `impl ForgeProvider` in `mod.rs` delegates one-line to feature-scoped methods. No file exceeds 400 LOC.
- **9.3** CI grep guard in `ci.yml` enforces trait-crate purity (provider + forge-provider cannot import reqwest/tokio/tauri/hyper at the src level). `mock` feature flag exposes `MockProvider` + `MockCiProvider` for integration tests.
- **9.4** Pure HTTP primitives (`api_error`, `retry_after_secs`, `trim_base_url`) extracted into `provider::http_helpers`; consumed by gitlab-api + github-api. Rate-limit arithmetic now unit-testable without fabricating HTTP responses.
- **9.5** `crates/CLAUDE.md` refreshed with the post-9.1/9.2 layout + an "Adding a new forge capability" how-to walkthrough.

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
