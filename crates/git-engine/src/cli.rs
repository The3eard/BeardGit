//! Git CLI wrapper for operations that `libgit2` cannot perform.
//!
//! Extends [`Repository`] with methods that shell out to the system `git`
//! binary. This covers merge, rebase, cherry-pick, revert, stash, tags, and
//! remote fetch/pull/push. All methods run in the repository's working
//! directory and return a [`GitCliResult`].

// CLI wrapper — shell out to git binary for operations libgit2 can't handle

use std::process::Command;

use serde::Serialize;
use tracing::instrument;

use crate::error::GitError;
use crate::repository::Repository;

/// Output captured from a single `git` CLI invocation.
#[derive(Debug, Clone)]
pub struct GitCliResult {
    /// `true` when the process exited with status 0.
    pub success: bool,
    /// Everything written to stdout, decoded as UTF-8 (lossy).
    pub stdout: String,
    /// Everything written to stderr, decoded as UTF-8 (lossy).
    pub stderr: String,
}

/// A parsed stash entry with structured metadata.
#[derive(Debug, Clone, Serialize)]
pub struct StashEntry {
    /// Zero-based stash index (`stash@{index}`).
    pub index: usize,
    /// User-provided or auto-generated stash message.
    pub message: String,
    /// Branch name where the stash was created.
    pub branch: String,
    /// Unix timestamp (seconds since epoch) when the stash was created.
    pub timestamp: i64,
    /// Full commit SHA of the stash.
    pub oid: String,
}

/// A structured description of a single git tag.
#[derive(Debug, Clone, Serialize)]
pub struct TagInfo {
    /// The short ref name of the tag (e.g. `v1.0.0`).
    pub name: String,
    /// The OID of the tag object itself (for annotated tags) or the commit (for lightweight).
    pub object_oid: String,
    /// The OID of the commit the tag ultimately points to.
    pub commit_oid: String,
    /// `true` when the tag is annotated (has its own object), `false` for lightweight tags.
    pub annotated: bool,
    /// The tag message (empty for lightweight tags).
    pub message: String,
    /// The name of the person who created the annotated tag (empty for lightweight).
    pub tagger_name: String,
    /// The email of the person who created the annotated tag (empty for lightweight).
    pub tagger_email: String,
    /// ISO-8601 date when the annotated tag was created (empty for lightweight).
    pub date: String,
}

/// Diff statistics for a single commit.
#[derive(Debug, Clone, Serialize)]
pub struct CommitStats {
    /// Number of files changed in this commit.
    pub files_changed: u32,
    /// Total lines inserted.
    pub insertions: u32,
    /// Total lines deleted.
    pub deletions: u32,
}

/// Parse the output of `git tag -l --format=...` into [`TagInfo`] structs.
///
/// Each line is expected to contain 8 pipe-separated fields:
/// `name|objecttype|objectname|*objectname|taggername|taggeremail|taggerdate|subject`.
fn parse_tag_list(output: &str) -> Vec<TagInfo> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.splitn(8, '|').collect();
            if parts.len() < 8 {
                return TagInfo {
                    name: line.to_string(),
                    object_oid: String::new(),
                    commit_oid: String::new(),
                    annotated: false,
                    message: String::new(),
                    tagger_name: String::new(),
                    tagger_email: String::new(),
                    date: String::new(),
                };
            }
            let name = parts[0].to_string();
            let obj_type = parts[1];
            let object_oid = parts[2].to_string();
            let deref_oid = parts[3].to_string();
            let annotated = obj_type == "tag";
            let commit_oid = if annotated && !deref_oid.is_empty() {
                deref_oid
            } else {
                object_oid.clone()
            };
            // For lightweight tags %(subject) returns the commit message, not a
            // tag message, so we only populate tagger/message fields when the
            // tag object is actually annotated.
            let (tagger_name, tagger_email, date, message) = if annotated {
                (
                    parts[4].to_string(),
                    parts[5]
                        .trim_start_matches('<')
                        .trim_end_matches('>')
                        .to_string(),
                    parts[6].to_string(),
                    parts[7].to_string(),
                )
            } else {
                (String::new(), String::new(), String::new(), String::new())
            };
            TagInfo {
                name,
                object_oid,
                commit_oid,
                annotated,
                message,
                tagger_name,
                tagger_email,
                date,
            }
        })
        .collect()
}

/// Parse the summary line from `git diff --stat` output.
///
/// The last line looks like: " 3 files changed, 45 insertions(+), 12 deletions(-)"
/// or just " 1 file changed, 2 insertions(+)" etc.
fn parse_stat_summary(output: &str) -> CommitStats {
    let mut files_changed: u32 = 0;
    let mut insertions: u32 = 0;
    let mut deletions: u32 = 0;

    // Find the summary line (last non-empty line containing "changed")
    if let Some(summary) = output.lines().rev().find(|l| l.contains("changed")) {
        for part in summary.split(',') {
            let part = part.trim();
            if part.contains("file") && part.contains("changed") {
                files_changed = part
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
            } else if part.contains("insertion") {
                insertions = part
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
            } else if part.contains("deletion") {
                deletions = part
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
            }
        }
    }

    CommitStats {
        files_changed,
        insertions,
        deletions,
    }
}

/// Apply Windows-specific process creation flags to hide the console window.
#[cfg(target_os = "windows")]
fn configure_no_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000);
}

/// No-op on non-Windows platforms.
#[cfg(not(target_os = "windows"))]
fn configure_no_window(_cmd: &mut Command) {}

impl Repository {
    /// Run a git command in the repository directory.
    ///
    /// On Windows, the `CREATE_NO_WINDOW` flag is set to prevent a visible
    /// console window from flashing on screen.
    pub fn git_cmd(&self, args: &[&str]) -> Result<GitCliResult, GitError> {
        let mut cmd = Command::new("git");
        cmd.args(args).current_dir(self.path());
        configure_no_window(&mut cmd);

        let output = cmd.output().map_err(GitError::Io)?;

        Ok(GitCliResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Run a git command with additional environment variables.
    ///
    /// Behaves identically to [`git_cmd`] but injects extra environment
    /// variables into the child process. This is used, for example, to set
    /// `GIT_SEQUENCE_EDITOR` for non-interactive interactive rebases.
    pub fn git_cmd_with_env(
        &self,
        args: &[&str],
        env_vars: &[(&str, &str)],
    ) -> Result<GitCliResult, GitError> {
        let mut cmd = Command::new("git");
        cmd.args(args).current_dir(self.path());
        for (key, val) in env_vars {
            cmd.env(key, val);
        }
        configure_no_window(&mut cmd);

        let output = cmd.output().map_err(GitError::Io)?;

        Ok(GitCliResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Merge `branch` into the current branch using `--no-edit` (no interactive prompt).
    #[instrument(skip(self), fields(branch = %branch))]
    pub fn merge_branch(&self, branch: &str) -> Result<GitCliResult, GitError> {
        self.git_cmd(&["merge", branch, "--no-edit"])
    }

    /// Rebase the current branch onto `onto`.
    #[instrument(skip(self), fields(onto = %onto))]
    pub fn rebase_branch(&self, onto: &str) -> Result<GitCliResult, GitError> {
        self.git_cmd(&["rebase", onto])
    }

    /// Cherry-pick a single commit by its OID onto the current branch.
    #[instrument(skip(self), fields(oid = %oid))]
    pub fn cherry_pick(&self, oid: &str) -> Result<GitCliResult, GitError> {
        self.git_cmd(&["cherry-pick", oid])
    }

    /// Revert a single commit by its OID, creating a new revert commit automatically.
    #[instrument(skip(self), fields(oid = %oid))]
    pub fn revert_commit(&self, oid: &str) -> Result<GitCliResult, GitError> {
        self.git_cmd(&["revert", oid, "--no-edit"])
    }

    /// Save uncommitted changes to the stash, optionally with a description message.
    #[instrument(skip(self))]
    pub fn stash_push(&self, message: Option<&str>) -> Result<GitCliResult, GitError> {
        match message {
            Some(msg) => self.git_cmd(&["stash", "push", "-m", msg]),
            None => self.git_cmd(&["stash", "push"]),
        }
    }

    /// Apply and remove the stash entry at `index` (defaults to the latest stash).
    #[instrument(skip(self))]
    pub fn stash_pop(&self, index: Option<usize>) -> Result<GitCliResult, GitError> {
        match index {
            Some(i) => {
                let stash_ref = format!("stash@{{{i}}}");
                self.git_cmd(&["stash", "pop", &stash_ref])
            }
            None => self.git_cmd(&["stash", "pop"]),
        }
    }

    /// Return each line of `git stash list` as a separate string.
    pub fn stash_list(&self) -> Result<Vec<String>, GitError> {
        let result = self.git_cmd(&["stash", "list"])?;
        Ok(result
            .stdout
            .lines()
            .map(String::from)
            .filter(|l| !l.is_empty())
            .collect())
    }

    /// Delete the stash entry at `index` without applying it (defaults to the latest).
    #[instrument(skip(self))]
    pub fn stash_drop(&self, index: Option<usize>) -> Result<GitCliResult, GitError> {
        match index {
            Some(i) => {
                let stash_ref = format!("stash@{{{i}}}");
                self.git_cmd(&["stash", "drop", &stash_ref])
            }
            None => self.git_cmd(&["stash", "drop"]),
        }
    }

    /// Apply the stash entry at `index` without removing it (defaults to the latest).
    #[instrument(skip(self))]
    pub fn stash_apply(&self, index: Option<usize>) -> Result<GitCliResult, GitError> {
        match index {
            Some(i) => {
                let stash_ref = format!("stash@{{{i}}}");
                self.git_cmd(&["stash", "apply", &stash_ref])
            }
            None => self.git_cmd(&["stash", "apply"]),
        }
    }

    /// Restore a single file from a stash entry into the working directory.
    ///
    /// Uses `git restore --source=stash@{index} -- path` to apply only the
    /// specified file without touching the rest of the working tree.
    #[instrument(skip(self), fields(path = %path))]
    pub fn stash_apply_file(&self, index: usize, path: &str) -> Result<GitCliResult, GitError> {
        let stash_ref = format!("stash@{{{index}}}");
        self.git_cmd(&["restore", "--source", &stash_ref, "--", path])
    }

    /// Return the diff of a stash entry as structured [`FileDiff`] objects.
    ///
    /// Uses `git stash show -p` to get unified diff text, then parses it into the
    /// same [`FileDiff`] structures used by [`Repository::diff_workdir`] and
    /// [`Repository::diff_index`].
    pub fn stash_show_parsed(
        &self,
        index: Option<usize>,
    ) -> Result<Vec<crate::diff::FileDiff>, GitError> {
        let result = match index {
            Some(i) => {
                let stash_ref = format!("stash@{{{i}}}");
                self.git_cmd(&["stash", "show", "-p", &stash_ref])?
            }
            None => self.git_cmd(&["stash", "show", "-p"])?,
        };

        if !result.success {
            return Err(GitError::Io(std::io::Error::other(result.stderr)));
        }

        Ok(crate::diff::parse_unified_diff(&result.stdout))
    }

    /// Return structured stash entries parsed from `git stash list`.
    pub fn stash_entries(&self) -> Result<Vec<StashEntry>, GitError> {
        let result = self.git_cmd(&["stash", "list", "--format=%gd|%gs|%ai|%H"])?;
        if result.stdout.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        for line in result.stdout.lines() {
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(4, '|').collect();
            if parts.len() < 4 {
                continue;
            }

            // Parse index from "stash@{0}"
            let index = parts[0]
                .trim_start_matches("stash@{")
                .trim_end_matches('}')
                .parse::<usize>()
                .unwrap_or(0);

            // Parse branch and message from subject like "On main: my message"
            // or "WIP on main: abc1234 commit msg"
            let subject = parts[1];
            let (branch, message) = if let Some(rest) = subject.strip_prefix("On ") {
                if let Some((b, m)) = rest.split_once(": ") {
                    (b.to_string(), m.to_string())
                } else {
                    (rest.to_string(), String::new())
                }
            } else if let Some(rest) = subject.strip_prefix("WIP on ") {
                if let Some((b, m)) = rest.split_once(": ") {
                    // WIP messages look like "abc1234 commit msg" — strip the leading SHA
                    let cleaned = m
                        .split_once(' ')
                        .filter(|(first, _)| {
                            first.len() >= 7 && first.chars().all(|c| c.is_ascii_hexdigit())
                        })
                        .map(|(_, rest)| rest.to_string())
                        .unwrap_or_else(|| m.to_string());
                    (b.to_string(), cleaned)
                } else {
                    (rest.to_string(), String::new())
                }
            } else {
                (String::new(), subject.to_string())
            };

            // Parse ISO 8601 timestamp to unix seconds
            let timestamp =
                chrono::DateTime::parse_from_str(parts[2].trim(), "%Y-%m-%d %H:%M:%S %z")
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0);

            let oid = parts[3].trim().to_string();

            entries.push(StashEntry {
                index,
                message,
                branch,
                timestamp,
                oid,
            });
        }

        Ok(entries)
    }

    /// Create a lightweight tag (`name`) or an annotated tag when `message` is provided.
    #[instrument(skip(self), fields(tag = %name))]
    pub fn create_tag(&self, name: &str, message: Option<&str>) -> Result<GitCliResult, GitError> {
        let result = match message {
            Some(msg) => self.git_cmd(&["tag", "-a", name, "-m", msg])?,
            None => self.git_cmd(&["tag", name])?,
        };
        if result.success {
            self.invalidate_tag_cache();
        }
        Ok(result)
    }

    /// Delete a local tag by name.
    #[instrument(skip(self), fields(tag = %name))]
    pub fn delete_tag(&self, name: &str) -> Result<GitCliResult, GitError> {
        let result = self.git_cmd(&["tag", "-d", name])?;
        if result.success {
            self.invalidate_tag_cache();
        }
        Ok(result)
    }

    /// List all local tags with full metadata, sorted by descending version.
    ///
    /// Returns a [`TagInfo`] for every tag in the repository. Annotated tags
    /// include the tagger name/email, date, and message; lightweight tags leave
    /// those fields empty.
    pub fn tags(&self) -> Result<Vec<TagInfo>, GitError> {
        let format = "%(refname:short)|%(objecttype)|%(objectname)|%(*objectname)|%(taggername)|%(taggeremail)|%(taggerdate:iso-strict)|%(subject)";
        let result = self.git_cmd(&[
            "tag",
            "-l",
            "--sort=-version:refname",
            &format!("--format={format}"),
        ])?;
        Ok(parse_tag_list(&result.stdout))
    }

    /// Get the full tag list, using cache if available.
    fn tags_cached(&self) -> Result<Vec<TagInfo>, GitError> {
        let cache = self.tag_cache.lock().unwrap();
        if let Some(ref tags) = *cache {
            return Ok(tags.clone());
        }
        drop(cache);
        let tags = self.tags()?;
        let mut cache = self.tag_cache.lock().unwrap();
        *cache = Some(tags.clone());
        Ok(tags)
    }

    /// Invalidate the cached tag list (call after create/delete).
    pub fn invalidate_tag_cache(&self) {
        let mut cache = self.tag_cache.lock().unwrap();
        *cache = None;
    }

    /// List local tags with pagination, sorted by descending version.
    ///
    /// `page` is 1-based. Returns up to `per_page` tags starting from
    /// `(page - 1) * per_page`.
    pub fn tags_paginated(&self, per_page: u32, page: u32) -> Result<Vec<TagInfo>, GitError> {
        let all = self.tags_cached()?;
        let start = ((page.saturating_sub(1)) * per_page) as usize;
        if start >= all.len() {
            return Ok(Vec::new());
        }
        let end = (start + per_page as usize).min(all.len());
        Ok(all[start..end].to_vec())
    }

    /// Search tags whose name matches `query` using git-native glob filtering.
    ///
    /// Uses `git tag -l '*query*'` for server-side filtering, which is
    /// significantly faster than loading and filtering all tags in-process.
    /// Note: matching is case-sensitive (git glob does not support `-i`).
    pub fn search_tags(&self, query: &str) -> Result<Vec<TagInfo>, GitError> {
        let format = "%(refname:short)|%(objecttype)|%(objectname)|%(*objectname)|%(taggername)|%(taggeremail)|%(taggerdate:iso-strict)|%(subject)";
        let pattern = format!("*{}*", query);
        let result = self.git_cmd(&[
            "tag",
            "-l",
            "--sort=-version:refname",
            &format!("--format={format}"),
            &pattern,
        ])?;
        Ok(parse_tag_list(&result.stdout))
    }

    /// Push a tag (or all tags) to the named remote.
    ///
    /// When `tag_name` is `Some`, only that specific tag ref is pushed.
    /// When `tag_name` is `None`, all local tags are pushed (`--tags`).
    #[instrument(skip(self), fields(remote = %remote))]
    pub fn push_tag(&self, remote: &str, tag_name: Option<&str>) -> Result<GitCliResult, GitError> {
        // `--` terminates flag parsing so a remote name like `--something`
        // (rare but configurable) cannot be misread as an option.
        match tag_name {
            Some(name) => self.git_cmd(&["push", "--", remote, &format!("refs/tags/{name}")]),
            None => self.git_cmd(&["push", "--tags", "--", remote]),
        }
    }

    /// Return diff statistics (files changed, insertions, deletions) for a commit.
    ///
    /// Uses `git diff --no-ext-diff --stat=999 <oid>^..<oid>` and parses the
    /// summary line. `--no-ext-diff` bypasses any global `diff.external` config
    /// (e.g. `difftastic`) so the parser always sees the canonical numstat
    /// summary. For root commits (no parent), uses
    /// `git diff-tree --stat=999 --root <oid>`.
    pub fn commit_stats(&self, oid: &str) -> Result<CommitStats, GitError> {
        // Try normal diff first (commit with parent)
        let result = self.git_cmd(&[
            "diff",
            "--no-ext-diff",
            "--stat=999",
            &format!("{oid}^..{oid}"),
        ])?;

        let output = if result.success {
            result.stdout
        } else {
            // Fallback for root commits: use diff-tree with --root
            let root_result = self.git_cmd(&["diff-tree", "--stat=999", "--root", oid])?;
            root_result.stdout
        };

        Ok(parse_stat_summary(&output))
    }

    /// Fetch all updates from the named remote.
    #[instrument(skip(self), fields(remote = %remote))]
    pub fn fetch_remote(&self, remote: &str) -> Result<GitCliResult, GitError> {
        self.git_cmd(&["fetch", "--", remote])
    }

    /// Pull `branch` from `remote` into the current branch.
    #[instrument(skip(self), fields(remote = %remote, branch = %branch))]
    pub fn pull_remote(&self, remote: &str, branch: &str) -> Result<GitCliResult, GitError> {
        self.git_cmd(&["pull", "--", remote, branch])
    }

    /// Build the argv vector for `git push`. Extracted so callers can
    /// unit-test the flag composition without shelling out.
    ///
    /// All flags appear before the `--` separator; `remote` and `branch`
    /// are positional, never flag-parsed, even if a future caller passes
    /// values that begin with `-`.
    pub fn push_args<'a>(&self, remote: &'a str, branch: &'a str, force: bool) -> Vec<&'a str> {
        let mut args: Vec<&str> = vec!["push", "-u"];
        if force {
            args.push("--force-with-lease");
        }
        args.push("--");
        args.push(remote);
        args.push(branch);
        args
    }

    /// Push `branch` to `remote`, optionally with `--force-with-lease`.
    ///
    /// Always passes `-u` so the first successful push establishes the
    /// upstream tracking ref; subsequent pushes are idempotent.
    #[instrument(skip(self), fields(remote = %remote, branch = %branch, force))]
    pub fn push_remote(
        &self,
        remote: &str,
        branch: &str,
        force: bool,
    ) -> Result<GitCliResult, GitError> {
        let args = self.push_args(remote, branch, force);
        self.git_cmd(&args)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn create_test_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        // Configure identity so git CLI commands work
        git_repo
            .config()
            .unwrap()
            .set_str("user.name", "Test")
            .unwrap();
        git_repo
            .config()
            .unwrap()
            .set_str("user.email", "test@test.com")
            .unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        fs::write(dir.path().join("file.txt"), "content\n").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = git_repo.find_tree(tree_id).unwrap();
        git_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        (dir, repo)
    }

    #[test]
    fn test_git_cmd_runs() {
        let (_dir, repo) = create_test_repo();
        let result = repo.git_cmd(&["status"]).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_stash_operations() {
        let (dir, repo) = create_test_repo();
        fs::write(dir.path().join("file.txt"), "dirty\n").unwrap();
        let result = repo.stash_push(Some("test stash")).unwrap();
        assert!(result.success);
        let stashes = repo.stash_list().unwrap();
        assert_eq!(stashes.len(), 1);
        let result = repo.stash_pop(None).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_create_and_delete_tag() {
        let (_dir, repo) = create_test_repo();
        let result = repo.create_tag("v1.0.0", Some("Release 1.0")).unwrap();
        assert!(result.success);
        let result = repo.delete_tag("v1.0.0").unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_stash_apply() {
        let (dir, repo) = create_test_repo();
        fs::write(dir.path().join("file.txt"), "dirty\n").unwrap();
        let result = repo.stash_push(Some("apply test")).unwrap();
        assert!(result.success);

        let result = repo.stash_apply(None).unwrap();
        assert!(result.success);

        // Stash should still exist after apply (unlike pop)
        let stashes = repo.stash_list().unwrap();
        assert_eq!(stashes.len(), 1);
    }

    #[test]
    fn test_stash_entries_empty() {
        let (_dir, repo) = create_test_repo();
        let entries = repo.stash_entries().unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_stash_entries_parsed() {
        let (dir, repo) = create_test_repo();
        fs::write(dir.path().join("file.txt"), "dirty\n").unwrap();
        repo.stash_push(Some("my stash message")).unwrap();

        let entries = repo.stash_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].index, 0);
        assert!(entries[0].message.contains("my stash message"));
        assert!(!entries[0].branch.is_empty());
        assert!(entries[0].timestamp > 0);
        assert!(!entries[0].oid.is_empty());
    }

    #[test]
    fn test_merge() {
        let (dir, repo) = create_test_repo();
        // Create a branch with a commit
        repo.create_branch("feature").unwrap();
        repo.checkout_branch("feature").unwrap();
        fs::write(dir.path().join("feature.txt"), "feature work\n").unwrap();
        repo.stage_files(&["feature.txt".to_string()]).unwrap();
        repo.create_commit("Feature commit").unwrap();
        // Switch back and merge
        repo.checkout_branch("master").unwrap();
        let result = repo.merge_branch("feature").unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_stash_show_parsed() {
        let (dir, repo) = create_test_repo();
        fs::write(dir.path().join("file.txt"), "modified content\n").unwrap();
        repo.stash_push(Some("diff test")).unwrap();

        let diffs = repo.stash_show_parsed(None).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "file.txt");
        assert_eq!(diffs[0].status, "modified");
        assert!(!diffs[0].hunks.is_empty());
        assert!(diffs[0].additions > 0 || diffs[0].deletions > 0);
    }

    #[test]
    fn test_stash_show_parsed_new_file() {
        let (dir, repo) = create_test_repo();
        fs::write(dir.path().join("new.txt"), "brand new\n").unwrap();
        // Stage the new file so stash captures it
        repo.stage_files(&["new.txt".to_string()]).unwrap();
        repo.stash_push(Some("new file stash")).unwrap();

        let diffs = repo.stash_show_parsed(None).unwrap();
        let new_diff = diffs.iter().find(|d| d.path == "new.txt").unwrap();
        assert_eq!(new_diff.status, "added");
        assert!(new_diff.additions > 0);
    }

    #[test]
    fn test_stash_apply_file() {
        let (dir, repo) = create_test_repo();
        // Create two dirty files
        fs::write(dir.path().join("file.txt"), "modified\n").unwrap();
        fs::write(dir.path().join("other.txt"), "new file\n").unwrap();
        repo.stage_files(&["other.txt".to_string()]).unwrap();
        repo.stash_push(Some("multi-file stash")).unwrap();

        // Apply only file.txt from the stash
        let result = repo.stash_apply_file(0, "file.txt").unwrap();
        assert!(result.success);

        // file.txt should be restored
        let content = fs::read_to_string(dir.path().join("file.txt")).unwrap();
        assert_eq!(content, "modified\n");

        // other.txt should NOT be restored (still at original or absent)
        assert!(
            !dir.path().join("other.txt").exists()
                || fs::read_to_string(dir.path().join("other.txt")).unwrap() != "new file\n"
        );
    }

    #[test]
    fn test_tags_returns_empty_on_no_tags() {
        let (_dir, repo) = create_test_repo();
        let tags = repo.tags().unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn test_tags_returns_lightweight_tag() {
        let (_dir, repo) = create_test_repo();
        repo.create_tag("v1.0.0", None).unwrap();
        let tags = repo.tags().unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1.0.0");
        assert!(!tags[0].annotated);
        assert!(tags[0].message.is_empty());
    }

    #[test]
    fn test_tags_returns_annotated_tag() {
        let (_dir, repo) = create_test_repo();
        repo.create_tag("v2.0.0", Some("Release 2.0")).unwrap();
        let tags = repo.tags().unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v2.0.0");
        assert!(tags[0].annotated);
        assert!(tags[0].message.contains("Release 2.0"));
        assert!(!tags[0].tagger_name.is_empty());
    }

    #[test]
    fn test_tags_returns_multiple_sorted() {
        let (_dir, repo) = create_test_repo();
        repo.create_tag("v1.0.0", None).unwrap();
        repo.create_tag("v1.1.0", None).unwrap();
        repo.create_tag("v2.0.0", Some("Major release")).unwrap();
        let tags = repo.tags().unwrap();
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].name, "v2.0.0");
        assert_eq!(tags[1].name, "v1.1.0");
        assert_eq!(tags[2].name, "v1.0.0");
    }

    #[test]
    fn test_delete_tag_removes_from_list() {
        let (_dir, repo) = create_test_repo();
        repo.create_tag("v1.0.0", None).unwrap();
        assert_eq!(repo.tags().unwrap().len(), 1);
        repo.delete_tag("v1.0.0").unwrap();
        assert!(repo.tags().unwrap().is_empty());
    }

    #[test]
    fn test_tag_commit_oid_resolves_to_commit() {
        let (_dir, repo) = create_test_repo();
        // Create both lightweight and annotated tags
        repo.create_tag("v1.0.0", None).unwrap();
        repo.create_tag("v2.0.0", Some("Annotated")).unwrap();

        let tags = repo.tags().unwrap();
        for tag in &tags {
            // commit_oid must be a full 40-char hex OID
            assert_eq!(
                tag.commit_oid.len(),
                40,
                "commit_oid for {} is not 40 chars: '{}'",
                tag.name,
                tag.commit_oid
            );
            // Must resolve via get_commit
            let commit = repo.get_commit(&tag.commit_oid);
            assert!(
                commit.is_ok(),
                "get_commit failed for tag {} with oid {}: {:?}",
                tag.name,
                tag.commit_oid,
                commit.err()
            );
            // Must resolve via commit_files
            let files = repo.commit_files(&tag.commit_oid);
            assert!(
                files.is_ok(),
                "commit_files failed for tag {} with oid {}: {:?}",
                tag.name,
                tag.commit_oid,
                files.err()
            );
        }
    }

    #[test]
    fn test_parse_stat_summary_full() {
        let output = " src/main.rs | 10 +++++-----\n src/lib.rs  |  5 +++++\n 2 files changed, 10 insertions(+), 5 deletions(-)\n";
        let stats = parse_stat_summary(output);
        assert_eq!(stats.files_changed, 2);
        assert_eq!(stats.insertions, 10);
        assert_eq!(stats.deletions, 5);
    }

    #[test]
    fn test_parse_stat_summary_insertions_only() {
        let output = " file.txt | 3 +++\n 1 file changed, 3 insertions(+)\n";
        let stats = parse_stat_summary(output);
        assert_eq!(stats.files_changed, 1);
        assert_eq!(stats.insertions, 3);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_parse_stat_summary_empty() {
        let stats = parse_stat_summary("");
        assert_eq!(stats.files_changed, 0);
        assert_eq!(stats.insertions, 0);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_commit_stats_on_initial_commit() {
        let (_dir, repo) = create_test_repo();
        let tags = repo.git_cmd(&["rev-parse", "HEAD"]).unwrap();
        let head_oid = tags.stdout.trim();
        let stats = repo.commit_stats(head_oid).unwrap();
        assert_eq!(stats.files_changed, 1);
        assert!(stats.insertions > 0);
    }

    #[test]
    fn test_commit_stats_on_second_commit() {
        let (dir, repo) = create_test_repo();
        std::fs::write(dir.path().join("file.txt"), "line1\nline2\nline3\n").unwrap();
        std::fs::write(dir.path().join("new.txt"), "new content\n").unwrap();
        repo.stage_files(&["file.txt".to_string(), "new.txt".to_string()])
            .unwrap();
        repo.create_commit("Second commit").unwrap();

        let result = repo.git_cmd(&["rev-parse", "HEAD"]).unwrap();
        let head_oid = result.stdout.trim();
        let stats = repo.commit_stats(head_oid).unwrap();
        assert_eq!(stats.files_changed, 2);
        assert!(stats.insertions > 0);
    }

    #[test]
    fn test_tags_paginated_first_page() {
        let (_dir, repo) = create_test_repo();
        repo.create_tag("v1.0.0", None).unwrap();
        repo.create_tag("v1.1.0", None).unwrap();
        repo.create_tag("v2.0.0", Some("Release")).unwrap();

        let page1 = repo.tags_paginated(2, 1).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].name, "v2.0.0");
        assert_eq!(page1[1].name, "v1.1.0");
    }

    #[test]
    fn test_tags_paginated_second_page() {
        let (_dir, repo) = create_test_repo();
        repo.create_tag("v1.0.0", None).unwrap();
        repo.create_tag("v1.1.0", None).unwrap();
        repo.create_tag("v2.0.0", Some("Release")).unwrap();

        let page2 = repo.tags_paginated(2, 2).unwrap();
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].name, "v1.0.0");
    }

    #[test]
    fn test_tags_paginated_beyond_end() {
        let (_dir, repo) = create_test_repo();
        repo.create_tag("v1.0.0", None).unwrap();

        let page = repo.tags_paginated(10, 2).unwrap();
        assert!(page.is_empty());
    }

    #[test]
    fn test_search_tags_by_name() {
        let (_dir, repo) = create_test_repo();
        repo.create_tag("v1.0.0", None).unwrap();
        repo.create_tag("v2.0.0-beta", None).unwrap();
        repo.create_tag("release-1", None).unwrap();

        let results = repo.search_tags("beta").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "v2.0.0-beta");
    }

    #[test]
    fn test_search_tags_case_sensitive_git_native() {
        // git tag -l glob is case-sensitive; uppercase query matches uppercase tag.
        let (_dir, repo) = create_test_repo();
        repo.create_tag("V1.0.0-RC", None).unwrap();

        let results = repo.search_tags("RC").unwrap();
        assert_eq!(results.len(), 1);

        // Lowercase query should NOT match the uppercase tag (case-sensitive glob).
        let no_results = repo.search_tags("rc").unwrap();
        assert!(no_results.is_empty());
    }

    #[test]
    fn push_remote_builds_force_with_lease_args() {
        // Verifies the CLI wrapper passes `--force-with-lease` when
        // `force=true` by asserting the arg vector through `push_args`.
        let (_dir, repo) = create_test_repo();
        let args = repo.push_args("origin", "main", true);
        assert_eq!(
            args,
            vec!["push", "-u", "--force-with-lease", "--", "origin", "main"]
        );
    }

    #[test]
    fn push_remote_builds_plain_args() {
        let (_dir, repo) = create_test_repo();
        let args = repo.push_args("origin", "main", false);
        assert_eq!(args, vec!["push", "-u", "--", "origin", "main"]);
    }
}
