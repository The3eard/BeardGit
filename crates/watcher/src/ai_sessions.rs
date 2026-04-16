//! Watcher for AI session directories (`~/.claude/sessions/`, etc.).
//!
//! Fires a callback whenever session files are created, modified, or deleted.
//! Unlike [`RepoWatcher`], this watches a global directory and does not filter
//! events — every change inside the watched directories triggers the callback.

use notify_debouncer_mini::new_debouncer;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

/// A live filesystem watcher for AI session directories.
///
/// Watches `~/.claude/sessions/` (and other provider session dirs) for changes.
/// Fires the callback on any file event. Drop to stop watching.
pub struct AiSessionWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

impl AiSessionWatcher {
    /// Start watching the given session directories.
    ///
    /// Only directories that actually exist on disk are watched. Returns `None`
    /// if no valid directories were found or if the OS watcher could not be
    /// initialised.
    pub fn start<F>(session_dirs: &[PathBuf], on_change: F) -> Option<Self>
    where
        F: Fn() + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let mut debouncer = new_debouncer(Duration::from_millis(500), tx).ok()?;
        let mut watching_any = false;

        for dir in session_dirs {
            if dir.is_dir()
                && debouncer
                    .watcher()
                    .watch(dir, notify::RecursiveMode::NonRecursive)
                    .is_ok()
            {
                watching_any = true;
            }
        }

        if !watching_any {
            return None;
        }

        std::thread::spawn(move || {
            while let Ok(result) = rx.recv() {
                if result.is_ok() {
                    on_change();
                }
            }
        });

        Some(Self {
            _debouncer: debouncer,
        })
    }
}
