//! Interactive rebase support.
//!
//! Provides [`Repository::get_rebase_commits`] to list the commits eligible for
//! rebasing and [`Repository::start_interactive_rebase`] to execute a
//! pre-planned interactive rebase using `GIT_SEQUENCE_EDITOR`.

use std::io::Write;

use serde::{Deserialize, Serialize};

use crate::error::GitError;
use crate::repository::Repository;

/// A commit in the rebase todo list.
#[derive(Debug, Clone, Serialize)]
pub struct RebaseCommit {
    /// Full SHA of the commit.
    pub oid: String,
    /// First line of the commit message.
    pub message: String,
    /// Author name.
    pub author: String,
    /// ISO-8601 author date.
    pub date: String,
}

/// An action for a commit in the interactive rebase.
#[derive(Debug, Clone, Deserialize)]
pub struct RebaseAction {
    /// Full or abbreviated SHA of the target commit.
    pub oid: String,
    /// Rebase verb: `"pick"`, `"squash"`, `"fixup"`, `"edit"`, or `"drop"`.
    pub action: String,
}

impl Repository {
    /// Get the commits between `base_oid` (exclusive) and HEAD (inclusive).
    ///
    /// Returns commits in rebase order (oldest first) — the same order git
    /// uses for the interactive rebase todo file.
    pub fn get_rebase_commits(&self, base_oid: &str) -> Result<Vec<RebaseCommit>, GitError> {
        let result = self.git_cmd(&[
            "log",
            "--reverse",
            "--format=%H|%s|%an|%ai",
            &format!("{base_oid}..HEAD"),
        ])?;

        if !result.success {
            return Err(GitError::RepoNotFound(result.stderr));
        }

        Ok(result
            .stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(4, '|').collect();
                if parts.len() == 4 {
                    Some(RebaseCommit {
                        oid: parts[0].to_string(),
                        message: parts[1].to_string(),
                        author: parts[2].to_string(),
                        date: parts[3].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    /// Start an interactive rebase with pre-defined actions.
    ///
    /// Generates a todo file from `actions` and uses `GIT_SEQUENCE_EDITOR` to
    /// inject it into `git rebase -i`. The sequence editor is a simple copy
    /// command that overwrites git's generated todo file with our pre-built one.
    pub fn start_interactive_rebase(
        &self,
        base_oid: &str,
        actions: &[RebaseAction],
    ) -> Result<(), GitError> {
        // Build the todo list content.
        let mut todo = String::new();
        for action in actions {
            let short_oid = if action.oid.len() > 7 {
                &action.oid[..7]
            } else {
                &action.oid
            };
            todo.push_str(&format!("{} {}\n", action.action, short_oid));
        }

        // Write todo to a temp file.
        let mut todo_file = tempfile::NamedTempFile::new().map_err(GitError::Io)?;
        todo_file.write_all(todo.as_bytes()).map_err(GitError::Io)?;
        todo_file.flush().map_err(GitError::Io)?;
        let todo_path = todo_file.path().to_string_lossy().to_string();

        // Create a command that copies our todo file over git's todo file.
        // Git invokes: $GIT_SEQUENCE_EDITOR <rebase-todo-path>
        let editor_cmd = if cfg!(target_os = "windows") {
            format!("copy /Y \"{}\" ", todo_path.replace('/', "\\"))
        } else {
            format!("cp '{}' ", todo_path)
        };

        let result = self.git_cmd_with_env(
            &["rebase", "-i", base_oid],
            &[("GIT_SEQUENCE_EDITOR", &editor_cmd)],
        )?;

        if result.success {
            Ok(())
        } else if result.stderr.contains("CONFLICT") || result.stderr.contains("could not apply") {
            // Conflict is not a fatal error — the ConflictToolbar will handle it.
            Ok(())
        } else {
            Err(GitError::RepoNotFound(result.stderr))
        }
    }
}

/// Build a rebase todo string from a slice of actions.
#[cfg(test)]
fn build_todo(actions: &[RebaseAction]) -> String {
    let mut todo = String::new();
    for action in actions {
        let short_oid = if action.oid.len() > 7 {
            &action.oid[..7]
        } else {
            &action.oid
        };
        todo.push_str(&format!("{} {}\n", action.action, short_oid));
    }
    todo
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use std::process::Command;

    /// Helper: create a git repo in a temp dir with an initial commit.
    fn init_repo(dir: &std::path::Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    /// Helper: create a file and commit it, returning the commit OID.
    fn commit_file(dir: &std::path::Path, name: &str, content: &str) -> String {
        std::fs::write(dir.join(name), content).unwrap();
        Command::new("git")
            .args(["add", name])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", &format!("Add {name}")])
            .current_dir(dir)
            .output()
            .unwrap();
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(dir)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn test_get_rebase_commits() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        init_repo(dir);

        let base = commit_file(dir, "a.txt", "a");
        let _c2 = commit_file(dir, "b.txt", "b");
        let _c3 = commit_file(dir, "c.txt", "c");

        let repo = Repository::open(dir).unwrap();
        let commits = repo.get_rebase_commits(&base).unwrap();

        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "Add b.txt");
        assert_eq!(commits[1].message, "Add c.txt");
    }

    #[test]
    fn test_get_rebase_commits_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        init_repo(dir);

        let head = commit_file(dir, "a.txt", "a");

        let repo = Repository::open(dir).unwrap();
        let commits = repo.get_rebase_commits(&head).unwrap();

        assert!(commits.is_empty());
    }

    #[test]
    fn test_rebase_action_deserialization() {
        let json = r#"{"oid":"abc1234","action":"squash"}"#;
        let action: RebaseAction = serde_json::from_str(json).unwrap();
        assert_eq!(action.oid, "abc1234");
        assert_eq!(action.action, "squash");
    }

    #[test]
    fn test_build_todo() {
        let actions = vec![
            RebaseAction {
                oid: "abc1234567890".to_string(),
                action: "pick".to_string(),
            },
            RebaseAction {
                oid: "def5678901234".to_string(),
                action: "squash".to_string(),
            },
            RebaseAction {
                oid: "short".to_string(),
                action: "drop".to_string(),
            },
        ];

        let todo = build_todo(&actions);
        assert_eq!(todo, "pick abc1234\nsquash def5678\ndrop short\n");
    }
}
