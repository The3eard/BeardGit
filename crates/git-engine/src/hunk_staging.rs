//! Hunk and line-level staging, unstaging, and discard.
//!
//! Extends [`Repository`] with methods that build unified diff patches from
//! user-selected hunks or individual lines and apply them via `git apply`.
//! This enables partial staging instead of whole-file operations.

use serde::Deserialize;
use std::io::Write;

use crate::diff::{DiffHunkInfo, DiffLineInfo, FileDiff};
use crate::error::GitError;
use crate::repository::Repository;

/// Describes which hunks/lines the user selected for staging/unstaging.
#[derive(Debug, Clone, Deserialize)]
pub struct HunkSelection {
    /// Index into the `FileDiff.hunks` array.
    pub hunk_index: usize,
    /// If `None`, the entire hunk is selected.
    /// If `Some`, only lines within these ranges (inclusive, 0-based within
    /// the hunk's `lines` array).
    pub line_ranges: Option<Vec<(usize, usize)>>,
}

impl Repository {
    /// Stage selected hunks/lines from the working directory.
    pub fn stage_hunks(&self, path: &str, selections: &[HunkSelection]) -> Result<(), GitError> {
        let diffs = self.diff_workdir()?;
        let file_diff = find_file_diff(&diffs, path)?;
        let patch = build_patch(path, file_diff, selections)?;
        self.apply_patch(&patch, &["--cached"])
    }

    /// Unstage selected hunks/lines from the index.
    pub fn unstage_hunks(&self, path: &str, selections: &[HunkSelection]) -> Result<(), GitError> {
        let diffs = self.diff_index()?;
        let file_diff = find_file_diff(&diffs, path)?;
        let patch = build_patch(path, file_diff, selections)?;
        self.apply_patch(&patch, &["--cached", "--reverse"])
    }

    /// Discard selected hunks/lines from the working directory.
    pub fn discard_hunks(&self, path: &str, selections: &[HunkSelection]) -> Result<(), GitError> {
        let diffs = self.diff_workdir()?;
        let file_diff = find_file_diff(&diffs, path)?;
        let patch = build_patch(path, file_diff, selections)?;
        self.apply_patch(&patch, &["--reverse"])
    }

    /// Apply a unified diff patch via `git apply`.
    fn apply_patch(&self, patch_content: &str, extra_args: &[&str]) -> Result<(), GitError> {
        let mut tmp = tempfile::NamedTempFile::new()?;
        tmp.write_all(patch_content.as_bytes())?;
        tmp.flush()?;

        let tmp_path = tmp.path().to_str().unwrap_or("");
        let mut args = vec!["apply"];
        args.extend(extra_args);
        args.push("--unidiff-zero");
        args.push(tmp_path);

        let result = self.git_cmd(&args)?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::RepoNotFound(format!(
                "git apply failed: {}",
                result.stderr
            )))
        }
    }
}

/// Find the [`FileDiff`] for a specific path within a list of diffs.
fn find_file_diff<'a>(diffs: &'a [FileDiff], path: &str) -> Result<&'a FileDiff, GitError> {
    diffs
        .iter()
        .find(|d| d.path == path)
        .ok_or_else(|| GitError::RepoNotFound(format!("No diff found for: {path}")))
}

/// Build a valid unified diff patch from selected hunks/lines.
///
/// The generated patch follows the standard unified diff format:
/// ```text
/// --- a/<path>
/// +++ b/<path>
/// @@ -old_start,old_count +new_start,new_count @@
///  context line
/// +added line
/// -removed line
/// ```
fn build_patch(
    path: &str,
    diff: &FileDiff,
    selections: &[HunkSelection],
) -> Result<String, GitError> {
    let mut patch = String::new();

    // File header
    let old_path = diff.old_path.as_deref().unwrap_or(path);
    patch.push_str(&format!("--- a/{old_path}\n"));
    patch.push_str(&format!("+++ b/{path}\n"));

    for sel in selections {
        if sel.hunk_index >= diff.hunks.len() {
            return Err(GitError::RepoNotFound(format!(
                "Hunk index {} out of bounds ({})",
                sel.hunk_index,
                diff.hunks.len()
            )));
        }
        let hunk = &diff.hunks[sel.hunk_index];

        match &sel.line_ranges {
            None => {
                // Entire hunk selected — emit as-is.
                patch.push_str(&format_hunk_header(hunk));
                for line in &hunk.lines {
                    patch.push(line.origin);
                    patch.push_str(&line.content);
                    if !line.content.ends_with('\n') {
                        patch.push('\n');
                    }
                }
            }
            Some(ranges) => {
                // Partial line selection within the hunk.
                let filtered = filter_hunk_lines(hunk, ranges);
                if filtered.is_empty() {
                    continue;
                }

                // Recalculate hunk header counts from the filtered lines.
                let old_count = filtered
                    .iter()
                    .filter(|l| l.origin == ' ' || l.origin == '-')
                    .count();
                let new_count = filtered
                    .iter()
                    .filter(|l| l.origin == ' ' || l.origin == '+')
                    .count();

                patch.push_str(&format!(
                    "@@ -{},{} +{},{} @@\n",
                    hunk.old_start, old_count, hunk.new_start, new_count
                ));

                for line in &filtered {
                    patch.push(line.origin);
                    patch.push_str(&line.content);
                    if !line.content.ends_with('\n') {
                        patch.push('\n');
                    }
                }
            }
        }
    }

    Ok(patch)
}

/// Format a hunk header from [`DiffHunkInfo`] fields.
fn format_hunk_header(hunk: &DiffHunkInfo) -> String {
    format!(
        "@@ -{},{} +{},{} @@\n",
        hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines
    )
}

/// Filter hunk lines to include only selected changed lines plus all context.
///
/// Non-selected additions are omitted entirely.
/// Non-selected deletions become context lines (preserving the old content).
fn filter_hunk_lines(hunk: &DiffHunkInfo, ranges: &[(usize, usize)]) -> Vec<DiffLineInfo> {
    let mut result = Vec::new();

    for (i, line) in hunk.lines.iter().enumerate() {
        let is_selected = ranges.iter().any(|(start, end)| i >= *start && i <= *end);

        match line.origin {
            ' ' => {
                // Context lines are always included.
                result.push(line.clone());
            }
            // Selected additions are kept; non-selected additions are omitted.
            '+' if is_selected => {
                result.push(line.clone());
            }
            '-' => {
                if is_selected {
                    result.push(line.clone());
                } else {
                    // Non-selected deletions become context lines.
                    result.push(DiffLineInfo {
                        origin: ' ',
                        content: line.content.clone(),
                        old_lineno: line.old_lineno,
                        new_lineno: line.old_lineno,
                    });
                }
            }
            _ => {}
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffHunkInfo, DiffLineInfo, FileDiff};
    use std::fs;
    use std::path::Path;

    /// Helper: build a simple `FileDiff` with one hunk for testing.
    fn make_file_diff() -> FileDiff {
        FileDiff {
            path: "test.txt".to_string(),
            old_path: None,
            status: "modified".to_string(),
            hunks: vec![DiffHunkInfo {
                header: "@@ -1,3 +1,3 @@".to_string(),
                old_start: 1,
                old_lines: 3,
                new_start: 1,
                new_lines: 3,
                lines: vec![
                    DiffLineInfo {
                        origin: ' ',
                        content: "line 1\n".to_string(),
                        old_lineno: Some(1),
                        new_lineno: Some(1),
                    },
                    DiffLineInfo {
                        origin: '-',
                        content: "line 2\n".to_string(),
                        old_lineno: Some(2),
                        new_lineno: None,
                    },
                    DiffLineInfo {
                        origin: '+',
                        content: "modified line 2\n".to_string(),
                        old_lineno: None,
                        new_lineno: Some(2),
                    },
                    DiffLineInfo {
                        origin: ' ',
                        content: "line 3\n".to_string(),
                        old_lineno: Some(3),
                        new_lineno: Some(3),
                    },
                ],
            }],
            additions: 1,
            deletions: 1,
        }
    }

    #[test]
    fn test_build_patch_full_hunk() {
        let diff = make_file_diff();
        let selections = vec![HunkSelection {
            hunk_index: 0,
            line_ranges: None,
        }];

        let patch = build_patch("test.txt", &diff, &selections).unwrap();

        assert!(patch.starts_with("--- a/test.txt\n+++ b/test.txt\n"));
        assert!(patch.contains("@@ -1,3 +1,3 @@\n"));
        assert!(patch.contains(" line 1\n"));
        assert!(patch.contains("-line 2\n"));
        assert!(patch.contains("+modified line 2\n"));
        assert!(patch.contains(" line 3\n"));
    }

    #[test]
    fn test_build_patch_selected_lines() {
        let diff = make_file_diff();
        // Select only the addition (index 2), not the deletion (index 1).
        let selections = vec![HunkSelection {
            hunk_index: 0,
            line_ranges: Some(vec![(2, 2)]),
        }];

        let patch = build_patch("test.txt", &diff, &selections).unwrap();

        // The deletion at index 1 is not selected, so it becomes a context line.
        // old_count = 3 (context line1 + context-from-delete line2 + context line3)
        // new_count = 4 (context line1 + context-from-delete line2 + add + context line3)
        assert!(patch.contains("+modified line 2\n"));
        // The non-selected deletion should become a context line.
        assert!(patch.contains(" line 2\n"));
    }

    #[test]
    fn test_filter_hunk_lines_selected_add() {
        let diff = make_file_diff();
        let hunk = &diff.hunks[0];

        // Select the added line (index 2).
        let filtered = filter_hunk_lines(hunk, &[(2, 2)]);

        let add_lines: Vec<_> = filtered.iter().filter(|l| l.origin == '+').collect();
        assert_eq!(add_lines.len(), 1);
        assert_eq!(add_lines[0].content, "modified line 2\n");
    }

    #[test]
    fn test_filter_hunk_lines_unselected_add() {
        let diff = make_file_diff();
        let hunk = &diff.hunks[0];

        // Select only the deletion (index 1), not the addition (index 2).
        let filtered = filter_hunk_lines(hunk, &[(1, 1)]);

        let add_lines: Vec<_> = filtered.iter().filter(|l| l.origin == '+').collect();
        assert!(
            add_lines.is_empty(),
            "unselected additions should be omitted"
        );
    }

    #[test]
    fn test_filter_hunk_lines_unselected_delete() {
        let diff = make_file_diff();
        let hunk = &diff.hunks[0];

        // Select only the addition (index 2), not the deletion (index 1).
        let filtered = filter_hunk_lines(hunk, &[(2, 2)]);

        // The deletion should have become a context line.
        let del_lines: Vec<_> = filtered.iter().filter(|l| l.origin == '-').collect();
        assert!(
            del_lines.is_empty(),
            "unselected deletion should become context"
        );

        let ctx_lines: Vec<_> = filtered
            .iter()
            .filter(|l| l.origin == ' ' && l.content == "line 2\n")
            .collect();
        assert_eq!(
            ctx_lines.len(),
            1,
            "unselected deletion should appear as context"
        );
    }

    #[test]
    fn test_build_patch_hunk_index_out_of_bounds() {
        let diff = make_file_diff();
        let selections = vec![HunkSelection {
            hunk_index: 5,
            line_ranges: None,
        }];

        let result = build_patch("test.txt", &diff, &selections);
        assert!(result.is_err());
    }

    /// Helper to create a repo with an initial committed file.
    fn create_repo_with_file() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();

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
    fn test_stage_hunk_roundtrip() {
        let (dir, repo) = create_repo_with_file();

        // Modify two separate sections to get two hunks.
        // With only 3 original lines, a single edit produces one hunk.
        // Instead, create a larger file with two distinct changed regions.
        let original = (1..=20).map(|i| format!("line {i}\n")).collect::<String>();
        fs::write(dir.path().join("test.txt"), &original).unwrap();

        // Re-stage and commit the larger file.
        {
            let git_repo = repo.inner();
            let sig = git2::Signature::now("Test", "test@test.com").unwrap();
            let mut idx = git_repo.index().unwrap();
            idx.add_path(Path::new("test.txt")).unwrap();
            idx.write().unwrap();
            let tree_id = idx.write_tree().unwrap();
            let tree = git_repo.find_tree(tree_id).unwrap();
            let parent = git_repo.head().unwrap().peel_to_commit().unwrap();
            git_repo
                .commit(Some("HEAD"), &sig, &sig, "Expand file", &tree, &[&parent])
                .unwrap();
        }

        // Now modify line 2 (near top) and line 19 (near bottom).
        let mut lines: Vec<String> = (1..=20).map(|i| format!("line {i}")).collect();
        lines[1] = "CHANGED TOP".to_string();
        lines[18] = "CHANGED BOTTOM".to_string();
        let modified = lines.join("\n") + "\n";
        fs::write(dir.path().join("test.txt"), &modified).unwrap();

        // Get workdir diff — should have at least one hunk.
        let diffs = repo.diff_workdir().unwrap();
        assert!(!diffs.is_empty(), "should have workdir changes");
        let file_diff = diffs.iter().find(|d| d.path == "test.txt").unwrap();
        assert!(!file_diff.hunks.is_empty(), "should have at least one hunk");

        // Stage only the first hunk.
        let selections = vec![HunkSelection {
            hunk_index: 0,
            line_ranges: None,
        }];
        repo.stage_hunks("test.txt", &selections).unwrap();

        // After staging the first hunk, the index should have staged changes.
        let index_diffs = repo.diff_index().unwrap();
        assert!(
            !index_diffs.is_empty(),
            "index should have staged changes from hunk"
        );
    }
}
