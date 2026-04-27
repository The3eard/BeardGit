//! Watcher for AI configuration directories (`.claude/` in project and home).
//!
//! Fires a callback with the changed file path and scope (project/user)
//! whenever config files are created, modified, or deleted. Debounced at 500ms.

use notify_debouncer_mini::new_debouncer;
use std::sync::mpsc;
use std::time::Duration;

/// Scope of a changed config file.
#[derive(Debug, Clone, serde::Serialize)]
pub enum ConfigChangeScope {
    /// File is under the project's `.claude/` directory.
    #[serde(rename = "project")]
    Project,
    /// File is under `~/.claude/`.
    #[serde(rename = "user")]
    User,
}

/// Payload emitted when a config file changes on disk.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AiConfigChange {
    /// Absolute path to the changed file.
    pub path: String,
    /// Whether this is a project-scoped or user-scoped file.
    pub scope: ConfigChangeScope,
}

/// A live filesystem watcher for AI config directories.
///
/// Watches `<project>/.claude/` and `~/.claude/` for changes.
/// Drop to stop watching.
pub struct AiConfigWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

impl AiConfigWatcher {
    /// Start watching the given project path's `.claude/` dir and `~/.claude/`.
    ///
    /// The `on_change` callback receives an [`AiConfigChange`] with the path
    /// and scope. Returns `None` if no valid directories exist or the OS
    /// watcher could not be initialised.
    pub fn start<F>(project_path: &std::path::Path, on_change: F) -> Option<Self>
    where
        F: Fn(AiConfigChange) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let mut debouncer = new_debouncer(Duration::from_millis(500), tx).ok()?;
        let mut watching_any = false;

        let project_claude = project_path.join(".claude");
        let user_claude = dirs::home_dir().map(|h| h.join(".claude"));

        let project_dir = if project_claude.is_dir() {
            if debouncer
                .watcher()
                .watch(&project_claude, notify::RecursiveMode::Recursive)
                .is_ok()
            {
                watching_any = true;
                Some(project_claude.clone())
            } else {
                None
            }
        } else {
            None
        };

        let _user_dir = if let Some(ref uc) = user_claude {
            if uc.is_dir()
                && debouncer
                    .watcher()
                    .watch(uc, notify::RecursiveMode::Recursive)
                    .is_ok()
            {
                watching_any = true;
                Some(uc.clone())
            } else {
                None
            }
        } else {
            None
        };

        if !watching_any {
            return None;
        }

        std::thread::spawn(move || {
            while let Ok(result) = rx.recv() {
                if let Ok(events) = result {
                    for event in events {
                        let path_str = event.path.to_string_lossy().to_string();
                        let scope = if let Some(ref pd) = project_dir {
                            if event.path.starts_with(pd) {
                                ConfigChangeScope::Project
                            } else {
                                ConfigChangeScope::User
                            }
                        } else {
                            ConfigChangeScope::User
                        };
                        on_change(AiConfigChange {
                            path: path_str,
                            scope,
                        });
                    }
                }
            }
        });

        Some(Self {
            _debouncer: debouncer,
        })
    }
}
