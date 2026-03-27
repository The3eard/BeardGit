//! Debounced filesystem watcher for git repository working trees.
//!
//! [`RepoWatcher`] wraps `notify` and `notify-debouncer-mini` to watch a
//! repository directory for changes and call a user-provided callback after a
//! brief quiet period. Events inside the `.git` directory are intentionally
//! ignored so that background git operations (e.g. index writes) do not
//! trigger spurious refreshes of the UI.

use notify_debouncer_mini::new_debouncer;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

/// A live filesystem watcher that fires a callback whenever the working tree changes.
///
/// The watcher debounces raw filesystem events with a 500 ms quiet window and
/// filters out events originating from the `.git` directory. Drop the
/// `RepoWatcher` to stop watching.
pub struct RepoWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

impl RepoWatcher {
    /// Start watching `repo_path` recursively and call `on_change` after each
    /// debounced batch of non-`.git` events.
    ///
    /// Returns an error if the underlying OS watcher cannot be initialised or
    /// if `repo_path` cannot be watched.
    pub fn start<F>(repo_path: &Path, on_change: F) -> Result<Self, notify::Error>
    where
        F: Fn() + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();

        let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;
        debouncer
            .watcher()
            .watch(repo_path, notify::RecursiveMode::Recursive)?;

        std::thread::spawn(move || {
            while let Ok(result) = rx.recv() {
                if let Ok(events) = result {
                    let relevant = events
                        .iter()
                        .any(|e| !e.path.components().any(|c| c.as_os_str() == ".git"));
                    if relevant {
                        on_change();
                    }
                }
            }
        });

        Ok(Self {
            _debouncer: debouncer,
        })
    }
}
