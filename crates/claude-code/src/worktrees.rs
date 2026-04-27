//! Claude Code worktree enumeration and cleanup.
//!
//! Cross-references `git worktree list --porcelain` with the `worktree-*`
//! branch naming convention used by Claude Code's `--worktree` flag.

use std::path::{Path, PathBuf};
use std::process::Command;

use ai_provider::{AiError, AiProviderKind, AiWorktree, WorktreeStatus};

/// A parsed entry from `git worktree list --porcelain`.
#[derive(Debug)]
struct GitWorktreeEntry {
    path: PathBuf,
    branch: String,
}

/// List AI-created worktrees by filtering `git worktree list` for `worktree-*` branches.
pub fn list_worktrees(repo_path: &Path) -> Result<Vec<AiWorktree>, AiError> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .map_err(|e| AiError::CommandBuild(format!("git worktree list failed: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries = parse_porcelain_output(&stdout);

    let ai_worktrees = entries
        .into_iter()
        .filter(|entry| entry.branch.starts_with("worktree-"))
        .map(|entry| {
            let status = determine_status(&entry);
            AiWorktree {
                path: entry.path,
                branch: entry.branch,
                provider: AiProviderKind::ClaudeCode,
                session_id: None,
                status,
            }
        })
        .collect();

    Ok(ai_worktrees)
}

/// Remove a worktree and delete its branch.
pub fn cleanup_worktree(worktree: &AiWorktree) -> Result<(), AiError> {
    let path_str = worktree.path.to_string_lossy();

    let output = Command::new("git")
        .args(["worktree", "remove", &path_str, "--force"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AiError::CommandBuild(format!(
            "git worktree remove failed: {stderr}"
        )));
    }

    let output = Command::new("git")
        .args(["branch", "-D", &worktree.branch])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AiError::CommandBuild(format!(
            "git branch -D failed: {stderr}"
        )));
    }

    Ok(())
}

/// Parse `git worktree list --porcelain` output into entries.
fn parse_porcelain_output(output: &str) -> Vec<GitWorktreeEntry> {
    let mut entries = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if let (Some(p), Some(b)) = (current_path.take(), current_branch.take()) {
                entries.push(GitWorktreeEntry { path: p, branch: b });
            }
            current_path = Some(PathBuf::from(path));
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            let branch = branch_ref.strip_prefix("refs/heads/").unwrap_or(branch_ref);
            current_branch = Some(branch.to_string());
        }
    }

    if let (Some(p), Some(b)) = (current_path, current_branch) {
        entries.push(GitWorktreeEntry { path: p, branch: b });
    }

    entries
}

/// Determine worktree status: Active, Clean, or Orphaned.
fn determine_status(entry: &GitWorktreeEntry) -> WorktreeStatus {
    if !entry.path.is_dir() {
        return WorktreeStatus::Orphaned;
    }
    let git_file = entry.path.join(".git");
    if git_file.exists() {
        WorktreeStatus::Clean
    } else {
        WorktreeStatus::Orphaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_porcelain_single_entry() {
        let output = "worktree /tmp/main\nHEAD abc123\nbranch refs/heads/main\n";
        let entries = parse_porcelain_output(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, PathBuf::from("/tmp/main"));
        assert_eq!(entries[0].branch, "main");
    }

    #[test]
    fn parse_porcelain_multiple_entries() {
        let output = "\
worktree /tmp/main
HEAD abc123
branch refs/heads/main

worktree /tmp/wt1
HEAD def456
branch refs/heads/worktree-feature

worktree /tmp/wt2
HEAD 789abc
branch refs/heads/worktree-bugfix
";
        let entries = parse_porcelain_output(output);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].branch, "worktree-feature");
        assert_eq!(entries[2].branch, "worktree-bugfix");
    }

    #[test]
    fn parse_porcelain_filters_worktree_branches() {
        let output = "\
worktree /tmp/main
HEAD abc
branch refs/heads/main

worktree /tmp/wt
HEAD def
branch refs/heads/worktree-ai-task
";
        let entries = parse_porcelain_output(output);
        let ai_entries: Vec<_> = entries
            .iter()
            .filter(|e| e.branch.starts_with("worktree-"))
            .collect();
        assert_eq!(ai_entries.len(), 1);
        assert_eq!(ai_entries[0].branch, "worktree-ai-task");
    }

    #[test]
    fn parse_porcelain_empty_output() {
        let entries = parse_porcelain_output("");
        assert!(entries.is_empty());
    }

    #[test]
    fn orphaned_status_for_missing_path() {
        let entry = GitWorktreeEntry {
            path: PathBuf::from("/nonexistent/path"),
            branch: "worktree-test".into(),
        };
        assert_eq!(determine_status(&entry), WorktreeStatus::Orphaned);
    }
}
