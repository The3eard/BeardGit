//! Interactive rebase support.
//!
//! Provides [`Repository::get_rebase_commits`] to list the commits eligible for
//! rebasing and [`Repository::start_interactive_rebase`] to execute a
//! pre-planned interactive rebase using `GIT_SEQUENCE_EDITOR`.

use std::io::Write;

use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::GitError;
use crate::repository::Repository;

/// Most commits [`Repository::get_rebase_commits`] will list.
///
/// A refusal, not a truncation. `git rebase -i` applies exactly the todo file
/// it is given and **silently drops every commit missing from it** — verified:
/// feeding a 2-line todo to a 5-commit range leaves a 2-commit branch. So a
/// capped list would not be a rendering shortcut, it would be a way to lose
/// commits. Above this many, the editor refuses to open and says so.
///
/// 1,000 is far past any interactive rebase anyone plans and still short of
/// the point where the todo list is git's problem rather than ours.
pub const MAX_REBASE_COMMITS: usize = 1_000;

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
    ///
    /// Refuses a range longer than [`MAX_REBASE_COMMITS`]. There is no `-n`
    /// here on purpose: a truncated list would be handed straight back as a
    /// truncated todo file, and git drops what the todo omits.
    pub fn get_rebase_commits(&self, base_oid: &str) -> Result<Vec<RebaseCommit>, GitError> {
        let result = self.git_cmd(&[
            "log",
            "--reverse",
            "--format=%H|%s|%an|%ai",
            &format!("{base_oid}..HEAD"),
        ])?;

        if !result.success {
            return Err(GitError::CliError(result.stderr));
        }

        let count = result.stdout.lines().filter(|l| !l.is_empty()).count();
        if count > MAX_REBASE_COMMITS {
            return Err(GitError::InvalidArgument(format!(
                "{count} commits between {base_oid} and HEAD — more than the {MAX_REBASE_COMMITS} an interactive rebase can be planned for here. Pick a base closer to HEAD."
            )));
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
    #[instrument(skip(self, actions), fields(base = %base_oid, action_count = actions.len()))]
    pub fn start_interactive_rebase(
        &self,
        base_oid: &str,
        actions: &[RebaseAction],
    ) -> Result<(), GitError> {
        // Re-read the range and check the plan still covers it.
        //
        // `git rebase -i` applies exactly the todo file it is handed and drops
        // every commit the file omits, without a word. The plan is a snapshot
        // taken when the editor opened, so anything that moves HEAD while it is
        // open — a commit from a terminal, a pull, the AI background runner —
        // leaves a plan that is missing the new commits. Handing that to git
        // would erase them from the branch.
        //
        // A `drop` is not this case: it travels in the plan with its verb, so
        // the oid is still covered.
        let current = self.get_rebase_commits(base_oid)?;
        let planned: std::collections::HashSet<&str> =
            actions.iter().map(|a| a.oid.as_str()).collect();
        let missing: Vec<&str> = current
            .iter()
            .map(|c| c.oid.as_str())
            .filter(|oid| !planned.contains(oid))
            .collect();
        if !missing.is_empty() {
            return Err(GitError::InvalidArgument(format!(
                "the rebase plan is out of date: {} commit(s) between {base_oid} and HEAD are not in it ({}). Reopen the editor so they are not dropped.",
                missing.len(),
                missing
                    .iter()
                    .take(3)
                    .map(|o| &o[..7.min(o.len())])
                    .collect::<Vec<_>>()
                    .join(", "),
            )));
        }

        // Build the todo list content. Use the FULL oid — a 7-char prefix can
        // be ambiguous in large repos, which makes `git rebase -i` abort with
        // "short SHA1 ... is ambiguous". The frontend already supplies full oids.
        let mut todo = String::new();
        for action in actions {
            todo.push_str(&format!("{} {}\n", action.action, action.oid));
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
            &[
                ("GIT_SEQUENCE_EDITOR", &editor_cmd),
                // `GIT_SEQUENCE_EDITOR` only drives the todo list. A plan
                // containing `squash`/`fixup`/`reword` makes `git rebase -i`
                // additionally open the *commit-message* editor via
                // `GIT_EDITOR`/`core.editor`. Launched from the GUI there is
                // no controlling TTY, so that editor blocks forever and the
                // synchronous `cmd.output()` (see `git_cmd_with_env`) never
                // returns — the command hangs, no `project-mutated` event
                // fires, and the conflict toolbar (the only place Abort
                // lives) never appears, leaving the rebase unrecoverable.
                // `true` accepts git's prepared message non-interactively.
                // Mirrors the `GIT_EDITOR=true` the conflict tests rely on.
                ("GIT_EDITOR", "true"),
            ],
        )?;

        if result.success {
            Ok(())
        } else if result.stderr.contains("CONFLICT") || result.stderr.contains("could not apply") {
            // Conflict is not a fatal error — the ConflictToolbar will handle it.
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }
}

/// Build a rebase todo string from a slice of actions.
#[cfg(test)]
fn build_todo(actions: &[RebaseAction]) -> String {
    let mut todo = String::new();
    for action in actions {
        todo.push_str(&format!("{} {}\n", action.action, action.oid));
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
        // Full OIDs are written verbatim (no 7-char truncation) to avoid
        // ambiguous-SHA aborts in large repos.
        assert_eq!(
            todo,
            "pick abc1234567890\nsquash def5678901234\ndrop short\n"
        );
    }

    /// Run `git` in `dir`, asserting success.
    fn git(dir: &std::path::Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Repo with `n` commits on `main`, identity pinned so the fixture does
    /// not inherit the machine's `~/.gitconfig`.
    fn repo_with(n: usize) -> (tempfile::TempDir, Repository) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        git(path, &["init", "-q", "-b", "main", "."]);
        git(path, &["config", "user.email", "test@example.com"]);
        git(path, &["config", "user.name", "Test"]);
        git(path, &["config", "commit.gpgsign", "false"]);
        for i in 0..n {
            std::fs::write(path.join(format!("f{i}.txt")), format!("{i}\n")).unwrap();
            git(path, &["add", "-A"]);
            git(path, &["commit", "-qm", &format!("c{i}")]);
        }
        let repo = Repository::open(path).unwrap();
        (tmp, repo)
    }

    fn head_oid(repo: &Repository) -> String {
        repo.inner()
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
            .to_string()
    }

    fn commit_count(repo: &Repository) -> usize {
        let mut walk = repo.inner().revwalk().unwrap();
        walk.push_head().unwrap();
        walk.count()
    }

    /// The reason the plan is re-checked before it is handed to git: a todo
    /// file that omits a commit makes `git rebase -i` drop it, silently. So a
    /// plan that no longer covers the range has to be refused, not applied.
    #[test]
    fn a_plan_missing_a_commit_is_refused_and_history_survives() {
        let (tmp, repo) = repo_with(5);
        let base = repo
            .git_cmd(&["rev-parse", "HEAD~4"])
            .unwrap()
            .stdout
            .trim()
            .to_string();

        let commits = repo.get_rebase_commits(&base).unwrap();
        assert_eq!(commits.len(), 4, "base..HEAD is four commits");

        // Plan the first two only — the shape a stale editor snapshot has
        // after two more commits land behind it.
        let stale: Vec<RebaseAction> = commits
            .iter()
            .take(2)
            .map(|c| RebaseAction {
                oid: c.oid.clone(),
                action: "pick".to_string(),
            })
            .collect();

        let before = (head_oid(&repo), commit_count(&repo));
        let err = repo.start_interactive_rebase(&base, &stale).unwrap_err();
        assert!(
            matches!(err, GitError::InvalidArgument(ref m) if m.contains("out of date")),
            "got {err:?}"
        );
        assert_eq!(
            (head_oid(&repo), commit_count(&repo)),
            before,
            "a refused rebase must not touch the branch"
        );
        drop(tmp);
    }

    /// `drop` travels in the plan with its verb, so it still covers the oid —
    /// the coverage check must not mistake it for a missing commit.
    #[test]
    fn dropping_a_commit_is_still_a_covered_plan() {
        let (tmp, repo) = repo_with(4);
        let base = repo
            .git_cmd(&["rev-parse", "HEAD~3"])
            .unwrap()
            .stdout
            .trim()
            .to_string();

        let commits = repo.get_rebase_commits(&base).unwrap();
        assert_eq!(commits.len(), 3);
        let plan: Vec<RebaseAction> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| RebaseAction {
                oid: c.oid.clone(),
                // Drop the middle one.
                action: if i == 1 { "drop" } else { "pick" }.to_string(),
            })
            .collect();

        repo.start_interactive_rebase(&base, &plan)
            .expect("a full plan with a drop is valid");
        assert_eq!(commit_count(&repo), 3, "four commits minus the dropped one");
        drop(tmp);
    }

    #[test]
    fn a_full_plan_is_accepted() {
        let (tmp, repo) = repo_with(3);
        let base = repo
            .git_cmd(&["rev-parse", "HEAD~2"])
            .unwrap()
            .stdout
            .trim()
            .to_string();
        let plan: Vec<RebaseAction> = repo
            .get_rebase_commits(&base)
            .unwrap()
            .iter()
            .map(|c| RebaseAction {
                oid: c.oid.clone(),
                action: "pick".to_string(),
            })
            .collect();

        repo.start_interactive_rebase(&base, &plan).unwrap();
        assert_eq!(commit_count(&repo), 3);
        drop(tmp);
    }
}
