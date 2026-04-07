# BeardGit

## Current State

**v0.1.0** — MVP complete. Theme system (4 built-in + user TOML themes, OS auto-detection). CI/CD verified cross-platform.

## Project Overview

Cross-platform desktop Git client with GitLab + GitHub CI integration. Tauri v2 (Rust) + Svelte 5 (TypeScript). Canvas-based graph supporting 100K+ commits.

## Architecture

**Three layers:** Rust Core → Tauri IPC → Svelte Frontend

| Crate | Purpose |
|---|---|
| `git-engine` | Git ops — libgit2 for reads, system git CLI for writes (merge, rebase, push/pull) |
| `graph-builder` | Pure DAG construction, lane assignment, viewport slicing (no I/O) |
| `provider` | `CiProvider` trait, unified CI types, `parse_remote_url()` |
| `gitlab-api` | GitLab REST v4 — implements `CiProvider` |
| `github-api` | GitHub REST API — implements `CiProvider` |
| `auth` | PAT validation + AES-256-GCM encrypted credential store (machine-bound) |
| `storage` | SQLite (commits cache) + JSON config + TOML themes |
| `task-runner` | Async background tasks with cancellation + streaming output |
| `watcher` | Debounced filesystem events via `notify` (500ms, filters `.git/`) |
| `app-core` | 75 Tauri commands, `AppState`, event bridge — **only crate coupled to Tauri** |

**Frontend:** SPA with no file-based routing — all views switched via `activeView` state in `+page.svelte`. Stores in `src/lib/stores/`, IPC in `src/lib/api/tauri.ts`, types in `src/lib/types/index.ts`.

**IPC flow:** Svelte store action → `tauri.ts` wrapper → `@tauri-apps/api/core invoke()` → Rust `#[tauri::command]` → domain crate → `Result<T, String>` back to frontend.

**State model:** `AppState` is a `Mutex`-wrapped singleton managing multi-tab repos. Heavy state (`Repository`, `GraphLayout`, `RepoWatcher`) is lazily loaded per active tab only. Network ops (fetch/pull/push) are fire-and-forget via `TaskManager` → `TaskId` returned immediately → output streamed via events.

## Development Commands

```bash
npm install                    # Install frontend deps
npm run tauri dev              # Dev mode (Rust + Svelte)
cargo test --workspace         # All Rust tests
cargo test -p <crate>          # Single crate tests
cargo check --workspace        # Type-check all Rust
cargo fmt --all                # Format Rust (CI enforces)
cargo clippy --workspace       # Lint Rust (CI enforces)
npm run check                  # svelte-check
npm run test                   # vitest (frontend)
```

**Pre-push verification (run before every push):**
```bash
cargo fmt --all -- --check && cargo clippy --workspace && cargo test --workspace && npm run check && npx vitest run
```

## Git Workflow

**CRITICAL — follow this flow for ALL changes. No exceptions.**

1. **Create a branch from `beta`**: `feature/`, `fix/`, `bug/`, `chore/`
   - **Always branch from `beta`** — never from `main`
   - **NEVER commit directly to `main` or `beta`**
2. **Develop and test** on the branch
3. **Before the final commit**, run the full verification suite and fix any issues:
   ```bash
   cargo fmt --all -- --check && cargo clippy --workspace && cargo test --workspace && npm run check && npx vitest run
   ```
   - Fix all fmt/clippy/test failures **before committing** — the goal is that every pushed commit passes CI
   - If fmt fails: `cargo fmt --all` then re-stage
   - If clippy fails: fix warnings, re-stage
   - If tests fail: fix the code, re-stage
4. **Push the branch** — CI runs on `feature/**` branches
5. **Wait for ALL CI checks to pass** (fmt, clippy, tests, svelte-check)
6. **Get explicit user approval** — do NOT merge without it
7. **Squash-merge to `beta`** — reset beta if it has broken commits, then single clean commit
8. **Verify Build pipeline** passes on beta (macOS arm64/x64, Linux, Windows)
9. **When beta is stable**, user requests promotion to `main`

**NEVER push without asking the user first.** Push is a shared-state action that requires confirmation every time.

**Merge to beta checklist:**
- [ ] All CI checks pass on feature branch
- [ ] User has given explicit approval
- [ ] Squashed to single commit on beta
- [ ] Build pipeline passes on beta

## Commit Conventions

- **Format:** `type(scope): description`
- **Types:** `feat:`, `fix:`, `chore:`, `refactor:`, `perf:`, `test:`
- **Scope:** crate or module name — `feat(git-engine):`, `fix(storage):`, `feat(frontend):`
- **Footer:** `Authored-by: Adolfo Fuentes <adolfofuentes@metricool.com>`

## Engineering Principles

1. **Performance first.** Never O(n^2) when O(n) is obvious. Profile before micro-optimizing.
2. **Reuse over rewrite.** Search existing types, utils, components, styles, and patterns before creating new ones. If something similar exists, extend or extract a shared abstraction.
3. **Responsive UI.** Never block main thread. Git ops go through task system or `spawn_blocking`. Batch rapid DOM updates with `requestAnimationFrame`. Debounce user-triggered input (search, filters).
4. **Prefer events over polling.** Use Tauri event bridge where possible. When polling is unavoidable, always auto-stop on terminal state and use the shortest acceptable interval.
5. **Tauri isolation.** Only `app-core` depends on Tauri. All other crates are reusable libraries.

## Key Dependencies

**Rust:** `git2` (vendored-openssl), `rusqlite` (bundled), `reqwest` (rustls-tls) + `tokio`, `serde`, `thiserror`, `toml`, `tauri` v2, `notify` 7, `aes-gcm`
**Frontend:** `svelte` 5, `@tauri-apps/api` v2, `@inlang/paraglide-js` 2.x, `vitest`

## CI/CD Pipelines

| Pipeline | Trigger | Checks |
|---|---|---|
| **CI** (`ci.yml`) | main, beta, feature/** | fmt, clippy, tests, svelte-check |
| **Build** (`build.yml`) | beta push | macOS arm64/x64, Linux x64, Windows x64 |
| **Release** (`release.yml`) | `v*` tags | Build matrix + GitHub Releases |
| **Security** (`security.yml`) | main/beta + weekly | cargo audit + npm audit |

## Module-Specific Instructions

Detailed conventions for each layer are in their respective CLAUDE.md files:
- **Rust crates:** `crates/CLAUDE.md`
- **Svelte frontend:** `src/CLAUDE.md`
- **Tauri integration:** `src-tauri/CLAUDE.md`

## License

CC BY-NC-SA 4.0
