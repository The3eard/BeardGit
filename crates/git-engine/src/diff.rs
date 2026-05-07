//! Diff computation between working directory, index, and commits.
//!
//! Extends [`Repository`] with methods that produce structured diff data
//! suitable for rendering in the frontend. All diff types are built on top
//! of `libgit2`'s diff API.

// Diff module — workdir vs index and index vs HEAD

use git2::{Diff, DiffOptions};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use crate::error::GitError;
use crate::repository::Repository;

/// Map a libgit2 delta status to a human-readable string.
fn delta_status_str(delta: git2::Delta) -> &'static str {
    match delta {
        git2::Delta::Added => "added",
        git2::Delta::Deleted => "deleted",
        git2::Delta::Modified => "modified",
        git2::Delta::Renamed => "renamed",
        git2::Delta::Copied => "copied",
        git2::Delta::Untracked => "untracked",
        _ => "unknown",
    }
}

/// A single file changed in a commit, with its status relative to the first parent.
#[derive(Debug, Clone, Serialize)]
pub struct CommitFileChange {
    /// Repo-relative path of the file (new path for renames).
    pub path: String,
    /// Change type: `"added"`, `"deleted"`, `"modified"`, `"renamed"`, or `"copied"`.
    pub status: String,
}

/// Complete diff for a single file including all hunks and line-level changes.
#[derive(Debug, Clone, Serialize)]
pub struct FileDiff {
    /// Repo-relative path of the file (new path for renames).
    pub path: String,
    /// Previous path of the file when it was renamed, otherwise `None`.
    pub old_path: Option<String>,
    /// Change type: `"added"`, `"deleted"`, `"modified"`, `"renamed"`, `"copied"`, or `"untracked"`.
    pub status: String,
    /// Ordered list of diff hunks within this file.
    pub hunks: Vec<DiffHunkInfo>,
    /// Total number of added lines across all hunks.
    pub additions: usize,
    /// Total number of deleted lines across all hunks.
    pub deletions: usize,
    /// `true` when the diff was truncated for performance (e.g. a single
    /// commit touched a 50 MB minified blob). The frontend renders a
    /// "diff too large to display" placeholder. Defaults to `false` so
    /// existing call sites and JSON payloads stay untouched.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

/// Maximum size, in bytes, of the raw `git diff` output rendered in
/// `commit_file_diff`. Beyond this, parsing is skipped and a synthetic
/// truncated `FileDiff` is returned. 5 MB is enough for any honest
/// commit while keeping IPC + frontend rendering responsive.
pub const MAX_COMMIT_DIFF_BYTES: usize = 5 * 1024 * 1024;

/// A single contiguous diff hunk within a file.
#[derive(Debug, Clone, Serialize)]
pub struct DiffHunkInfo {
    /// The `@@ ... @@` hunk header string.
    pub header: String,
    /// Starting line number in the old file.
    pub old_start: u32,
    /// Number of lines from the old file covered by this hunk.
    pub old_lines: u32,
    /// Starting line number in the new file.
    pub new_start: u32,
    /// Number of lines in the new file covered by this hunk.
    pub new_lines: u32,
    /// Individual lines in the hunk (context, additions, deletions).
    pub lines: Vec<DiffLineInfo>,
}

/// A single line within a diff hunk.
#[derive(Debug, Clone, Serialize)]
pub struct DiffLineInfo {
    /// Line origin character: `'+'` for added, `'-'` for deleted, `' '` for context.
    pub origin: char,
    /// Raw text content of the line (may include a trailing newline).
    pub content: String,
    /// Line number in the old file, or `None` for added lines.
    pub old_lineno: Option<u32>,
    /// Line number in the new file, or `None` for deleted lines.
    pub new_lineno: Option<u32>,
}

impl Repository {
    /// Diff between working directory and index (unstaged changes).
    pub fn diff_workdir(&self) -> Result<Vec<FileDiff>, GitError> {
        let repo = self.inner();
        let diff = repo.diff_index_to_workdir(
            None,
            Some(
                DiffOptions::new()
                    .include_untracked(true)
                    .show_untracked_content(true),
            ),
        )?;
        collect_file_diffs(&diff)
    }

    /// Diff between index and HEAD (staged changes).
    pub fn diff_index(&self) -> Result<Vec<FileDiff>, GitError> {
        let repo = self.inner();
        let head_tree = repo.head()?.peel_to_tree()?;
        let diff = repo.diff_tree_to_index(Some(&head_tree), None, None)?;
        collect_file_diffs(&diff)
    }

    /// Get the list of files changed in a specific commit (compared to its first parent).
    pub fn commit_files(&self, oid_str: &str) -> Result<Vec<CommitFileChange>, GitError> {
        let repo = self.inner();
        let oid = git2::Oid::from_str(oid_str)?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;

        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

        let mut files = Vec::new();
        for delta in diff.deltas() {
            let path = delta
                .new_file()
                .path()
                .unwrap_or(std::path::Path::new(""))
                .to_string_lossy()
                .to_string();
            files.push(CommitFileChange {
                path,
                status: delta_status_str(delta.status()).to_string(),
            });
        }
        Ok(files)
    }

    /// Return files changed between two arbitrary commits.
    ///
    /// Uses `diff_tree_to_tree` to compare `from_oid` and `to_oid` directly,
    /// without assuming any parent–child relationship. This is used, for example,
    /// to show what a merged branch contributed relative to the merge base.
    pub fn diff_commits(
        &self,
        from_oid: &str,
        to_oid: &str,
    ) -> Result<Vec<CommitFileChange>, GitError> {
        let repo = self.inner();
        let from_commit = repo.find_commit(git2::Oid::from_str(from_oid)?)?;
        let to_commit = repo.find_commit(git2::Oid::from_str(to_oid)?)?;
        let from_tree = from_commit.tree()?;
        let to_tree = to_commit.tree()?;

        let diff = repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?;
        let mut files = Vec::new();
        for delta in diff.deltas() {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            files.push(CommitFileChange {
                path,
                status: delta_status_str(delta.status()).to_string(),
            });
        }
        Ok(files)
    }

    /// Return the full diff (with hunks/lines) for a single file in a commit.
    ///
    /// Uses `git diff --no-ext-diff <oid>^..<oid> -- <path>` via CLI, then
    /// parses with `parse_unified_diff`. The `--no-ext-diff` flag bypasses any
    /// global `diff.external` config (e.g. `difftastic`) so the output is
    /// always parseable unified diff. For root commits (no parent), uses
    /// `git diff-tree -p`.
    pub fn commit_file_diff(&self, oid: &str, path: &str) -> Result<Vec<FileDiff>, GitError> {
        let result = self.git_cmd(&[
            "diff",
            "--no-ext-diff",
            &format!("{oid}^..{oid}"),
            "--",
            path,
        ]);

        let output = match result {
            Ok(r) if r.success => r.stdout,
            _ => {
                // Fallback for root commits
                let root = self.git_cmd(&["diff-tree", "-p", "--root", oid, "--", path])?;
                root.stdout
            }
        };

        if output.len() > MAX_COMMIT_DIFF_BYTES {
            return Ok(vec![FileDiff {
                path: path.to_string(),
                old_path: None,
                status: "modified".to_string(),
                hunks: Vec::new(),
                additions: 0,
                deletions: 0,
                truncated: true,
            }]);
        }

        Ok(parse_unified_diff(&output))
    }

    /// Diff for a specific file (workdir vs index).
    pub fn diff_file(&self, path: &str) -> Result<FileDiff, GitError> {
        let repo = self.inner();
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path);
        let diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;
        let diffs = collect_file_diffs(&diff)?;
        diffs
            .into_iter()
            .find(|d| d.path == path)
            .ok_or_else(|| GitError::Git(git2::Error::from_str(&format!("No changes for {path}"))))
    }
}

fn collect_file_diffs(diff: &Diff) -> Result<Vec<FileDiff>, GitError> {
    let mut files: Vec<FileDiff> = Vec::new();

    let num_deltas = diff.deltas().len();
    for delta_idx in 0..num_deltas {
        let delta = diff.get_delta(delta_idx).unwrap();
        let path = delta
            .new_file()
            .path()
            .unwrap_or(Path::new(""))
            .to_string_lossy()
            .to_string();
        let old_path = delta
            .old_file()
            .path()
            .map(|p| p.to_string_lossy().to_string());
        let status = delta_status_str(delta.status()).to_string();

        files.push(FileDiff {
            path,
            old_path,
            status,
            hunks: Vec::new(),
            additions: 0,
            deletions: 0,
            truncated: false,
        });
    }

    // Build an O(1) lookup map from file path to index in `files`.
    let file_index: HashMap<String, usize> = files
        .iter()
        .enumerate()
        .map(|(i, f)| (f.path.clone(), i))
        .collect();

    // Second pass: collect hunks and lines using print callback
    diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
        let file_path = delta
            .new_file()
            .path()
            .unwrap_or(Path::new(""))
            .to_string_lossy()
            .to_string();

        let file_idx = file_index.get(&file_path).copied();
        if let Some(idx) = file_idx {
            if let Some(h) = hunk {
                let header = String::from_utf8_lossy(h.header()).trim().to_string();

                let file = &mut files[idx];
                if file
                    .hunks
                    .last()
                    .map(|lh| lh.header != header)
                    .unwrap_or(true)
                {
                    file.hunks.push(DiffHunkInfo {
                        header,
                        old_start: h.old_start(),
                        old_lines: h.old_lines(),
                        new_start: h.new_start(),
                        new_lines: h.new_lines(),
                        lines: Vec::new(),
                    });
                }
            }

            let origin = line.origin();
            let content = String::from_utf8_lossy(line.content()).to_string();
            let file = &mut files[idx];

            match origin {
                '+' => file.additions += 1,
                '-' => file.deletions += 1,
                _ => {}
            }

            if matches!(origin, '+' | '-' | ' ')
                && let Some(hunk) = file.hunks.last_mut()
            {
                hunk.lines.push(DiffLineInfo {
                    origin,
                    content,
                    old_lineno: line.old_lineno(),
                    new_lineno: line.new_lineno(),
                });
            }
        }
        true
    })?;

    Ok(files)
}

/// Parse a unified diff text (like `git diff` output) into structured [`FileDiff`] objects.
///
/// This is used for cases where we have raw diff text (e.g. `git stash show -p`)
/// rather than a `libgit2` [`Diff`] handle.
pub fn parse_unified_diff(diff_text: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut current_hunk: Option<usize> = None; // index into files.last().hunks
    // Running line-number counters for the current hunk — reset each time a new
    // hunk header is encountered, then incremented O(1) per line instead of
    // re-iterating all previously seen lines (which was O(n²) per hunk).
    let mut old_line: u32 = 1;
    let mut new_line: u32 = 1;

    for line in diff_text.lines() {
        if line.starts_with("diff --git ") {
            // New file diff — push a placeholder, paths come from ---/+++ lines
            files.push(FileDiff {
                path: String::new(),
                old_path: None,
                status: "modified".to_string(),
                hunks: Vec::new(),
                additions: 0,
                deletions: 0,
                truncated: false,
            });
            current_hunk = None;
        } else if line.starts_with("--- ") {
            if let Some(file) = files.last_mut() {
                let path = line.trim_start_matches("--- ").trim_start_matches("a/");
                if path != "/dev/null" {
                    file.old_path = Some(path.to_string());
                } else {
                    file.status = "added".to_string();
                }
            }
        } else if line.starts_with("+++ ") {
            if let Some(file) = files.last_mut() {
                let path = line.trim_start_matches("+++ ").trim_start_matches("b/");
                if path == "/dev/null" {
                    file.status = "deleted".to_string();
                } else {
                    file.path = path.to_string();
                }
            }
        } else if line.starts_with("@@ ") {
            // Parse hunk header: @@ -old_start,old_lines +new_start,new_lines @@
            if let Some(file) = files.last_mut() {
                let (old_start, old_lines, new_start, new_lines) = parse_hunk_header(line);
                // Reset running counters to the hunk's starting line numbers.
                old_line = old_start;
                new_line = new_start;
                file.hunks.push(DiffHunkInfo {
                    header: line.to_string(),
                    old_start,
                    old_lines,
                    new_start,
                    new_lines,
                    lines: Vec::new(),
                });
                current_hunk = Some(file.hunks.len() - 1);
            }
        } else if let Some(hunk_idx) = current_hunk
            && let Some(file) = files.last_mut()
        {
            let (origin, content) = if let Some(rest) = line.strip_prefix('+') {
                ('+', rest.to_string())
            } else if let Some(rest) = line.strip_prefix('-') {
                ('-', rest.to_string())
            } else if let Some(rest) = line.strip_prefix(' ') {
                (' ', rest.to_string())
            } else {
                // Skip non-diff lines (e.g. "\ No newline at end of file")
                continue;
            };

            let (old_lineno, new_lineno) = compute_line_numbers(old_line, new_line, origin);

            // Advance the running counters for the next line.
            match origin {
                '-' => old_line += 1,
                '+' => new_line += 1,
                _ => {
                    old_line += 1;
                    new_line += 1;
                }
            }

            match origin {
                '+' => file.additions += 1,
                '-' => file.deletions += 1,
                _ => {}
            }

            file.hunks[hunk_idx].lines.push(DiffLineInfo {
                origin,
                content,
                old_lineno,
                new_lineno,
            });
        }
    }

    files
}

/// Parse `@@ -old_start,old_lines +new_start,new_lines @@` into numeric values.
fn parse_hunk_header(header: &str) -> (u32, u32, u32, u32) {
    // Format: @@ -10,5 +10,7 @@ optional context
    let stripped = header.trim_start_matches("@@ ");
    let at_end = stripped.find(" @@").unwrap_or(stripped.len());
    let range_part = &stripped[..at_end];

    let parts: Vec<&str> = range_part.split(' ').collect();
    let (old_start, old_lines) = parse_range(parts.first().unwrap_or(&"-1,1"));
    let (new_start, new_lines) = parse_range(parts.get(1).unwrap_or(&"+1,1"));

    (old_start, old_lines, new_start, new_lines)
}

/// Parse a range like "-10,5" or "+10,7" into (start, lines).
fn parse_range(s: &str) -> (u32, u32) {
    let s = s.trim_start_matches(['-', '+'].as_ref());
    if let Some((start, lines)) = s.split_once(',') {
        (start.parse().unwrap_or(1), lines.parse().unwrap_or(1))
    } else {
        (s.parse().unwrap_or(1), 1)
    }
}

/// Compute old/new line numbers for a diff line from running counters.
///
/// The caller maintains `old_line` and `new_line` as running counters
/// (reset to hunk start on each new hunk header, incremented after each call),
/// making the overall diff parsing O(n) rather than O(n²) per hunk.
fn compute_line_numbers(old_line: u32, new_line: u32, origin: char) -> (Option<u32>, Option<u32>) {
    match origin {
        '+' => (None, Some(new_line)),
        '-' => (Some(old_line), None),
        _ => (Some(old_line), Some(new_line)),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_repo_with_file() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();

        // Create and commit a file
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
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
    fn test_diff_workdir_no_changes() {
        let (_dir, repo) = create_repo_with_file();
        let diffs = repo.diff_workdir().unwrap();
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_diff_workdir_modified_file() {
        let (dir, repo) = create_repo_with_file();
        fs::write(dir.path().join("test.txt"), "line 1\nmodified\nline 3\n").unwrap();
        let diffs = repo.diff_workdir().unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "test.txt");
        assert_eq!(diffs[0].status, "modified");
        assert!(diffs[0].additions > 0 || diffs[0].deletions > 0);
    }

    #[test]
    fn test_diff_workdir_has_hunks() {
        let (dir, repo) = create_repo_with_file();
        fs::write(dir.path().join("test.txt"), "line 1\nmodified\nline 3\n").unwrap();
        let diffs = repo.diff_workdir().unwrap();
        assert!(!diffs[0].hunks.is_empty());
        assert!(!diffs[0].hunks[0].lines.is_empty());
    }

    #[test]
    fn test_diff_index_staged_changes() {
        let (dir, repo) = create_repo_with_file();
        // Modify and stage
        fs::write(dir.path().join("test.txt"), "staged change\n").unwrap();
        let git_repo = repo.inner();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let diffs = repo.diff_index().unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].status, "modified");
    }

    #[test]
    fn test_diff_file_specific() {
        let (dir, repo) = create_repo_with_file();
        fs::write(dir.path().join("test.txt"), "changed\n").unwrap();
        let diff = repo.diff_file("test.txt").unwrap();
        assert_eq!(diff.path, "test.txt");
    }

    #[test]
    fn test_diff_commits_between_two() {
        let (dir, repo) = create_repo_with_file();

        // Capture the first commit OID via the underlying git2 repo
        let first_oid = repo.inner().head().unwrap().target().unwrap().to_string();

        // Create a second commit that adds a new file, using raw git2 so we
        // stay consistent with how create_repo_with_file() works above.
        let file_path = dir.path().join("new.txt");
        fs::write(&file_path, "new\n").unwrap();
        let git_repo = repo.inner();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = git_repo.find_tree(tree_id).unwrap();
        let parent = git_repo
            .find_commit(git2::Oid::from_str(&first_oid).unwrap())
            .unwrap();
        git_repo
            .commit(Some("HEAD"), &sig, &sig, "Add new.txt", &tree, &[&parent])
            .unwrap();

        let second_oid = git_repo.head().unwrap().target().unwrap().to_string();

        let files = repo.diff_commits(&first_oid, &second_oid).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "new.txt");
        assert_eq!(files[0].status, "added");
    }

    #[test]
    fn test_diff_workdir_untracked_file_has_content() {
        let (dir, repo) = create_repo_with_file();
        fs::write(dir.path().join("new_file.txt"), "new content\n").unwrap();
        let diffs = repo.diff_workdir().unwrap();
        let new = diffs.iter().find(|d| d.path == "new_file.txt").unwrap();
        assert_eq!(new.status, "untracked");
        assert!(
            !new.hunks.is_empty(),
            "untracked files must have diff content"
        );
        assert!(new.additions > 0);
    }

    #[test]
    fn test_commit_files_initial_commit() {
        let (_dir, repo) = create_repo_with_file();
        let oid = repo.inner().head().unwrap().target().unwrap().to_string();
        let files = repo.commit_files(&oid).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "test.txt");
        assert_eq!(files[0].status, "added");
    }

    #[test]
    fn test_commit_files_modified() {
        let (dir, repo) = create_repo_with_file();
        let first_oid = repo.inner().head().unwrap().target().unwrap().to_string();

        // Modify test.txt and create a second commit
        fs::write(dir.path().join("test.txt"), "line 1\nchanged\nline 3\n").unwrap();
        let git_repo = repo.inner();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = git_repo.find_tree(tree_id).unwrap();
        let parent = git_repo
            .find_commit(git2::Oid::from_str(&first_oid).unwrap())
            .unwrap();
        git_repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Modify test.txt",
                &tree,
                &[&parent],
            )
            .unwrap();

        let second_oid = git_repo.head().unwrap().target().unwrap().to_string();
        let files = repo.commit_files(&second_oid).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "test.txt");
        assert_eq!(files[0].status, "modified");
    }

    #[test]
    fn test_commit_files_added_new_file() {
        let (dir, repo) = create_repo_with_file();
        let first_oid = repo.inner().head().unwrap().target().unwrap().to_string();

        // Add new.txt and create a second commit
        fs::write(dir.path().join("new.txt"), "new content\n").unwrap();
        let git_repo = repo.inner();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = git_repo.find_tree(tree_id).unwrap();
        let parent = git_repo
            .find_commit(git2::Oid::from_str(&first_oid).unwrap())
            .unwrap();
        git_repo
            .commit(Some("HEAD"), &sig, &sig, "Add new.txt", &tree, &[&parent])
            .unwrap();

        let second_oid = git_repo.head().unwrap().target().unwrap().to_string();
        let files = repo.commit_files(&second_oid).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "new.txt");
        assert_eq!(files[0].status, "added");
    }

    #[test]
    fn test_parse_unified_diff_basic() {
        let diff_text = "\
diff --git a/file.txt b/file.txt
--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,3 @@
 line 1
-line 2
+modified line 2
 line 3";

        let files = parse_unified_diff(diff_text);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "file.txt");
        assert_eq!(files[0].status, "modified");
        assert_eq!(files[0].hunks.len(), 1);
        assert_eq!(files[0].additions, 1);
        assert_eq!(files[0].deletions, 1);

        let hunk = &files[0].hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_lines, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_lines, 3);

        // Check line numbers: context line 1, deleted line 2, added line 2, context line 3
        let lines = &hunk.lines;
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].origin, ' ');
        assert_eq!(lines[0].old_lineno, Some(1));
        assert_eq!(lines[0].new_lineno, Some(1));
        assert_eq!(lines[1].origin, '-');
        assert_eq!(lines[1].old_lineno, Some(2));
        assert_eq!(lines[1].new_lineno, None);
        assert_eq!(lines[2].origin, '+');
        assert_eq!(lines[2].old_lineno, None);
        assert_eq!(lines[2].new_lineno, Some(2));
        assert_eq!(lines[3].origin, ' ');
        assert_eq!(lines[3].old_lineno, Some(3));
        assert_eq!(lines[3].new_lineno, Some(3));
    }

    #[test]
    fn test_parse_unified_diff_new_file() {
        let diff_text = "\
diff --git a/new.txt b/new.txt
--- /dev/null
+++ b/new.txt
@@ -0,0 +1,2 @@
+hello
+world";

        let files = parse_unified_diff(diff_text);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "new.txt");
        assert_eq!(files[0].status, "added");
        assert_eq!(files[0].additions, 2);
        assert_eq!(files[0].deletions, 0);
    }

    #[test]
    fn test_parse_unified_diff_deleted_file() {
        let diff_text = "\
diff --git a/old.txt b/old.txt
--- a/old.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-goodbye
-world";

        let files = parse_unified_diff(diff_text);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, "deleted");
        // old_path should be set from the --- line
        assert_eq!(files[0].old_path.as_deref(), Some("old.txt"));
        assert_eq!(files[0].additions, 0);
        assert_eq!(files[0].deletions, 2);
    }

    #[test]
    fn test_parse_unified_diff_multiple_files() {
        let diff_text = "\
diff --git a/a.txt b/a.txt
--- a/a.txt
+++ b/a.txt
@@ -1 +1 @@
-old a
+new a
diff --git a/b.txt b/b.txt
--- /dev/null
+++ b/b.txt
@@ -0,0 +1 @@
+new b";

        let files = parse_unified_diff(diff_text);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "a.txt");
        assert_eq!(files[0].status, "modified");
        assert_eq!(files[1].path, "b.txt");
        assert_eq!(files[1].status, "added");
    }

    #[test]
    fn test_parse_unified_diff_empty() {
        let files = parse_unified_diff("");
        assert!(files.is_empty());
    }
}
