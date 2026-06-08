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

use mutation_events::{MutationKind, Snapshot, emit_mutation};
use notify_debouncer_mini::new_debouncer;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tauri::AppHandle;

/// Check whether a filesystem event is relevant for UI refresh.
///
/// Working-tree changes (outside `.git/`) are always relevant. Inside `.git/`,
/// only `refs/` subtree and the `HEAD` file are relevant — these change on
/// commits, branch creation/deletion, and checkouts.
fn is_relevant_event(event: &notify_debouncer_mini::DebouncedEvent) -> bool {
    path_is_relevant(&event.path)
}

/// Pure path classifier behind [`is_relevant_event`] — extracted so the
/// allowlist can be unit-tested without constructing a `DebouncedEvent`.
fn path_is_relevant(path: &std::path::Path) -> bool {
    let components: Vec<_> = path.components().collect();

    // Find the .git component index
    let git_idx = components.iter().position(|c| c.as_os_str() == ".git");

    let Some(idx) = git_idx else {
        // Not inside .git/ — always relevant (working tree change)
        return true;
    };

    // Inside .git/ — allow the entries that change on real repo mutations:
    //  - refs/**, HEAD: commits, branch create/delete, checkout
    //  - packed-refs: refs packed by gc / fetch / branch -d of a packed ref
    //  - index: external stage/unstage/reset (writes .git/index only — the
    //    working tree is untouched, so nothing else in the batch is relevant)
    //  - FETCH_HEAD / MERGE_HEAD / ORIG_HEAD: fetch / merge / rebase flows
    let after_git: Vec<_> = components[idx + 1..].iter().collect();
    matches!(
        after_git.first().and_then(|c| c.as_os_str().to_str()),
        Some("refs" | "HEAD" | "packed-refs" | "index" | "FETCH_HEAD" | "MERGE_HEAD" | "ORIG_HEAD")
    )
}

/// A live filesystem watcher that emits `project-mutated` Tauri events.
///
/// The watcher debounces raw filesystem events with a 500 ms quiet window.
/// Most `.git/` events are filtered out, but `.git/refs/` and `.git/HEAD`
/// changes are allowed through so that external commits and branch switches
/// trigger a refresh. For every debounced batch of relevant events we
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
                if !events.iter().any(is_relevant_event) {
                    continue;
                }
                // Acquire the cache lock first so a concurrent caller that
                // is mid-internal-mutation (holding the same lock and about
                // to overwrite the cache) wins the race — we read the
                // post-mutation snapshot they store, diff against current,
                // and emit nothing.
                let mut guard = match cached_for_thread.lock() {
                    Ok(g) => g,
                    Err(err) => {
                        tracing::warn!(?err, "watcher cache mutex poisoned");
                        continue;
                    }
                };
                let after = match Snapshot::capture(&cb_path) {
                    Ok(s) => s,
                    Err(err) => {
                        tracing::warn!(?err, "watcher snapshot capture failed");
                        continue;
                    }
                };
                let flags = guard.diff(&after);
                if flags.is_empty() {
                    continue;
                }
                if let Err(err) = emit_mutation(&app, MutationKind::External, flags, &cb_path) {
                    tracing::warn!(?err, "watcher emit failed");
                }
                *guard = after;
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
    }

    #[test]
    fn git_internal_noise_is_filtered() {
        assert!(!path_is_relevant(Path::new("/repo/.git/objects/ab/cdef")));
        assert!(!path_is_relevant(Path::new("/repo/.git/logs/HEAD")));
        assert!(!path_is_relevant(Path::new("/repo/.git/COMMIT_EDITMSG")));
    }
}
