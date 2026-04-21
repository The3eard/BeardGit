//! OpenCode worktree discovery and cleanup.
//!
//! BeardGit-spawned OpenCode worktrees live under the shared convention:
//!
//! ```text
//! <repo>/.beardgit/ai-worktrees/opencode/<slug>/
//!   .beardgit/ai-session          ← optional marker with session id
//! ```
//!
//! Each subdirectory of the `opencode/` directory corresponds to one
//! OpenCode worktree. The optional `.beardgit/ai-session` file holds the
//! session id (written by the orchestrator when launching a background
//! run). The layout mirrors [`codex::worktrees`] so the UI can treat all
//! provider worktrees uniformly.

use std::fs;
use std::path::Path;

use ai_provider::{AiError, AiProviderKind, AiWorktree, WorktreeStatus};

/// Relative path (from the repo root) where OpenCode worktrees live.
pub const OPENCODE_WORKTREE_DIR: &str = ".beardgit/ai-worktrees/opencode";

/// Optional marker file inside each worktree that holds the session id.
const SESSION_MARKER_REL: &str = ".beardgit/ai-session";

/// List all OpenCode worktrees spawned by BeardGit for `repo_path`.
///
/// Returns `Ok(Vec::new())` when the parent directory doesn't exist —
/// a brand-new repo with zero BeardGit activity is the common case.
pub fn list_worktrees(repo_path: &Path) -> Result<Vec<AiWorktree>, AiError> {
    let root = repo_path.join(OPENCODE_WORKTREE_DIR);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut worktrees = Vec::new();
    let entries = fs::read_dir(&root).map_err(AiError::Io)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let branch = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let session_id = read_session_marker(&path);
        let status = determine_status(&path);
        worktrees.push(AiWorktree {
            path,
            branch,
            provider: AiProviderKind::OpenCode,
            session_id,
            status,
        });
    }
    Ok(worktrees)
}

/// Remove the given worktree directory recursively.
///
/// OpenCode worktrees are plain directories (not necessarily linked git
/// worktrees), so a simple `remove_dir_all` is sufficient. Cleanup is
/// idempotent: a non-existent path is a no-op.
pub fn cleanup_worktree(worktree: &AiWorktree) -> Result<(), AiError> {
    if worktree.path.exists() {
        fs::remove_dir_all(&worktree.path).map_err(AiError::Io)?;
    }
    Ok(())
}

/// Read the optional `.beardgit/ai-session` marker file if present.
fn read_session_marker(worktree_path: &Path) -> Option<String> {
    let marker = worktree_path.join(SESSION_MARKER_REL);
    fs::read_to_string(marker).ok().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// Classify a worktree as Active / Clean / Orphaned.
///
/// - **Active**: session marker present (we don't probe liveness here —
///   callers can cross-reference with [`crate::OpenCodeProvider::is_session_active`]
///   if needed).
/// - **Clean**: directory exists but no session marker.
/// - **Orphaned**: path doesn't exist.
fn determine_status(path: &Path) -> WorktreeStatus {
    if !path.is_dir() {
        return WorktreeStatus::Orphaned;
    }
    let marker = path.join(SESSION_MARKER_REL);
    if marker.is_file() {
        WorktreeStatus::Active
    } else {
        WorktreeStatus::Clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn mkwt(repo: &Path, slug: &str) -> PathBuf {
        let path = repo.join(OPENCODE_WORKTREE_DIR).join(slug);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn empty_repo_returns_no_worktrees() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_worktrees(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn lists_every_subdir_as_worktree() {
        let dir = tempfile::tempdir().unwrap();
        mkwt(dir.path(), "feat-a");
        mkwt(dir.path(), "feat-b");
        let wts = list_worktrees(dir.path()).unwrap();
        assert_eq!(wts.len(), 2);
        let branches: Vec<_> = wts.iter().map(|w| w.branch.clone()).collect();
        assert!(branches.contains(&"feat-a".to_string()));
        assert!(branches.contains(&"feat-b".to_string()));
        assert!(wts.iter().all(|w| w.provider == AiProviderKind::OpenCode));
    }

    #[test]
    fn reads_optional_session_marker() {
        let dir = tempfile::tempdir().unwrap();
        let wt = mkwt(dir.path(), "feat-a");
        let marker_dir = wt.join(".beardgit");
        fs::create_dir_all(&marker_dir).unwrap();
        fs::write(marker_dir.join("ai-session"), "ses_abc\n").unwrap();

        let wts = list_worktrees(dir.path()).unwrap();
        assert_eq!(wts.len(), 1);
        assert_eq!(wts[0].session_id.as_deref(), Some("ses_abc"));
        assert_eq!(wts[0].status, WorktreeStatus::Active);
    }

    #[test]
    fn worktree_without_marker_is_clean() {
        let dir = tempfile::tempdir().unwrap();
        mkwt(dir.path(), "feat-a");
        let wts = list_worktrees(dir.path()).unwrap();
        assert_eq!(wts.len(), 1);
        assert!(wts[0].session_id.is_none());
        assert_eq!(wts[0].status, WorktreeStatus::Clean);
    }

    #[test]
    fn ignores_files_in_worktree_root() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(OPENCODE_WORKTREE_DIR)).unwrap();
        fs::write(
            dir.path().join(OPENCODE_WORKTREE_DIR).join("notes.txt"),
            "hi",
        )
        .unwrap();
        assert!(list_worktrees(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn cleanup_worktree_removes_directory() {
        let dir = tempfile::tempdir().unwrap();
        let wt = mkwt(dir.path(), "feat-clean");
        fs::write(wt.join("file.txt"), "data").unwrap();
        let worktree = AiWorktree {
            path: wt.clone(),
            branch: "feat-clean".into(),
            provider: AiProviderKind::OpenCode,
            session_id: None,
            status: WorktreeStatus::Clean,
        };
        cleanup_worktree(&worktree).unwrap();
        assert!(!wt.exists());
    }

    #[test]
    fn cleanup_worktree_missing_path_is_ok() {
        let worktree = AiWorktree {
            path: PathBuf::from("/nonexistent/opencode/worktree"),
            branch: "ghost".into(),
            provider: AiProviderKind::OpenCode,
            session_id: None,
            status: WorktreeStatus::Orphaned,
        };
        cleanup_worktree(&worktree).unwrap();
    }
}
