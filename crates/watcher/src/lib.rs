//! Debounced filesystem watchers for BeardGit.
//!
//! [`RepoWatcher`] watches a git repository working tree for changes and,
//! after a brief quiet period, emits a `project-mutated` Tauri event with
//! [`MutationKind::External`] whenever the on-disk state actually changed
//! (per the [`Snapshot`] diff). Most events inside `.git/` are filtered
//! out to avoid spurious refreshes, but changes to `.git/refs/` and
//! `.git/HEAD` are allowed through so that external commits and branch
//! switches are detected.
//!
//! [`AiSessionWatcher`] watches AI provider transcript / rollout
//! directories (e.g. `~/.claude/projects/`, `~/.codex/sessions/`) and
//! fires on any change, without filtering.

mod ai_config;
mod ai_sessions;
pub use ai_config::{AiConfigChange, AiConfigWatcher, ConfigChangeScope};
pub use ai_sessions::AiSessionWatcher;

use mutation_events::{MutationFlags, MutationKind, Snapshot, emit_mutation};
use notify_debouncer_mini::new_debouncer;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tauri::AppHandle;

/// Whether a path is relevant for UI refresh based on its location alone.
///
/// Working-tree changes (outside `.git/`) pass this classifier; the git-ignore
/// filter that drops `target/`・`node_modules/` churn is applied separately in
/// [`classify_batch`], which needs a repo handle. Inside `.git/`, only entries
/// that change on real repo mutations are relevant:
///  - `refs/**`, `HEAD`: commits, branch create/delete, checkout
///  - `packed-refs`: refs packed by gc / fetch / branch -d of a packed ref
///  - `index`: external stage/unstage/reset (writes `.git/index` only — the
///    working tree is untouched, so nothing else in the batch is relevant)
///  - `FETCH_HEAD` / `MERGE_HEAD` / `ORIG_HEAD`: fetch / merge / rebase flows
///  - `info/exclude`: repo-local ignore rules — a change can alter which
///    working-tree files are ignored, so refresh conservatively
///
/// Kept as a pure classifier so the allowlist can be unit-tested without
/// constructing a `DebouncedEvent`.
fn path_is_relevant(path: &Path) -> bool {
    let components: Vec<_> = path.components().collect();

    // Find the .git component index
    let git_idx = components.iter().position(|c| c.as_os_str() == ".git");

    let Some(idx) = git_idx else {
        // Not inside .git/ — a working-tree path. The git-ignore decision is
        // made by `classify_batch` (it needs a repo handle), not here.
        return true;
    };

    let after_git: Vec<_> = components[idx + 1..].iter().collect();
    let first = after_git.first().and_then(|c| c.as_os_str().to_str());

    // `.git/info/exclude` holds repo-local ignore rules; only that one file
    // under `.git/info/` matters (attributes, dumb-HTTP `refs`, etc. do not).
    if first == Some("info") {
        return after_git.get(1).and_then(|c| c.as_os_str().to_str()) == Some("exclude");
    }

    matches!(
        first,
        Some("refs" | "HEAD" | "packed-refs" | "index" | "FETCH_HEAD" | "MERGE_HEAD" | "ORIG_HEAD")
    )
}

/// Whether any component of `path` is a `.git` directory.
fn is_under_git_dir(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == ".git")
}

/// Whether git ignores `path` (given as the absolute path the watcher
/// reported). `is_path_ignored` matches `target/`-style rules against bare
/// directories and honours the full parent hierarchy, so a file deep inside an
/// ignored tree is reported ignored even when only an ancestor directory
/// matches — and, unlike a single flattened matcher, it also honours nested
/// `.gitignore` files and negation (`!keep.me`) patterns.
fn path_is_ignored(repo: &git2::Repository, repo_root: &Path, path: &Path) -> bool {
    // libgit2 wants the path relative to the work directory.
    let rel = path.strip_prefix(repo_root).unwrap_or(path);
    repo.is_path_ignored(rel).unwrap_or(false)
}

/// Outcome of classifying one debounced batch.
struct BatchClassification {
    /// At least one path survived filtering → the batch warrants a capture.
    relevant: bool,
    /// How many paths survived (kept) — for the debug log.
    kept: usize,
    /// Total paths in the batch — for the debug log.
    total: usize,
}

/// Decide whether a debounced batch warrants a [`Snapshot::capture`].
///
/// `.git/` paths keep their existing allowlist ([`path_is_relevant`]);
/// working-tree paths are dropped when git considers them ignored (`target/`,
/// `node_modules/`, build output …). The repo is opened **at most once for the
/// whole batch**: batches are ~500 ms apart, so a per-batch `git2::Repository`
/// open is cheap, and — unlike a matcher cached at watcher start — it always
/// reflects the current `.gitignore` / `.git/info/exclude` / global excludes,
/// including nested `.gitignore` files. A touched (non-ignored) `.gitignore`
/// or `.git/info/exclude` naturally survives as relevant, so editing ignore
/// rules still refreshes the UI.
fn classify_batch(paths: &[PathBuf], repo_root: &Path) -> BatchClassification {
    let total = paths.len();

    // Only open the repo if a working-tree path actually needs an ignore check.
    let needs_repo = paths.iter().any(|p| !is_under_git_dir(p));
    let repo = if needs_repo {
        git2::Repository::open(repo_root).ok()
    } else {
        None
    };

    let mut kept = 0usize;
    for path in paths {
        if !path_is_relevant(path) {
            continue; // filtered .git/ noise (objects, logs, COMMIT_EDITMSG …)
        }
        // A surviving working-tree path is dropped only if git ignores it. If
        // the repo failed to open (e.g. mid-init), keep the path — the
        // pre-ignore-filter behaviour — rather than risk missing a refresh.
        if !is_under_git_dir(path)
            && let Some(repo) = &repo
            && path_is_ignored(repo, repo_root, path)
        {
            continue;
        }
        kept += 1;
    }

    BatchClassification {
        relevant: kept > 0,
        kept,
        total,
    }
}

/// What a debounced batch resolved to after filtering + diffing.
#[derive(Debug)]
enum BatchOutcome {
    /// Every path was ignored or irrelevant `.git/` noise — no capture ran.
    Skipped,
    /// A snapshot was captured but the diff was empty — nothing to emit.
    Unchanged,
    /// Repo state changed — emit these flags.
    Mutated(MutationFlags),
}

/// Filter the batch, and (only if it survives) capture + diff against the
/// cached snapshot. Split out of the debounce thread so tests can drive the
/// exact hot path and observe that an all-ignored batch performs **zero**
/// captures (returns [`BatchOutcome::Skipped`]).
fn evaluate_batch(paths: &[PathBuf], repo_root: &Path, cached: &Mutex<Snapshot>) -> BatchOutcome {
    let class = classify_batch(paths, repo_root);
    if class.kept != class.total {
        // Field-debugging aid: surfaces build/install churn being filtered.
        tracing::debug!(
            kept = class.kept,
            dropped = class.total - class.kept,
            total = class.total,
            "watcher dropped ignored paths from batch"
        );
    }
    if !class.relevant {
        return BatchOutcome::Skipped;
    }

    // Acquire the cache lock first so a concurrent caller mid-internal-mutation
    // (holding the same lock, about to overwrite the cache) wins the race — we
    // then read their post-mutation snapshot and diff against current.
    let mut guard = match cached.lock() {
        Ok(g) => g,
        Err(err) => {
            tracing::warn!(?err, "watcher cache mutex poisoned");
            return BatchOutcome::Skipped;
        }
    };
    let after = match Snapshot::capture(repo_root) {
        Ok(s) => s,
        Err(err) => {
            tracing::warn!(?err, "watcher snapshot capture failed");
            return BatchOutcome::Unchanged;
        }
    };
    let flags = guard.diff(&after);
    if flags.is_empty() {
        return BatchOutcome::Unchanged;
    }
    *guard = after;
    BatchOutcome::Mutated(flags)
}

/// A live filesystem watcher that emits `project-mutated` Tauri events.
///
/// The watcher debounces raw filesystem events with a 500 ms quiet window.
/// Most `.git/` events are filtered out, but `.git/refs/` and `.git/HEAD`
/// changes are allowed through so that external commits and branch switches
/// trigger a refresh. Working-tree paths that git ignores (`target/`,
/// `node_modules/`, build output …) are dropped from each batch *before* the
/// capture, so `cargo build` / `npm install` churn under ignored trees costs
/// nothing. For every debounced batch of surviving events we
/// capture a fresh [`Snapshot`] and diff it against the cached one — if the
/// resulting [`mutation_events::MutationFlags`] is non-empty, we emit a
/// `project-mutated` event with [`MutationKind::External`] and cache the
/// new snapshot. Drop the `RepoWatcher` to stop watching.
///
/// The cached snapshot is reachable via [`RepoWatcher::cached_snapshot`].
/// Callers that perform a known internal mutation (e.g. the AI background
/// coordinator creating a worktree) can hold the lock across the mutation
/// and overwrite the snapshot before releasing, so the next debounce sees
/// an empty diff and stays quiet.
pub struct RepoWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
    cached: Arc<Mutex<Snapshot>>,
}

impl RepoWatcher {
    /// Start watching `repo_path` recursively and emit a `project-mutated`
    /// Tauri event with [`MutationKind::External`] after each debounced
    /// batch of relevant events that actually changed repo state.
    ///
    /// Working-tree changes always qualify as "relevant". Inside `.git/`,
    /// only `refs/` and `HEAD` changes qualify (external commits, branch
    /// switches). If [`Snapshot::capture`] of the seed state fails (e.g.
    /// the repo is being initialised), we fall back to a default snapshot
    /// so the first real change still produces a diff.
    ///
    /// Returns an error if the underlying OS watcher cannot be initialised
    /// or if `repo_path` cannot be watched.
    pub fn start(app: AppHandle, repo_path: PathBuf) -> Result<Self, notify::Error> {
        let cached = Arc::new(Mutex::new(
            Snapshot::capture(&repo_path).ok().unwrap_or_default(),
        ));
        let cached_for_thread = Arc::clone(&cached);
        let cb_path = repo_path.clone();
        let (tx, rx) = mpsc::channel();

        let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;
        debouncer
            .watcher()
            .watch(&repo_path, notify::RecursiveMode::Recursive)?;

        std::thread::spawn(move || {
            while let Ok(result) = rx.recv() {
                let Ok(events) = result else { continue };
                let paths: Vec<PathBuf> = events.iter().map(|e| e.path.clone()).collect();
                // `evaluate_batch` drops git-ignored paths, then (only if any
                // survive) captures + diffs against the cache — so an
                // all-ignored build/install burst never pays the snapshot walk.
                if let BatchOutcome::Mutated(flags) =
                    evaluate_batch(&paths, &cb_path, &cached_for_thread)
                    && let Err(err) = emit_mutation(&app, MutationKind::External, flags, &cb_path)
                {
                    tracing::warn!(?err, "watcher emit failed");
                }
            }
        });

        Ok(Self {
            _debouncer: debouncer,
            cached,
        })
    }

    /// Handle to the cached snapshot used by the debounce thread for
    /// diffing.
    ///
    /// Callers that own a mutation the watcher would otherwise observe
    /// (e.g. creating a worktree the user explicitly requested via the AI
    /// background flow) should:
    ///
    /// 1. Lock this mutex *before* performing the mutation.
    /// 2. Run the mutation.
    /// 3. Overwrite the locked snapshot with a fresh
    ///    [`Snapshot::capture`].
    /// 4. Drop the guard.
    ///
    /// The next debounced batch will diff against the stored
    /// post-mutation snapshot, see no relevant flags, and skip the emit —
    /// suppressing exactly the spurious refresh the mutation would have
    /// triggered while leaving any *concurrent* external mutation
    /// detectable on the following batch.
    pub fn cached_snapshot(&self) -> Arc<Mutex<Snapshot>> {
        Arc::clone(&self.cached)
    }
}

#[cfg(test)]
mod emit_tests {
    use mutation_events::Snapshot;

    #[test]
    fn external_change_derives_flags_via_diff() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "T").unwrap();
        cfg.set_str("user.email", "t@t").unwrap();
        std::fs::write(tmp.path().join("a.txt"), "a").unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(std::path::Path::new("a.txt")).unwrap();
        idx.write().unwrap();
        let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "seed", &tree, &[])
            .unwrap();

        let before = Snapshot::capture(tmp.path()).unwrap();
        // Simulate external commit.
        std::fs::write(tmp.path().join("b.txt"), "b").unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        idx.write().unwrap();
        let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "ext", &tree, &[&parent])
            .unwrap();

        let after = Snapshot::capture(tmp.path()).unwrap();
        let flags = before.diff(&after);
        assert!(flags.head_changed && flags.refs_changed);
    }
}

#[cfg(test)]
mod filter_tests {
    use super::path_is_relevant;
    use std::path::Path;

    #[test]
    fn working_tree_changes_are_relevant() {
        assert!(path_is_relevant(Path::new("/repo/src/main.rs")));
        assert!(path_is_relevant(Path::new("/repo/README.md")));
    }

    #[test]
    fn git_internal_allowlist() {
        // Allowed git-internal entries (incl. the newly-added ones).
        assert!(path_is_relevant(Path::new("/repo/.git/HEAD")));
        assert!(path_is_relevant(Path::new("/repo/.git/refs/heads/main")));
        assert!(path_is_relevant(Path::new("/repo/.git/packed-refs")));
        assert!(path_is_relevant(Path::new("/repo/.git/index")));
        assert!(path_is_relevant(Path::new("/repo/.git/FETCH_HEAD")));
        assert!(path_is_relevant(Path::new("/repo/.git/MERGE_HEAD")));
        assert!(path_is_relevant(Path::new("/repo/.git/ORIG_HEAD")));
        // Repo-local ignore rules — a change alters what's ignored.
        assert!(path_is_relevant(Path::new("/repo/.git/info/exclude")));
    }

    #[test]
    fn git_internal_noise_is_filtered() {
        assert!(!path_is_relevant(Path::new("/repo/.git/objects/ab/cdef")));
        assert!(!path_is_relevant(Path::new("/repo/.git/logs/HEAD")));
        assert!(!path_is_relevant(Path::new("/repo/.git/COMMIT_EDITMSG")));
        // Other `.git/info/` files are not ignore rules → filtered.
        assert!(!path_is_relevant(Path::new("/repo/.git/info/attributes")));
    }
}

#[cfg(test)]
mod ignore_tests {
    use super::{BatchOutcome, Snapshot, classify_batch, evaluate_batch};
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    /// Init a bare-committed repo and return its `TempDir` (kept alive) + root.
    fn init_repo() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "T").unwrap();
        cfg.set_str("user.email", "t@t").unwrap();
        let root = tmp.path().to_path_buf();
        (tmp, root)
    }

    /// Init a repo with one committed tracked file (`tracked.txt`).
    fn init_repo_with_commit() -> (tempfile::TempDir, PathBuf) {
        let (tmp, root) = init_repo();
        let repo = git2::Repository::open(&root).unwrap();
        std::fs::write(root.join("tracked.txt"), "v1\n").unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(Path::new("tracked.txt")).unwrap();
        idx.write().unwrap();
        let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "seed", &tree, &[])
            .unwrap();
        (tmp, root)
    }

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, "x").unwrap();
    }

    #[test]
    fn ignored_paths_are_dropped() {
        let (_tmp, root) = init_repo();
        std::fs::write(root.join(".gitignore"), "target/\n").unwrap();
        let p = root.join("target/debug/foo.o");
        touch(&p);
        let class = classify_batch(&[p], &root);
        assert!(!class.relevant, "a target/ artifact must be filtered out");
        assert_eq!(class.kept, 0);
        assert_eq!(class.total, 1);
    }

    #[test]
    fn tracked_working_tree_path_survives() {
        let (_tmp, root) = init_repo();
        let class = classify_batch(&[root.join("src/main.rs")], &root);
        assert!(class.relevant);
        assert_eq!(class.kept, 1);
    }

    #[test]
    fn mixed_batch_keeps_the_non_ignored_path() {
        let (_tmp, root) = init_repo();
        std::fs::write(root.join(".gitignore"), "target/\n").unwrap();
        let ignored = root.join("target/foo.o");
        touch(&ignored);
        let real = root.join("src/lib.rs");
        let class = classify_batch(&[ignored, real], &root);
        assert!(class.relevant);
        assert_eq!(class.kept, 1);
        assert_eq!(class.total, 2);
    }

    #[test]
    fn gitignore_edit_is_relevant() {
        let (_tmp, root) = init_repo();
        std::fs::write(root.join(".gitignore"), "target/\n").unwrap();
        // The `.gitignore` itself is not ignored → survives as relevant, so an
        // edit to the ignore rules still refreshes the UI.
        let class = classify_batch(&[root.join(".gitignore")], &root);
        assert!(class.relevant);
    }

    #[test]
    fn git_info_exclude_is_relevant() {
        let (_tmp, root) = init_repo();
        let class = classify_batch(&[root.join(".git/info/exclude")], &root);
        assert!(class.relevant, "info/exclude changes ignore rules");
    }

    #[test]
    fn git_object_noise_is_dropped() {
        let (_tmp, root) = init_repo();
        let class = classify_batch(&[root.join(".git/objects/ab/cdef")], &root);
        assert!(!class.relevant);
    }

    #[test]
    fn negation_reincludes_a_file_in_an_ignored_dir() {
        let (_tmp, root) = init_repo();
        // `build/*` ignores the dir's contents, `!build/keep.me` re-includes
        // one file. (git can't re-include under a wholesale-excluded `build/`,
        // so the contents form is the canonical negation pattern.)
        std::fs::write(root.join(".gitignore"), "build/*\n!build/keep.me\n").unwrap();
        let keep = root.join("build/keep.me");
        let drop = root.join("build/other.log");
        touch(&keep);
        touch(&drop);

        let kept = classify_batch(&[keep], &root);
        assert!(kept.relevant, "!build/keep.me must survive filtering");

        let dropped = classify_batch(&[drop], &root);
        assert!(!dropped.relevant, "build/other.log stays ignored");
    }

    #[test]
    fn nested_gitignore_is_respected() {
        // The whole point of using git2 over a single flattened matcher: a
        // nested `.gitignore` applies only under its own directory.
        let (_tmp, root) = init_repo();
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("sub/.gitignore"), "secret.txt\n").unwrap();
        let secret = root.join("sub/secret.txt");
        let visible = root.join("sub/visible.txt");
        touch(&secret);
        touch(&visible);

        assert!(
            !classify_batch(&[secret], &root).relevant,
            "nested .gitignore must ignore sub/secret.txt"
        );
        assert!(
            classify_batch(&[visible], &root).relevant,
            "sub/visible.txt is not matched by any rule"
        );
    }

    #[test]
    fn open_failure_falls_back_to_relevant() {
        // A non-repo path can't be opened → keep working-tree paths relevant
        // rather than silently dropping a change we can't classify.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let class = classify_batch(&[root.join("some/file.rs")], &root);
        assert!(class.relevant);
    }

    #[test]
    fn ignored_burst_skips_capture() {
        // The headline scenario: a `cargo build`-style burst of ignored files
        // must NOT trigger a Snapshot::capture. `BatchOutcome::Skipped` is
        // returned only on the pre-capture skip path, so this asserts zero
        // captures happened.
        let (_tmp, root) = init_repo_with_commit();
        std::fs::write(root.join(".gitignore"), "target/\n").unwrap();
        let cached = Mutex::new(Snapshot::capture(&root).unwrap());

        let mut paths = Vec::new();
        for i in 0..64 {
            let p = root.join(format!("target/debug/artifact-{i}.o"));
            touch(&p);
            paths.push(p);
        }

        let outcome = evaluate_batch(&paths, &root, &cached);
        assert!(
            matches!(outcome, BatchOutcome::Skipped),
            "an all-ignored batch must perform zero captures"
        );
    }

    #[test]
    fn tracked_file_edit_triggers_capture() {
        let (_tmp, root) = init_repo_with_commit();
        let cached = Mutex::new(Snapshot::capture(&root).unwrap());
        // Modify the committed tracked file mid-"build".
        std::fs::write(root.join("tracked.txt"), "v2\n").unwrap();

        let outcome = evaluate_batch(&[root.join("tracked.txt")], &root, &cached);
        match outcome {
            BatchOutcome::Mutated(flags) => assert!(flags.status_changed),
            other => panic!("expected Mutated, got {other:?}"),
        }
    }

    #[test]
    fn edit_alongside_ignored_burst_still_triggers() {
        // A tracked edit buried in a flood of ignored build output must still
        // be seen — the ignore filter drops the noise, not the real change.
        let (_tmp, root) = init_repo_with_commit();
        std::fs::write(root.join(".gitignore"), "target/\n").unwrap();
        let cached = Mutex::new(Snapshot::capture(&root).unwrap());
        std::fs::write(root.join("tracked.txt"), "v2\n").unwrap();

        let mut paths = vec![root.join("tracked.txt")];
        for i in 0..32 {
            let p = root.join(format!("target/debug/x-{i}.o"));
            touch(&p);
            paths.push(p);
        }

        assert!(matches!(
            evaluate_batch(&paths, &root, &cached),
            BatchOutcome::Mutated(_)
        ));
    }
}
