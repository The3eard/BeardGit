# Rust Crates

## Workspace Conventions

- **Edition 2024**, all crates
- **Error handling:** `thiserror` for all custom error enums. Pattern: domain-specific variants + `#[from]` for upstream library errors
- **Serialization:** `serde` on all IPC-facing types. Enums use `snake_case` or `lowercase` rename. `TaskStatus` uses `#[serde(tag = "state")]` (internally tagged)
- **No `unsafe` code** anywhere in the workspace
- **No feature flags** — only platform detection via `#[cfg(target_os)]` in `auth`
- **Tests:** `#[cfg(test)] mod tests` in the same file, `tempfile` for filesystem tests, `Database::open_in_memory()` for DB tests
- **Performance:** Prefer `&str` over `String` cloning in hot paths. Use `Arc<T>` for shared ownership — avoid unnecessary `.clone()` on large data structures. Check existing types in `provider` and `storage` before defining new ones

## Crate Responsibilities

### `git-engine`
- **Hybrid approach:** `git2` (libgit2) for reads, system `git` binary (`std::process::Command`) for writes (merge, rebase, cherry-pick, revert, stash, push/pull). System git **must be on PATH**.
- `Repository` is not `Clone`/`Send`/`Sync` — callers must wrap in `Mutex` (which `AppState` does)
- `tag_cache: Mutex<Option<Vec<TagInfo>>>` — lazy-populated, invalidated on create/delete
- Multiple `impl Repository` blocks spread across modules, sharing struct definition in `repository.rs`
- `String::from_utf8_lossy` on git CLI output — binary output is silently mangled
- Error type: `GitError` — `Git(git2::Error)`, `RepoNotFound(String)`, `Io(io::Error)`

### `graph-builder`
- **Pure computation** — no I/O, no async, no platform code. Only depends on `serde`
- Two-pass DAG: pass 1 creates nodes, pass 2 adds bidirectional child links
- `HashMap<String, DagNode>` for O(1) OID lookup + `Vec<String>` for insertion order
- Consumed by `app-core` on `spawn_blocking` threads with `rayon` parallelism

### `provider`
- Defines `CiProvider` trait — the contract for all CI backends
- All shared CI types live here: `CiRun`, `CiRunDetail`, `CiStage`, `CiJob`, `CiStatus`, `ProviderKind`
- `parse_remote_url()` handles SSH, HTTPS, well-known hosts, self-hosted, GitLab subgroups
- `log_preprocessor` lives here (not in API crates) to avoid circular dependency — needs `ProviderKind` to dispatch
- `CiStatus::Manual` is GitLab-only; `CiStatus::TimedOut` is GitHub-only
- Error type: `ProviderError` — `Http(String)`, `Api { status, message }`, `Json(String)`, `RateLimited { retry_after_secs }`

### `gitlab-api` / `github-api`
- Both implement `CiProvider` trait from `provider` crate
- Both use `danger_accept_invalid_certs(true)` for self-hosted instances with self-signed certs
- **GitHub:** auto-rewrites `github.com` → `api.github.com` in `normalize_url()`. No native stages — jobs grouped under virtual `"Jobs"` stage. Duration computed from timestamp arithmetic. Rate limit detected via `x-ratelimit-remaining` header
- **GitLab:** project paths URL-encoded via `urlencoding::encode()`. Duration provided natively. `group_jobs_by_stage()` uses linear search (acceptable for ~10-50 jobs)
- Error conversion: internal `ApiError` → `ProviderError` at the trait boundary via `into_provider_error()`
- **GitLab gotcha:** `pub use types::*` glob re-export — only crate with this pattern

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
- Single-file crate (~57 lines), single struct `RepoWatcher`
- `.git/` events explicitly filtered out to prevent spurious refreshes
- Debounce hardcoded at 500ms
- Uses `std::sync::mpsc` + `std::thread::spawn` (NOT tokio) — OS thread for event loop
- Drop semantics = shutdown (dropping `RepoWatcher` stops watching)
- `notify::RecommendedWatcher` is platform-auto-selected (kqueue/inotify/ReadDirectoryChangesW)

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
