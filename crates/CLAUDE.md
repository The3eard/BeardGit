# Rust Crates

## Workspace Conventions

- **Edition 2024**, all crates
- **Error handling:** `thiserror` for all custom error enums. Pattern: domain-specific variants + `#[from]` for upstream library errors
- **Serialization:** `serde` on all IPC-facing types. Enums use `snake_case` or `lowercase` rename. `TaskStatus` uses `#[serde(tag = "state")]` (internally tagged)
- **No `unsafe` code** anywhere in the workspace
- **Feature flags are rare.** Only used for platform detection via `#[cfg(target_os)]` in `auth`, and for gated `mock` test stubs in `provider` + `forge-provider`.
- **Tests:** `#[cfg(test)] mod tests` in the same file, `tempfile` for filesystem tests, `Database::open_in_memory()` for DB tests
- **Performance:** Prefer `&str` over `String` cloning in hot paths. Use `Arc<T>` for shared ownership — avoid unnecessary `.clone()` on large data structures. Check existing types in `provider` and `storage` before defining new ones

## Crate Responsibilities

### `git-engine`
- **Hybrid approach:** `git2` (libgit2) for reads, system `git` binary (`std::process::Command`) for writes (merge, rebase, cherry-pick, revert, stash, push/pull). System git **must be on PATH**.
- `Repository` is not `Clone`/`Send`/`Sync` — callers must wrap in `Mutex` (which `AppState` does)
- `tag_cache: Mutex<Option<Vec<TagInfo>>>` — lazy-populated, invalidated on create/delete
- Multiple `impl Repository` blocks spread across modules, sharing struct definition in `repository.rs`
- `String::from_utf8_lossy` on git CLI output — binary output is silently mangled
- `bisect` module: full git bisect lifecycle (start, good, bad, skip, reset, log) via system git CLI
- Error type: `GitError` — `Git(git2::Error)`, `RepoNotFound(String)`, `CliError(String)`, `Io(io::Error)`

### `graph-builder`
- **Pure computation** — no I/O, no async, no platform code. Only depends on `serde`
- Two-pass DAG: pass 1 creates nodes, pass 2 adds bidirectional child links
- `HashMap<String, DagNode>` for O(1) OID lookup + `Vec<String>` for insertion order
- Consumed by `app-core` on `spawn_blocking` threads with `rayon` parallelism

### `provider`
- Defines `CiProvider` trait — the contract for all CI backends
- Module layout (Phase 9.1): `lib.rs` is pure re-exports, `traits.rs` holds the async `CiProvider` trait, `types.rs` holds the shared CI + user + project + trigger/workflow types, `kind.rs` holds `ProviderKind` + `parse_remote_url`, `error.rs` holds `ProviderError`, `http_helpers.rs` holds pure reqwest-free HTTP helpers (`api_error`, `retry_after_secs`, `trim_base_url`), `log_preprocessor.rs` stays (needs `ProviderKind` to dispatch), `mock.rs` is gated behind the `mock` feature
- **Trait-crate purity:** must stay free of `reqwest`, `tokio` runtime imports, `tauri`, `hyper` — enforced by a CI grep guard in `ci.yml`. Dev-dependencies (e.g. `tokio` for `#[tokio::test]`) are fine; source-code imports are not
- `parse_remote_url()` handles SSH, HTTPS, well-known hosts, self-hosted, GitLab subgroups
- `CiStatus::Manual` is GitLab-only; `CiStatus::TimedOut` is GitHub-only
- Error type: `ProviderError` — `Http(String)`, `Api { status, message }`, `Json(String)`, `RateLimited { retry_after_secs }`, `NotSupported`
- `MockCiProvider` (behind `mock` feature) returns canned empty lists + a stub user for integration tests

### `forge-provider`
- Defines `ForgeProvider` trait — the contract for MR/PR + issues + releases + labels + reviewers backends
- Same trait-crate purity rules as `provider` (enforced by the same CI guard)
- `ForgeError` variants: `NotFound`, `NotSupported`, `Cli`, `Io` — string-wrapped
- `mock` feature flag exposes `MockProvider` returning empty data / `NotSupported`
- All optional methods (labels, reviewers, lifecycle, discussions, checkout, issues, releases) have default `NotSupported` impls so implementations opt-in

### `gitlab-api` / `github-api`
- Both implement `CiProvider` trait from `provider` crate
- Both use `danger_accept_invalid_certs(true)` for self-hosted instances with self-signed certs
- Both delegate base-URL trimming and rate-limit reset arithmetic to `provider::http_helpers`
- **GitHub:** auto-rewrites `github.com` → `api.github.com` in `normalize_url()`. No native stages — jobs grouped under virtual `"Jobs"` stage. Duration computed from timestamp arithmetic. Rate limit detected via `x-ratelimit-remaining` header, retry-after computed by `http_helpers::retry_after_secs`
- **GitLab:** project paths URL-encoded via `urlencoding::encode()`. Duration provided natively. `group_jobs_by_stage()` uses linear search (acceptable for ~10-50 jobs)
- Error conversion: internal `ApiError` → `ProviderError` at the trait boundary via `into_provider_error()`
- **GitLab gotcha:** `pub use types::*` glob re-export — only crate with this pattern

### `cli-provider`
- CLI-backed `ForgeProvider` implementations that wrap the bundled `gh` / `glab` binaries
- Module layout (Phase 9.2): `github/` and `gitlab/` are **directories**, not single files. Each contains `mod.rs` (struct definition + pure-delegation `impl ForgeProvider`) plus per-feature submodules:
  - `mr_pr.rs` — list / get / diff / create / edit / merge / close / approve / request-changes / comment
  - `labels.rs` — label add/remove/list
  - `reviewers.rs` — reviewer add/remove (GitLab computes the new set locally because `glab mr update --reviewer` replaces)
  - `lifecycle.rs` — ready / draft / reopen
  - `discussions.rs` — (GitLab only) resolve/unresolve
  - `checkout.rs` — MR/PR checkout + stdout parser, colocated with the feature
  - `issues.rs` — issue CRUD + argv-builder helpers colocated
  - `releases.rs` — release CRUD, delegates to shared `crate::releases` for argv + parser helpers
- Per-feature submodules add `pub(super) fn *_impl(…)` methods to `impl GitHubCli` / `impl GitLabCli`; the trait impl in `mod.rs` is one-line delegation only. This keeps trait methods scannable and lets each feature evolve inside a single file
- Shared helpers at the crate root: `runner.rs` (run / run_json / run_with_stdin), `parsers.rs` (JSON parsing — uses shared `parse_mr_pr` with per-forge field maps), `releases.rs` (argv + parser helpers used by both providers), `error.rs` (`CliError` → `ForgeError` via `#[from]`), `auth.rs` (`gh`/`glab auth status` + OAuth scaffolding)
- Target: **no file >400 LOC**. Shared helpers can be larger (`parsers.rs`, `releases.rs`) because they're the shared surface, not a per-feature vertical

### `auth`
- AES-256-GCM encryption with HKDF-SHA256, machine-bound keys (credentials useless if copied)
- **Platform-specific** `machine_key.rs`: macOS (`ioreg`), Linux (`/etc/machine-id`), Windows (registry). No fallback for unsupported platforms — won't compile
- `CredentialStore` read-modify-write is non-atomic (documented as safe for single-credential MVP)
- `CredentialStore::with_key()` test constructor bypasses machine ID for portable tests
- Error type: `AuthError` — all string-wrapped variants to avoid leaking internal types

### `storage`
- SQLite schema migrations via `PRAGMA user_version` — currently v1 only
- `commits_cache` stores `parents` and `refs` as JSON strings in TEXT columns (not normalized)
- Built-in themes compiled via `include_str!()`: `github_dark/light.toml`, `gitlab_dark/light.toml`
- `AppConfig` has **three-tier migration** for legacy provider formats (pre-Plan5, Plan5, current). Legacy fields use `#[serde(default, skip_serializing)]`
- `AppConfig::load` returns `Default::default()` when file missing — safe startup
- `logging` module: structured file logging via `tracing` + `tracing-appender` with daily rotation, platform-specific log directories, debug info collection
- Error type: `StorageError` — `Sqlite`, `Io`, `Json` (all `#[from]`)

### `task-runner`
- `TaskManager` uses `tokio::sync::Mutex` (async) + `AtomicU64` for ID generation
- Cancellation via `tokio_util::sync::CancellationToken` — only for tasks with `cancellable = true`
- `TaskManager::spawn` takes `self: &Arc<Self>` — Arc clone moved into spawned task
- `OutputLine.timestamp` is `std::time::Instant` with `#[serde(skip)]` — not meaningful across processes
- `TaskStatus::Queued` exists but unused — tasks go directly to `Running`
- Decoupled from Tauri via `TaskEventSink` trait
- Error type: `TaskError` — `NotFound`, `NotRunning`, `NotCancellable`, `Io`

### `watcher`
- `RepoWatcher`: debounced filesystem events via `notify` (500ms), `.git/` filtered out
- `AiSessionWatcher`: watches `~/.claude/sessions/` for session file changes, emits events for AI Sessions view
- Uses `std::sync::mpsc` + `std::thread::spawn` (NOT tokio) — OS thread for event loop
- Drop semantics = shutdown (dropping watcher stops watching)
- `notify::RecommendedWatcher` is platform-auto-selected (kqueue/inotify/ReadDirectoryChangesW)

### `codex`
- `AiProvider` implementation for Codex CLI
- Binary detection via `which codex`, version parsing from `codex --version`
- Command building for headless execution and interactive terminal launch
- Config discovery: `codex.toml`, `~/.config/codex/` settings

### `opencode`
- `AiProvider` implementation for OpenCode CLI
- Binary detection via `which opencode`, version parsing from `opencode --version`
- Command building for headless execution and interactive terminal launch
- Config discovery: `opencode.json`, `~/.config/opencode/` settings

## Error Handling by Layer

| Layer | Pattern |
|---|---|
| Domain crates | Custom `thiserror` enums with typed variants |
| `app-core` commands | `Result<T, String>` — `.to_string()` on domain errors (Tauri IPC convention) |
| Frontend | Catches string errors from IPC, displays in UI |

## Adding a New Crate

1. Create under `crates/<name>/`
2. Add to workspace `Cargo.toml` members
3. Use `thiserror` for errors, `serde` on IPC types
4. Keep Tauri-free — only `app-core` should depend on Tauri
5. Tests in same file with `#[cfg(test)]`, use `tempfile` for filesystem needs
6. Add doc comments on all public types and functions

## Adding a New Forge Capability

End-to-end walkthrough for adding a new MR/PR / issue / release / … operation that the UI calls through to `gh` or `glab`. Follow each step in order; every step corresponds to a single layer in the stack and skipping one will break the IPC chain.

1. **Trait method on `ForgeProvider`** (`crates/forge-provider/src/lib.rs`)
   - Add the method signature with a `NotSupported` default impl so existing implementations keep compiling
   - Add/extend any new types (e.g. `CreateThingInput`, `ThingDetail`) in `crates/forge-provider/src/types.rs` — derive `Debug, Clone, Serialize, Deserialize`, use `snake_case` rename for enums
   - Update `MockProvider` in `mock.rs` if the return type is non-trivial

2. **GitHub impl** (`crates/cli-provider/src/github/<vertical>.rs`)
   - Pick the right submodule: `mr_pr` / `labels` / `reviewers` / `lifecycle` / `checkout` / `issues` / `releases`
   - Add a `pub(super) fn <name>_impl(…)` method to `impl GitHubCli` that invokes `gh` via `self.run(&[...])` / `self.run_json(&[...])` / `self.run_with_stdin(&[...], …)`
   - Keep argv-builder helpers and stdout/JSON parsers in the **same file** as the `_impl` method that uses them — this is the whole point of the vertical split
   - Add a one-line delegation in `github/mod.rs`: `fn <name>(…) { self.<name>_impl(…) }`
   - Unit-test argv builders + parsers with fixtures (see `tests/fixtures/` pattern)

3. **GitLab impl** (`crates/cli-provider/src/gitlab/<vertical>.rs`) — same shape as step 2
   - Some capabilities are GitLab-specific (discussions) — skip the GitHub impl and let the trait default return `NotSupported`
   - Some capabilities are GitHub-specific (draft releases) — GitLab returns `ForgeError::NotSupported` explicitly from the impl, not the default

4. **Tauri command in `app-core`** (`crates/app-core/src/commands/forge_*.rs`)
   - Add `#[tauri::command] async fn forge_<verb>(state: State<AppState>, …) -> Result<T, String>`
   - Use `spawn_blocking` — the CLI runner is sync, the Tauri command boundary is async
   - Convert `ForgeError` → `String` via `.to_string()` at the boundary (Tauri IPC convention)
   - Register the command in `lib.rs`'s `.invoke_handler()` list

5. **TypeScript type** (`src/lib/types/index.ts`)
   - Mirror the Rust type (tag-for-tag field names since `serde` uses `snake_case` and TS is also `snake_case` here)
   - Export from the barrel so stores and components can import from `$lib/types`

6. **IPC wrapper** (`src/lib/api/tauri.ts`)
   - Add `forge<Verb>(…): Promise<T>` — thin wrapper around `invoke('forge_<verb>', { … })`
   - Keep parameter names exactly matching the Rust command's argument names (Tauri auto-camelCases)

7. **Store action** (`src/lib/stores/forgeStore.ts` or the relevant feature store)
   - Call the IPC wrapper, handle the `string` error by setting store error state
   - Update the reactive state so the UI reflects the new data

8. **i18n keys** — add English + Spanish pairs
   - `src/lib/i18n/messages/en-US/<feature>.json`
   - `src/lib/i18n/messages/es-ES/<feature>.json`
   - Never hardcode user-visible strings in components — always through paraglide

9. **Component wiring** (`src/lib/components/<Feature>/*.svelte`)
   - Call the store action on user interaction
   - Reuse existing button/toast/modal primitives — check `src/lib/components/ui/` before building new ones
   - Wire the i18n keys with `m.the_key()` from paraglide

10. **Verify**
    ```bash
    cargo fmt --all -- --check && cargo clippy --workspace && cargo test --workspace && npm run check && npx vitest run
    ```
