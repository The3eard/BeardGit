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
///
/// Both git commands are run with `current_dir` set to the OWNING repo's main
/// worktree. They are repo-relative — especially `git branch -D` — so without
/// a cwd they would inherit the application process's cwd and either fail
/// ("not a git repository") or, worse, delete a same-named branch in whatever
/// unrelated repo happened to be the cwd (data loss).
pub fn cleanup_worktree(worktree: &AiWorktree) -> Result<(), AiError> {
    let path_str = worktree.path.to_string_lossy();
    let main_repo = main_worktree_dir(&worktree.path)?;

    let output = Command::new("git")
        .current_dir(&main_repo)
        .args(["worktree", "remove", &path_str, "--force"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AiError::CommandBuild(format!(
            "git worktree remove failed: {stderr}"
        )));
    }

    let output = Command::new("git")
        .current_dir(&main_repo)
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

/// Resolve the main working directory of the repository that owns
/// `worktree_path`. `git worktree list --porcelain` always lists the main
/// worktree first, so its path is the repo root we must run cleanup from.
fn main_worktree_dir(worktree_path: &Path) -> Result<PathBuf, AiError> {
    let output = Command::new("git")
        .current_dir(worktree_path)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .map_err(|e| AiError::CommandBuild(format!("git worktree list failed: {e}")))?;

    if !output.status.success() {
        return Err(AiError::CommandBuild(format!(
            "could not resolve owning repo for worktree {}: {}",
            worktree_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_porcelain_output(&stdout)
        .into_iter()
        .next()
        .map(|e| e.path)
        .ok_or_else(|| AiError::CommandBuild("no main worktree found".into()))
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

    /// Regression: cleanup must run in the OWNING repo (resolved from the
    /// worktree), not the process cwd — so the branch is deleted in the right
    /// place. The test process cwd is not `main`, so this would mis-delete (or
    /// fail) before the `current_dir` fix.
    #[test]
    fn cleanup_worktree_removes_worktree_and_branch_in_owning_repo() {
        let git = |args: &[&str], cwd: &Path| {
            Command::new("git")
                .current_dir(cwd)
                .args(args)
                .output()
                .unwrap()
        };
        let tmp = tempfile::tempdir().unwrap();
        let main = tmp.path().join("main");
        std::fs::create_dir(&main).unwrap();
        git(&["init", "-q"], &main);
        git(&["config", "user.email", "t@t.com"], &main);
        git(&["config", "user.name", "t"], &main);
        std::fs::write(main.join("f.txt"), "x").unwrap();
        git(&["add", "."], &main);
        git(&["commit", "-qm", "init"], &main);

        let wt = tmp.path().join("wt");
        git(
            &[
                "worktree",
                "add",
                "-b",
                "worktree-test",
                wt.to_str().unwrap(),
            ],
            &main,
        );
        assert!(wt.is_dir(), "worktree should be created");

        let aiwt = AiWorktree {
            path: wt.clone(),
            branch: "worktree-test".into(),
            provider: AiProviderKind::ClaudeCode,
            session_id: None,
            status: WorktreeStatus::Clean,
        };
        cleanup_worktree(&aiwt).expect("cleanup should succeed");

        assert!(!wt.exists(), "worktree dir should be removed");
        let branches = git(&["branch", "--list", "worktree-test"], &main);
        assert!(
            String::from_utf8_lossy(&branches.stdout).trim().is_empty(),
            "branch must be deleted in the owning repo"
        );
    }
}
