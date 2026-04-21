# Changelog

All notable changes to BeardGit are documented here. Format follows [keepachangelog.com](https://keepachangelog.com).

## [Unreleased] — Reactivity foundation, AI sessions UX, forge data fixes, settings IA polish, log rename, E2E retirement

Six sequential specs brainstormed and shipped on 2026-04-21. Each spec merged into `beta` on its own feature branch with a dedicated design + plan document.

### Reactivity & feedback foundation (Spec 1)

Every repository mutation — UI-initiated, AI-initiated, or external CLI — now broadcasts a precise `project-mutated` event so the UI converges on fresh state without per-call-site refresh code.

- **New `mutation-events` Rust crate** — `Snapshot::capture` + `diff`, `MutationGuard` RAII wrapper, `MutationKind` enum (commit / push / stash / worktree / staging_change / ai / external / …), `MutationFlags` struct, `emit_mutation` helper. Status fingerprint tracks per-file index/worktree bitflags so staging transitions flip `status_changed` even when the overall dirty boolean doesn't move.
- **app-core commands wrapped** — every mutating Tauri command (commit / amend / branch / tag / stash / staging / worktree / remote / cherry-pick / revert / rebase / reset / merge / conflict / clean / patch / submodule / mr_pr / releases) fires the guard on success.
- **watcher crate** — debounced `.git/**` change → `MutationKind::External` so CLI edits outside BeardGit still refresh the UI.
- **AI background runs** — coordinator captures pre/post worktree snapshots and emits `MutationKind::Ai { source }` on completion / failure / cancellation.
- **TS `mutations.ts` store** — single `project-mutated` listener coalesces events per rAF tick, buffers flags per project path, flushes on tab switch, dispatches the minimal refresh set to `graph` / `changes` / `stashes` / `worktrees` / `repoConfig`.
- **`runMutation` wrapper** — caller-side toast + task-record seam. Silent-set (stage / unstage / discard) suppresses success toast. Failures are sticky with a **See details** action that opens the Tasks popover at the failing task's detail panel.
- **Graph cache-first paint** — per-project `GraphViewportCache` slice persisted in `project-cache.ts`; synchronous hydration on cold start; faint skeleton stripes while first paint resolves; HEAD-OID reconciliation preserves scroll anchor when new commits land above the cached top.
- **Statusbar provider filter** — new `projectProvider` derived store. `repoConfig` wins; otherwise inferred from `origin` URL (`github.com`, `gitlab.*`). Renders 0 or 1 pill, never both.
- **Tasks popover regression fixed** — click-bubble race where the opening click hit the outside-click handler on the same frame. Rising-edge `ready` latch tied to the `open` transition.

### AI sessions UX pass (Spec 2)

The AI Sessions tab is now async-first, populates detail on click, supports open-in-terminal via the shared runMutation seam, and renders brand logos at native transparency.

- **`ProviderIcon` shared component** + brand SVG assets for Claude Code, Codex, OpenCode, and a generic fallback. No enclosing background square — brand logos render at native transparency.
- **`AiSessionList`** — shell paints immediately, refresh fires fire-and-forget in `onMount`. Row layout: 8 px padding, vertically-centered 20 px icon slot, External badge when `worktree_path` is missing or unreachable.
- **`AiSessionDetail`** — populates on `selectedBackgroundSessionId` change. Header uses `ProviderIcon`. Open-in-terminal / Cancel / Discard routed through `runMutation` with sticky-failure toasts.
- **AI Settings, TabBar, AiSlot, TaskEntryRow** — migrated to the shared `ProviderIcon`; generic nerd-font glyphs retired.
- **i18n** — en-US + es-ES keys for the new toast labels.

### Forge data fixes (Spec 3)

PR and Release detail panes no longer hang; error and empty states are distinct and localized.

- **`ForgeDetailShell`** — shared loading / error / empty / content state primitive used by both `MrPrDetail` and `ReleaseDetail`.
- **`withTimeout`** helper + `TimeoutError` — 15 s bound on detail fetches; errors surface via a new per-detail error store (`mrPrDetailError` / `releaseDetailError`) + sticky toast with a **Retry** action.
- **PR #18 infinite loading** — root cause traced to unbounded `gh api --paginate` for ~3 400-file diffs. Fix: 50 MB payload cap + 20 s subprocess timeout in `cli-provider` via `wait-timeout`. Frontend's 15 s `withTimeout` is the outer guard.
- **Release-blank** — `#[serde(default)]` didn't accept explicit JSON `null`. New `null_as_default` deserializer handles null `body` / `assets` from `gh`/`glab` payloads.
- **Empty-state copy** — "No changes in this pull request." / "No release notes or assets published for {tag}."

### Settings IA polish (Spec 4)

One canonical shape for the settings navigation, driven by the shared primitives from Spec 2.

- **`LookAndFeelSection.svelte`** extracted from `GeneralSettings`. General now owns a single Look & Feel card (no duplicate blocks).
- **Appearance tab removed** — collapsed into General; legacy `appearance` deep-links redirect to `general`.
- **Editor/Diff tab removed** — the placeholder page wasn't implemented; legacy `editor` deep-links redirect to `general`.
- **`CATEGORY_IDS`** reduced to `general / git / ai / integrations / advanced`.
- **AI Settings** — stray broken-glyph refresh button deleted; provider icons verified wired to `ProviderIcon`.
- **`ConnectionHowTo`** — reworked as a compact top-level dropdown (OAuth / PAT / CLI modes) rendered above the card, not inside.
- **Integrations Connections** — unified single Card with a new `ConnectionRow` primitive dispatching on `kind` (github / gitlab / gh / glab). `CliAuthSection.svelte` and `ProviderSetup.svelte` deleted.

### Log filename convention (Spec 5)

Log files now write as `beardgit.{date}.log` (was `beardgit.log.{date}`) so `*.log` globs match them and log-rotation tooling sees the date as the disambiguator, not the extension. Rotation cleanup tolerates both shapes so legacy files age out under the existing retention policy.

### E2E infrastructure retired

The WebdriverIO + tauri-driver suite is removed while the app is in heavy flux. Specs would need continuous rewriting against a moving target; the Vitest integration layer under `src/test/e2e/` remains as the sustainable cross-store regression suite. Re-introduction happens once the UI stabilises and a focused "write E2E from scratch" spec is brainstormed.

- Deleted `e2e/` directory (specs, fixtures, page objects, Dockerfile, run scripts).
- Dropped the `e2e-tests` job from `.github/workflows/ci.yml`.
- Removed `@wdio/*` devDependencies + the `npm run e2e*` scripts from `package.json`.
- Dropped the `window.__E2E__` surface + `VITE_BEARDGIT_E2E` gate from `src/routes/+layout.svelte`.

### Testing

- **Rust:** 965 tests across 22 crates, clippy clean on `--workspace --all-targets`.
- **Frontend:** 595 Vitest tests across 86 files.
- **svelte-check:** 0 errors / 0 warnings across 2 618 files.

## [0.1.8] — Phases 6–10: Bisect, CLI Auth, AI Stack, Forge Integration, Bundled CLIs, Refactor, E2E, Performance

The biggest release since the MVP — everything since `v0.1.7-beta` ships in one cut. Five phases of feature work plus a deep architecture and performance pass: visual bisect, CLI auth, the full AI stack (three providers, headless background runs in worktrees), GitLab + GitHub forge integration with bundled CLIs, the provider architecture cleanup, and the E2E + tracing infrastructure.

### AI Background Worktree Runs (Phase 10)

Launch a headless AI coding run inside a fresh git worktree without opening a terminal. Three entry points: tab bar button, AI Sessions header, and `Cmd+Shift+A`. Prompt source: free text, saved prompt from `.claude/prompts/`, or skill from `.claude/skills/` (user or project scope). Provider: Claude Code, Codex, or OpenCode. Worktree root configurable (default `.beardgit/ai-worktrees`); concurrency cap configurable (default 3) with FIFO queueing past the cap.

- **`ai-provider`** — `AiBackgroundRunInput` + `AiBackgroundRunStatus` + `AiTokenUsage` types; `launch_background` trait method with `NotSupported` default; `MockProvider` override for tests.
- **`claude-code` / `codex` / `opencode`** — headless command builders with provider-specific flags (Claude: `--print --output-format stream-json --verbose`; Codex/OpenCode: prompt concatenation fallback where skill/prompt flags aren't native).
- **`task-runner`** — `TaskKind::AiBackground` variant + `spawn_with_options` with stdin piping (backwards compatible — `spawn()` unchanged).
- **`app-core`** — `AiBackgroundCoordinator` with full lifecycle (Queued → Running → Completed / Failed / Cancelled), concurrency cap enforcement, worktree creation via `git-engine` and cleanup on discard. 6 Tauri commands + 2 settings commands.
- **`git-engine`** — `create_worktree_at` helper used by the coordinator.
- **`storage`** — `AppConfig` gains `ai_worktree_root`, `ai_background_concurrency_cap`, `ai_prompt_auto_accept` fields with serde defaults.
- **Frontend** — `CreateBackgroundRunDialog` with Free / Saved / Skill tabs, `BackgroundRunStatusBadge`, `BackgroundRunTranscript` with ANSI stripping, session detail + list integration, settings card, ~50 i18n keys per locale. `aiBackground.ts` store wires `ai-background-output` / `ai-background-status` events via `requestAnimationFrame` batching (matches the `tasks.ts` pattern); merges live runs into `aiSessions` for unified sidebar display.
- **Testing** — 13 tests in `app-core::ai_background` (coordinator lifecycle + cap + cancel + discard), 5 in `ai-provider`, 3 per provider for argv builders, 7 vitest tests for the store.

Known follow-ups for a later release: "View changes" button (deferred — merge-editor expects a conflict state, current release ships "Switch to worktree tab" as the review path), toast notification event wiring (i18n keys present), and an end-to-end spec exercising the full dialog (placeholder at `e2e/specs/regression/ai-background.spec.ts`).

### Beta Audit — Performance & Code Quality

A bundled audit pass landing 15 fixes from the beta-audit spec — the highest-leverage cleanup before tagging the release.

**Performance (high impact)**
- Cache `which::which()` results per provider kind on `AppState` — repeated provider detection no longer hits the filesystem.
- Replace task polling loops with `TaskManager::wait_for_terminal` backed by `tokio::sync::Notify` — no more spin-wait on long-running tasks.
- Memoise `Arc<dyn ForgeProvider>` keyed on `(provider_index, project_path)` — repeated forge lookups skip the construction cost.

**Correctness**
- Populate the GitLab label cache so issue labels render with their real colour.
- Unify `MrPr.labels` with `Issue.labels` on `Vec<Label>`; `PillRow` now renders the real label colour everywhere.
- Drop redundant `refreshIssueList` calls on label / assignee / milestone mutations — the optimistic update already covers it.
- Route `resolve_startup_theme` through `src/lib/api/tauri.ts` for consistency with every other IPC call.
- Key `#each` blocks over MR/PR diff files + comment lists for stable Svelte reconciliation.

**Code quality**
- Extend the trait-crate purity CI guard to include `ai-provider` (alongside `provider` and `forge-provider`).
- Share `build_gh_upload_args` / `build_glab_upload_args` across crates instead of duplicating the argv shape.
- Add `TaskManager::get_status` and a frontend `taskById` derived map for O(1) status lookup.
- Move `shell_escape` into `helpers.rs` with unit tests.
- Rename `MrPrComment` to `ForgeComment` in TypeScript (deprecated alias kept for one release).
- Extend `MrPrFilter` with author / label / text fields, matching `IssueFilter`.
- Rename the `render:text` perf measure to `render:badges-and-text` to match what it actually measures.

### GitLab Provider Polish

- **Per-file +/- counts** — `projects/:id/merge_requests/{n}/diffs` returns the raw patch but no additions/deletions counts. We were hardcoding 0/0, which showed as "+0 -0" beside every file in the MR detail panel. New `count_patch_changes` parser counts `+` / `-` content lines while skipping `+++` / `---` file headers and `@@` hunk headers. 4 unit tests.
- **`glab mr list` boolean state flags** — glab (both 1.46.1 and 1.92.1) does not accept `--state <value>`; passing `--state opened` made glab reply "Unknown flag" and our list returned empty. Switched to the boolean form glab actually supports: default → opened, `--closed`, `--merged`, `--all`. Dropped the unused `state_to_glab_str` helper.
- **Provider-aware sidebar label** — sidebar "Merge Requests" now reads "Pull Requests" when the active provider is GitHub. View id stays `merge-requests` so routing is unchanged; only the label swaps. New `sidebar_pull_requests` i18n key.
- **MR/PR list errors are surfaced** — `refreshMrPrList()` no longer swallows failures. Errors go to a new `mrPrListError` store and `MrPrList` renders them inline with a Retry button instead of an empty list with no explanation.

### Settings — Connection Guide

- **"How to connect" guide** — collapsible help block in Settings → Connection covering standard gitlab.com / github.com setup (PAT + CLI flows), self-hosted GitLab with OAuth and token fallback, plus troubleshooting for the multi-config warning and the 404-when-self-hosted-points-at-gitlab.com trap.

### Distribution

- **macOS x64 dropped from the release matrix** — Apple Silicon runners are now the only macOS target. Reduces CI matrix time and avoids the dual-bundle confusion at install time.
- **Bundle formats trimmed** — `.msi`, `.deb`, and `.rpm` removed from the bundle list. The `.dmg`, `.AppImage`, and `.exe` remain as the supported install paths per platform.
- **First-launch documentation** — README now explains the unsigned-build workaround for macOS until code signing lands (Gatekeeper right-click → Open). E2E fixture path no longer pins to a hardcoded location; derived from the working directory at runtime so contributors can run the suite from anywhere.
- **Repository hygiene** — AI assistant artifacts (`.claude/`, `.codex/`, etc.) untracked from the repo and added to `.gitignore`.

### Forge Integration (Phase 8)

Full daily-dev-workflow parity with GitHub and GitLab web UIs, behind a clean provider abstraction.

**MR/PR Enhancements (8.2)** — 11 new forge methods. Add/remove labels and reviewers post-creation, mark draft ↔ ready, reopen closed MR/PRs, resolve / unresolve GitLab discussion threads, and check out an MR/PR branch locally via `TaskManager`-streamed CLI output. The detail panel gains `LabelPicker`, `ReviewerPicker`, draft toggle, reopen button, per-comment resolve controls, and a "Checkout locally" action.

**Issues (8.3)** — Complete issue management as a new sidebar vertical. List / get / create / edit / close / reopen / comment plus assignees, labels, and milestones via 13 new trait methods. New `IssueView`, `IssueList`, `IssueDetail`, `CreateIssueDialog`, `AssigneePicker`, `MilestonePicker` components. Generic shared `LabelPicker` and `Xrefs` components plus a new `xrefs.ts` utility that auto-links `#NNN`, `!MMM`, `@user`, and short SHAs inside any text body.

**CI/CD Control (8.4)** — Actions on top of the existing Pipelines view. Trigger, retry, retry-failed-only, per-job retry, cancel, and list-workflows via six new `CiProvider` methods implemented over reqwest. `PipelineList` gets a "Run Workflow" button and row context menu; `PipelineDetail` gains action buttons and per-job retry. New `TriggerWorkflowDialog` with dynamic input form. GitHub PAT `workflow` scope hint surfaced in provider setup.

**Releases (8.5)** — Release management as its own vertical. 9 new trait methods including asset upload that streams via `TaskManager` for non-blocking progress. New `ReleaseView`, `ReleaseList`, `ReleaseDetail`, `CreateReleaseDialog`, `AssetUploadProgress` components plus an atomic `create_tag_and_release` flow that pushes the tag and creates the release in one streamed task. The cross-reference parser is extended to recognise release tags against a live tag cache.

**ForgeProvider Trait (8.1)** — New `forge-provider` crate extracts the `ForgeProvider` trait and shared types (`MrPr`, `Issue`, `Release`, `Label`, `User`, `Milestone`, `Comment`, …) with a `ForgeError` enum. `cli-provider` split into `GitHubCli` and `GitLabCli` structs each implementing the trait. `build_forge_provider(AppState) → Arc<dyn ForgeProvider>`. Zero user-visible change; foundation for 8.2–8.5.

### Bundled CLI Binaries (Phase 7.2)

BeardGit now ships `gh` (v2.62.0) and `glab` (v1.46.1) as Tauri sidecars on macOS arm64, Linux x64, and Windows x64 — no manual install required. `scripts/download-cli-binaries.js` pulls pinned binaries from the official release URLs; the Build and Release pipelines fetch the matrix-specific target before `tauri-action`. `resolve_cli_binary()` checks the sidecar location first and falls back to the system PATH, so existing installations keep working. Validated end-to-end across all four platforms.

### Terminal Enhancements (Phase 7.1)

- **OSC 7 cwd auto-detection** — terminals emit their current working directory on every prompt; when the cwd matches an open project path, the terminal tab auto-links to that project's composite tab.
- **AI provider auto-detection** — a lightweight polling loop detects when a terminal launches `claude`, `codex`, or `opencode` and updates the tab label plus brand icon dynamically.

### UI Polish & Bug Fixes (Phase 7.6)

- **Bisect graph integration** — good/bad/current/skipped commits get colored overlays in the canvas graph; right-click a commit for "Mark as good / bad / skip".
- **Worktree lock / unlock** — full `git worktree lock` / `unlock` wired through `git-engine` with context menu controls.
- **Worktree "Open in graph"** — navigates the graph view to the worktree's branch.
- **AI Config Editor live reload** — new `watcher::ai_config` module picks up external edits to `settings.json`, `agents/*.md`, `skills/*/SKILL.md`, and `CLAUDE.md` files; the editor refreshes without losing in-progress changes.
- **AI Sessions "Focus"** — focuses the linked terminal tab if the session has one; otherwise launches `claude --resume <sessionId>` in a new PTY terminal.

### Infrastructure (Phase 7.5)

- **Log rotation** — `storage::logging::purge_old_logs()` auto-removes `beardgit.*.log` files older than 7 days on startup (async, non-blocking). Legacy `beardgit.log.*` files from pre-rename installs are also purged by age.
- **Tracing on git writes** — 41 `#[instrument]` spans on `git-engine` write operations (bisect / operations / conflict / reset / clean / remote / worktree / submodule / interactive_rebase). Sensitive fields (commit bodies, PR descriptions, PAT tokens) excluded via `skip(...)`.
- **Tracing on Tauri commands** — 80 `#[instrument(name = "cmd::…")]` spans across 19 command modules. Hierarchical names make log grepping trivial.

### Performance (Phase 7.7)

- **Graph render profiler** — six `performance.mark` pairs around the render loop plus a dev-only FPS overlay toggled with `Ctrl+Shift+P`. Measurement infrastructure for future optimisations without runtime overhead in production bundles.
- **Interactive terminal pool** — 3-deep `xterm.js` instance pool recycles terminals across tab open/close. Faster tab spawn, lower GC pressure.
- **CodeMirror language cache** — module-level `Map<string, Extension>` short-circuits repeated dynamic imports per file extension. Second-and-subsequent opens of a language are instant.

### Code Quality (Phase 7.3)

The remaining items from Phase 6.3 plus anything picked up along the way. Generic `<List>` component now backs 10 consumers (Branch / Tag / Stash / Reflog / MrPr / Worktree / Submodule / Release / Issue / AiSession). `fetchIntoStore` / `fetchListIntoStore` / `fetchPageIntoStore` helpers consumed by 10 stores. Two residual `serde_json::from_str` call sites in `cli-provider/src/{github,gitlab}/mr_pr.rs` swapped for the shared `run_json` helper.

### E2E Testing (Phase 7.4 + follow-up)

Full WebdriverIO + `tauri-driver` suite covering every major vertical. 9 spec files, ~53 tests: app-launch, navigation, golden-path, and regression suites for graph / branches / staging / terminal / bisect / settings. 6 new page objects, data-testid attributes across the UI, and a Linux `e2e-tests` job in `ci.yml`. Follow-up pass in the same release cycle fixed every layer end-to-end: ESM `__dirname` shim for wdio v9, specs glob resolution, tauri-driver hostname/port, workspace-root binary path, `VITE_BEARDGIT_E2E` frontend hook, and switching to `tauri build --debug --no-bundle` so the frontend actually embeds. A new Docker harness (`e2e/Dockerfile` + `npm run e2e:docker`) lets macOS contributors run the full suite locally in ~1–2 min per iteration.

### Provider Architecture Cleanup (Phase 9)

Pure refactor. `provider/lib.rs` (883 LOC of trait + types + kind + error) split into `traits.rs` / `types.rs` / `kind.rs` / `error.rs` / `http_helpers.rs` / `mock.rs`; `lib.rs` is now 43 LOC of re-exports. `cli-provider/src/{github,gitlab}.rs` (~800 LOC each) converted to directory modules with per-vertical submodules (`mr_pr`, `labels`, `reviewers`, `lifecycle`, `discussions`, `checkout`, `issues`, `releases`). The `impl ForgeProvider` block stays in `mod.rs` as pure delegation to feature-scoped methods — no file exceeds 400 LOC. A CI grep guard in `ci.yml` enforces that `provider` and `forge-provider` never import `reqwest`, `tokio`, `tauri`, or `hyper`. Shared HTTP primitives (`api_error`, `retry_after_secs`, `trim_base_url`) extracted into `provider::http_helpers` and consumed by both `gitlab-api` and `github-api`. `crates/CLAUDE.md` refreshed with the new layout and an "Adding a new forge capability" walkthrough.

### Security

- `npm audit --audit-level=moderate` clean. Override added for `serialize-javascript` (wdio transitive dep) that upstream hadn't patched yet.

### Tooling

- `npm run e2e:docker` (plus `:rebuild` and `:shell`) — one-command local E2E.
- `e2e/README.md` documents the happy-path authoring pattern so test authors have a template.

---

### Phase 6 — Bisect, CLI Auth, AI Views, Multi-Provider, Code Quality

(Previously drafted as a standalone `[0.1.8]` release; folded into the unified `[0.1.8]` cut since it never tagged separately.)

**Git Bisect**

- Visual bisect workflow with good/bad/skip controls and progress indicator
- Auto-bisect mode: provide a test command, BeardGit runs `git bisect run` and reports the culprit
- New `git-engine` bisect module with full lifecycle (start, good, bad, skip, reset, log)
- 8 new Tauri commands, dedicated store, 2 Svelte components (BisectWorkflow, AutoBisectDialog)

**CLI Auth (gh/glab)**

- `gh auth status` and `glab auth status` detection — shows CLI login state in Settings
- Terminal-based login flow: "Login with CLI" opens interactive `gh auth login` / `glab auth login` in a PTY tab
- Unified Authentication settings page combining Token Auth and CLI Auth sections
- New `cli-provider` auth module with status parsing and terminal login commands

**AI Config Editor**

- Dual file tree (project-scoped + user-scoped) showing all AI config files
- Editable CodeMirror pane for settings.json, agents, skills, and CLAUDE.md files
- Create Config dialog for adding new agent/skill/settings files
- 3 new Tauri commands: `ai_get_config_content`, `ai_save_config_content`, `ai_create_config_file`

**AI Sessions**

- Project-scoped session list showing active and recent Claude Code sessions
- File watcher on `~/.claude/sessions/` with auto-refresh on changes
- Session metadata: model, start time, duration, token usage, status (active/completed)

**AI Worktree Enrichment**

- `EnrichedWorktree` type combining git worktree data with AI provider status
- AI badges on worktrees created by Claude Code / Codex / OpenCode
- Context menu with cleanup action for orphaned AI worktrees

**Codex & OpenCode Providers**

- New `codex` crate: full `AiProvider` implementation with binary detection, command building, and config discovery
- New `opencode` crate: full `AiProvider` implementation with binary detection, command building, and config discovery
- Both wired into `app-core` provider factory with automatic detection
- Dynamic terminal dropdown: only shows providers detected on the system
- Codex brand color corrected (#10a37f → #ffffff)

**Structured Error Logging**

- Structured file logging via `tracing` with `tracing-appender` daily rotation
- Logs written to `~/.local/share/com.beardgit.app/logs/` (platform-appropriate data dir)
- New `ErrorDialog` component with copy-error-to-clipboard and open-log-file actions
- All dialogs (Confirm, Clean, CreateMrPr, PatchPreview, TagCreate, CreateWorktree) upgraded with error display

**Composite Tab Upgrade**

- Multi-segment tabs: N terminals + worktrees per project in a single composite tab
- Fixed segment ordering: Project → Worktrees → AI Terminals → Terminals
- Terminal button always adds to the active project's composite tab instead of creating standalone tabs

**Code Quality — commands.rs Split**

- Split monolithic `commands.rs` (3,267 LOC) into 24 feature-based modules under `commands/`
- Modules: advanced, bisect, branch, ci, clean, cli_auth, commit, config, conflict, diff, gitignore, graph, helpers, logging, mod, mr_pr, patch, project, provider_auth, reflog, remote, repository, settings, staging, stash, submodule, tag, theme, worktree
- Extracted shared `dialog.css` (93 lines) replacing duplicated dialog styles across 7 components
- New `fetchIntoStore` utility for consistent store-loading patterns

**E2E Test Infrastructure**

- WebdriverIO + `tauri-driver` configuration for end-to-end testing
- Fixture repo setup script (`e2e/fixtures/setup.sh`) for reproducible test environments
- Page objects: `sidebar.page.ts`, `graph.page.ts`
- Initial specs: `app-launch.spec.ts`, `navigation.spec.ts`

**Bug Fixes & Polish**

- AI Config file tree correctly distinguishes project vs user scope
- AI Sessions auto-cleanup on component destroy (watcher unsubscribe)
- CreateConfigDialog validates file paths and prevents duplicates
- Store helpers centralized with `fetchIntoStore` reducing boilerplate across stores

## [0.1.7] — AI Provider Integration, Changes Redesign, UI Polish

**AI Provider Architecture**

- New `ai-provider` crate: `AiProvider` trait with 17 methods across 7 capability groups (identity, detection, headless execution, specialized actions, interactive launch, session/worktree introspection, config/attribution)
- Shared types: `AiProviderKind`, `AiSession`, `AiWorktree`, `AiConfigFile`, `ExecuteOptions`, `AttributionPattern`
- Trait builds `std::process::Command` objects without executing — execution delegated to `TaskManager` (headless) or `TerminalManager` (interactive)
- Default implementations return empty/None/NotSupported — providers override what they support

**Claude Code (First Provider)**

- New `claude-code` crate implementing `AiProvider` for Claude Code CLI
- Binary detection via `which` + version parsing from `claude --version`
- Repo artifact detection (`.claude/` directory, `CLAUDE.md` file)
- Headless command builder: `--print`, `--output-format`, `--model`, `--max-budget-usd`
- Interactive launch: spawns `claude` binary directly in PTY terminal
- Worktree support: `--worktree [name]` flag
- Session introspection: parses `~/.claude/sessions/*.json`, PID liveness checks (`kill(pid, 0)` on Unix)
- Worktree introspection: `git worktree list --porcelain` parser, filters `worktree-*` branches, status detection (Active/Clean/Orphaned)
- Config discovery: user/project/local settings.json, `.claude/agents/*.md`, `.claude/skills/*/SKILL.md`, CLAUDE.md hierarchy
- Commit attribution: detects `Authored-by:` footer, `Co-authored-by:` trailer with Claude/Anthropic mention, author name matching

**16 Tauri Commands**

- Detection: `ai_get_providers`, `ai_get_repo_status`, `ai_refresh_detection`
- Headless actions (via TaskManager): `ai_generate_commit_message`, `ai_analyze_code`, `ai_generate_pr_description`, `ai_review_code`, `ai_review_pr`
- Interactive launch (via TerminalManager): `ai_launch_interactive`, `ai_launch_worktree`
- Introspection: `ai_list_sessions`, `ai_list_worktrees`, `ai_cleanup_worktree`, `ai_get_config_files`
- Preference: `ai_get_preferred_provider`, `ai_set_preferred_provider`

**AI Provider Settings**

- New "AI Provider" section in Settings replacing the WIP "Editor" section
- Shows all known providers (Claude Code, Codex, OpenCode) with detection status
- Detected providers show version and "Detected" badge; unavailable ones are greyed out
- Click to set default provider, click again to reset to auto-detect
- Preference persisted in `AppConfig.preferred_ai_provider` across restarts
- Refresh button to re-scan PATH for provider binaries

**AI Button Validation**

- AI Commit Message button now shows a warning toast when no staged changes exist
- AI Code Review button now shows a warning toast when no changes exist at all
- Previously both buttons silently triggered tasks with no input

**Terminal AI Launch**

- Terminal dropdown "Claude Code" now calls `ai_launch_interactive` — spawns the `claude` binary directly in PTY (Claude Code starts automatically)
- Terminal tabs show Claude Code SVG brand icon (coral `#d97757`) instead of generic terminal icon
- Brand-colored status dots: Claude (#d97757), Codex (#10a37f), OpenCode (#8b8b8b)
- Same icon treatment in both standalone `TerminalTab` and composite tab terminal segments
- `TerminalTabInfo` extended with optional `provider` field for brand identification

**Changes Section Redesign**

- Pinned commit box at bottom with toolbar row: amend toggle, AI buttons, overflow menu
- AI Commit Message button (purple accent) with loading spinner; Code Review button (blue accent)
- Overflow menu: Create Patch, Clean, History (reflog), Push — replacing scattered buttons
- Commit message textarea with Cmd+Enter shortcut
- Single commit button replacing separate stage+commit actions

**Reflog Section Overhaul**

- Fixed broken "Create Branch" context menu action — was creating branch at HEAD instead of at the reflog entry's commit. New `create_branch_at(name, oid)` backend operation
- Fixed misleading "Checkout" action — was performing `reset --mixed` (destructive). New `checkout_detached(oid)` backend operation for proper detached HEAD checkout
- Fixed selection model — `selectedReflogOid` used just the OID which is not unique across reflog entries. Switched to index-based selection
- Removed duplicate `repo-changed` listeners — SplitView now handles lifecycle exclusively
- Added action buttons to detail pane: Checkout, Create Branch, Reset (dropdown with Soft/Mixed/Hard), Copy SHA
- Added refresh button to list header
- Context menu actions now refresh the reflog list after operations
- Selection cleared when navigating away to prevent stale state on return
- File diff panel: clicking a file in the reflog commit detail now shows a resizable diff editor below

**Submodule Management — Add & Remove**

- New "Add Submodule" button in header — opens inline form with URL and path inputs
- New `add_submodule(url, path)` backend operation (`git submodule add`)
- New "Remove Submodule" in right-click context menu with confirmation dialog
- New `remove_submodule(path)` backend operation (`git submodule deinit -f` + `git rm -f`)
- Empty state no longer blocks the "Add Submodule" button

**UI Polish**

- Folder icons changed from orange to blue for better visual cohesion
- Tab badge style changed from solid orange pill to subtle green tint with green text
- Tab hover tooltips with project snapshot (branch, changes, last commit)
- Project snapshot cache for instant tooltip display
- Task panel command bar truncated to single line with ellipsis (fixes output being pushed off-screen by long AI commands)

**Bug Fixes**

- Fixed task panel output not visible when AI commands have long prompts (command bar had no max-height)
- Fixed `width: 100%` missing on SplitView — right pane not reaching container edge in flex layouts
- Fixed graph tooltip positioning and content
- Fixed terminal resize on tab switch
- Fixed project switch clearing stale data (reflog, conflict state, diffs)
- Fixed unstaged file diff preview not loading after project tab switch
- Removed gitignore editor component (functionality preserved via context menu)

**E2E Test Infrastructure**

- Global vitest setup mocking `@tauri-apps/api/core`, `@tauri-apps/api/event`, `@tauri-apps/api/window`, `@tauri-apps/plugin-dialog`
- Configurable `mockInvokeResponse()` helper for per-test IPC mocking
- 6 E2E workflow test suites: repo-open, staging-commit, branch-ops, tag-ops, stash-ops, ai-provider
- 103 new tests (149 total frontend tests, all passing)

## [0.1.6] — Interactive Terminal Tabs, Composite Tabs, Sidebar Collapse

**Composite Segmented Tabs**

- Project + linked terminal merge into a single segmented pill tab: `[● Repo | ⌨ Terminal]`
- Each segment independently clickable, closeable (hover-only ✕), and middle-click closeable
- Closing a segment reverts the composite to a simple tab (project-only or terminal-only)
- Terminal opens in-place — project tab is promoted to composite, not a new tab at the end
- Shell exit auto-removes the terminal segment, reverting to a simple project tab
- Cmd+W closes the active segment of a composite tab (not the whole tab)
- Standalone terminal tabs remain for "New terminal in ~" (not linked to any project)

**Interactive Terminal Tabs**

- Full interactive xterm.js terminal wired to Rust PTY backend (keyboard input, resize, base64 byte streaming)
- Terminal split button in the actions area: left (terminal icon) opens terminal, right (chevron) opens dropdown
- Dropdown options: "New terminal in ~", Claude Code, Codex, OpenCode — with official SVG brand logos and hardcoded brand colors (#d97757, #10a37f, #8b8b8b)
- Claude logo uses official Anthropic symbol (CC0 public domain from Wikimedia Commons)
- NerdFont icons render correctly in terminal (NerdFontSymbols added to xterm.js fontFamily)
- Cmd+T shortcut to open a new terminal tab
- Terminal tabs auto-close when the shell process exits
- Fetch/Pull/Push buttons hidden when a terminal tab is active

**Sidebar Collapse**

- New collapse toggle button at bottom of sidebar with chevron icon
- Collapsed mode: icon-only (44px width) with smooth 150ms CSS transition
- Tooltips on hover when collapsed
- Cmd+B keyboard shortcut to toggle
- Collapse state persisted in AppConfig across restarts

**Performance**

- Graph viewport cached per project — instant tab switching with no loading spinner for the graph view
- Auto-navigate to graph on project tab switch — prevents stale pipeline/changes data from previous project

**Bug Fixes**

- Fixed: recent projects list empty on first use — now populated when opening a project, not just when closing one
- Fixed: unstaged file diff preview not loading after project tab switch (diffs now auto-refresh on file click)
- Fixed: close button icons inconsistent — standardized to `\uF00D` (nf-fa-times) across all tabs and panels
- Fixed: + button icon inconsistent — standardized to `\uF067` (nf-fa-plus)
- Fixed: icons not vertically centered in Fetch/Pull/Push/Terminal action buttons
- Fixed: tab close buttons oversized with circle hover — now smaller, highlight-only on hover
- Fixed: + button popup not closing when clicking outside
- Sidebar navigation from a terminal tab automatically switches to the most recent project tab

## [0.1.5] — Terminal Core + Theme Redesign

**Terminal Core (xterm.js) + Theme Redesign**

- New `terminal` Rust crate with PTY lifecycle management via `portable-pty`
- Cross-platform shell detection (zsh/bash on Unix, powershell/cmd on Windows)
- `TerminalManager` with spawn, write, resize, kill, kill_all operations
- Tauri commands and event bridge for terminal sessions (base64-encoded byte streaming)
- Reusable `<Terminal>` Svelte component (xterm.js with WebGL, fit, web-links, search addons)
- Read-only xterm.js instance pool (max 3: 2 visible + 1 warm) for zero-lag view switching
- TaskPanel output migrated from manual ANSI-to-HTML to xterm.js read-only terminal
- JobLog (CI pipeline logs) migrated from manual ANSI-to-HTML to xterm.js read-only terminal
- Theme system redesigned: 18 base colors (background + foreground + 16 ANSI) replace 12 semantic colors
- All 14 TOML themes updated with explicit ANSI color palettes
- Semantic UI colors now auto-derived from base palette (DerivedColors struct)
- Direct xterm.js ITheme mapping from base colors (no derivation needed for terminal)
- Retired `ansi.ts` (250+ lines) — replaced by native xterm.js rendering + lightweight `stripAnsi()` utility

**Auto-Update System**

- Tauri updater plugin checks GitHub Releases for updates on app launch
- Two-step update flow: toast notification → Download → Restart (non-disruptive)
- Download progress shown in toast with percentage
- Updater signing keys configured in CI release workflow

**Toast Notifications**

- Reusable toast notification system (bottom-right, max 3, stackable)
- Types: success, error, warning, info with auto-dismiss
- Used by auto-updater, extensible for future notifications

**Multi-File Selection in Changes**

- Per-file checkboxes in both staged and unstaged file lists
- Select All header checkbox with indeterminate state
- Header action swaps contextually: Stage All / Stage Selected (N) and Unstage All / Unstage Selected (N)
- Selection clears on refresh

**Bug Fixes**

- Commits now use git config identity (user.name/user.email) instead of hardcoded author
- Untracked directories show individual files instead of collapsed folder entry (recurse_untracked_dirs)
- README prerequisites and architecture table accuracy fixes

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

- GitHub/GitLab CLI OAuth login disabled until terminal integration (PAT-only for now)
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
