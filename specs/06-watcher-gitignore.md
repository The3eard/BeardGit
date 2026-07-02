# Spec 06 — Teach the watcher about .gitignore

**Priority:** P1 · **Effort:** M (2–4 days incl. Linux verification) · **Branch:** `perf/watcher-gitignore` · **Depends on:** —

## Problem

`path_is_relevant` (`crates/watcher/src/lib.rs:38-61`) treats **any** path outside `.git/` as relevant — no ignore check — and the watch is registered recursively over the whole worktree. Every debounced 500 ms batch containing a "relevant" event triggers `Snapshot::capture` (`crates/mutation-events/src/snapshot.rs:56-`): open repo, walk all refs, `stash_foreach`, worktrees, remotes, **full status walk**.

Consequences:

- A `cargo build` / `npm install` / webpack writing thousands of files under `target/`・`node_modules/` wakes the watcher every 500 ms for the duration and pays a full refs+status walk each tick. The UI is protected (the empty-diff gate holds, no refresh storm reaches the frontend) — but it's sustained background CPU on exactly the machines (dev machines, big repos) the app targets.
- **Linux watch exhaustion:** recursive inotify registers a descriptor per subdirectory; `target/`+`node_modules/` can blow the limit. `open_repo` already anticipates this and logs "repo watcher failed to start" (`crates/app-core/src/commands/repository.rs:62-66`) — live refresh silently dies for that repo. Not watching ignored trees prevents the exhaustion instead of logging it.

## Goal (success criteria)

- Events whose paths are all git-ignored trigger **zero** `Snapshot::capture` calls (observable via a `tracing` counter).
- On Linux, opening a repo with a multi-GB `target/` registers watches only on non-ignored directories — watcher starts successfully where it previously hit the limit.
- Live refresh still fires for: tracked-file edits, new untracked (non-ignored) files, `.git/` ref changes, and edits to `.gitignore` itself (which must invalidate the matcher).

## Design

Use the `ignore` crate's `gitignore::Gitignore` matcher (no repo handle needed, honors nested `.gitignore` + `.git/info/exclude` + global excludes when built via `GitignoreBuilder`) — cheaper and lock-free vs calling `git2::Repository::is_path_ignored` per event from the watcher thread.

1. **Batch filtering (portable core):** build the matcher once per repo at watcher start; in the debounce handler, drop ignored paths from the batch *before* the relevance check; if the batch empties, skip `Snapshot::capture` entirely. Rebuild the matcher when a batch touches any `.gitignore` / `.git/info/exclude` (then process that batch with the new matcher, conservatively treating it as relevant once).
2. **Selective watch registration (Linux-focused):** where the notify backend enumerates directories (inotify), register non-recursively per directory and skip ignored dirs during the walk; new directories arriving in events get registered on the fly if non-ignored. On macOS (FSEvents is naturally cheap and path-prefix based) and Windows (single ReadDirectoryChangesW handle), keep recursive registration + batch filtering only — don't complicate platforms that don't need it.
3. Keep the existing 500 ms debounce and the `.git/` special-casing unchanged.

Edge cases to test: negated patterns (`!keep.me` inside an ignored dir — matcher handles, selective registration must not skip a dir containing negations → only skip dirs that are themselves matched with no descendant negation; when in doubt, watch it and rely on batch filtering), case-insensitive filesystems, symlinked dirs.

## Files to touch

- `crates/watcher/src/lib.rs` (matcher, filtering, per-backend registration), `Cargo.toml` (workspace dep `ignore`).
- `crates/app-core/src/commands/repository.rs` (pass repo root/config to watcher construction if not already).
- Tests in `crates/watcher` (currently the weakest-tested crate at 4 tests — this work brings fixtures that fix that).

## Verification

1. Unit tests: ignored-path batch → no capture; `.gitignore` edit → matcher rebuild; negation pattern respected.
2. Manual (macOS): run `cargo build` in a watched repo, watch the capture counter stay flat; touch a tracked file mid-build → refresh still fires.
3. Manual (Linux VM/CI): repo with huge `target/`, lowered `fs.inotify.max_user_watches` → watcher starts, live refresh works.

## Out of scope

- Cheapening `Snapshot::capture` itself (worth a follow-up: skip the full status walk when the batch only touched `.git/refs`) — smaller win once ignored churn is filtered.
