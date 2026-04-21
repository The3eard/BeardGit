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
//! [`AiSessionWatcher`] watches AI provider session directories (e.g.
//! `~/.claude/sessions/`) and fires on any change, without filtering.

mod ai_config;
mod ai_sessions;
pub use ai_config::{AiConfigChange, AiConfigWatcher, ConfigChangeScope};
pub use ai_sessions::AiSessionWatcher;

use mutation_events::{emit_mutation, MutationKind, Snapshot};
use notify_debouncer_mini::new_debouncer;
use std::path::PathBuf;
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use tauri::AppHandle;

/// Check whether a filesystem event is relevant for UI refresh.
///
/// Working-tree changes (outside `.git/`) are always relevant. Inside `.git/`,
/// only `refs/` subtree and the `HEAD` file are relevant — these change on
/// commits, branch creation/deletion, and checkouts.
fn is_relevant_event(event: &notify_debouncer_mini::DebouncedEvent) -> bool {
    let path = &event.path;
    let components: Vec<_> = path.components().collect();

    // Find the .git component index
    let git_idx = components.iter().position(|c| c.as_os_str() == ".git");

    let Some(idx) = git_idx else {
        // Not inside .git/ — always relevant (working tree change)
        return true;
    };

    // Inside .git/ — only allow refs/ and HEAD
    let after_git: Vec<_> = components[idx + 1..].iter().collect();
    match after_git.first() {
        Some(c) if c.as_os_str() == "refs" => true,
        Some(c) if c.as_os_str() == "HEAD" => true,
        _ => false,
    }
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
pub struct RepoWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
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
        let cached = Mutex::new(Snapshot::capture(&repo_path).ok().unwrap_or_default());
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
                let after = match Snapshot::capture(&cb_path) {
                    Ok(s) => s,
                    Err(err) => {
                        tracing::warn!(?err, "watcher snapshot capture failed");
                        continue;
                    }
                };
                let mut guard = match cached.lock() {
                    Ok(g) => g,
                    Err(err) => {
                        tracing::warn!(?err, "watcher cache mutex poisoned");
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
        })
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
