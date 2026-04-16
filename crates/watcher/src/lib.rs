//! Debounced filesystem watchers for BeardGit.
//!
//! [`RepoWatcher`] watches a git repository working tree for changes and calls
//! a user-provided callback after a brief quiet period. Most events inside
//! `.git/` are filtered out to avoid spurious refreshes, but changes to
//! `.git/refs/` and `.git/HEAD` are allowed through so that external commits
//! and branch switches are detected.
//!
//! [`AiSessionWatcher`] watches AI provider session directories (e.g.
//! `~/.claude/sessions/`) and fires on any change, without filtering.

mod ai_sessions;
pub use ai_sessions::AiSessionWatcher;

use notify_debouncer_mini::new_debouncer;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

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

/// A live filesystem watcher that fires a callback whenever the working tree changes.
///
/// The watcher debounces raw filesystem events with a 500 ms quiet window.
/// Most `.git/` events are filtered out, but `.git/refs/` and `.git/HEAD`
/// changes are allowed through so that external commits and branch switches
/// trigger a UI refresh. Drop the `RepoWatcher` to stop watching.
pub struct RepoWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

impl RepoWatcher {
    /// Start watching `repo_path` recursively and call `on_change` after each
    /// debounced batch of relevant events.
    ///
    /// Working-tree changes always fire. Inside `.git/`, only `refs/` and
    /// `HEAD` changes fire (external commits, branch switches).
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
                    let relevant = events.iter().any(is_relevant_event);
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
