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

/// Which direction the patch we are about to build will be applied in.
///
/// This is the axis a partial (line-level) selection turns on, because it
/// decides which side of the patch has to match the target: forward, the old
/// side must match; in reverse, the *new* side must.
///
/// **Not** index-vs-worktree, which is the tempting way to model it and is
/// wrong: `unstage_hunks` targets the index and still applies in reverse, so a
/// `{ Index, Worktree }` enum picks the forward polarity for it and unstaging a
/// single line stays broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApplyDirection {
    /// `git apply [--cached]` — staging.
    Forward,
    /// `git apply --reverse [--cached]` — discarding and unstaging.
    Reverse,
}

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
    /// Re-derive the diff the caller was looking at.
    ///
    /// `context_lines` has to match what produced the selection, because a
    /// [`HunkSelection`] is *positional*: hunk 2, lines 5–7 of the array the
    /// UI rendered. Change the context and libgit2 cuts the file into
    /// different hunks with different line arrays, so the same indices name
    /// different lines — and the patch applies cleanly to the wrong ones.
    ///
    /// This used to call `diff_workdir()` / `diff_index()`, which are fixed
    /// at libgit2's default of 3. That agreed with the UI right up until the
    /// UI could ask for the whole file as one hunk, at which point staging a
    /// line under "show whole file" staged a different line and said it had
    /// succeeded.
    fn diff_for_selection(
        &self,
        path: &str,
        staged: bool,
        context_lines: Option<u32>,
    ) -> Result<FileDiff, GitError> {
        self.diff_single_file(path, staged, context_lines)?
            .ok_or_else(|| {
                GitError::Git(git2::Error::from_str(&format!(
                    "No diff found for file: {path}"
                )))
            })
    }

    /// Stage selected hunks/lines from the working directory.
    ///
    /// `context_lines` must be the value the displayed diff was fetched
    /// with — see [`Repository::diff_for_selection`].
    pub fn stage_hunks(
        &self,
        path: &str,
        selections: &[HunkSelection],
        context_lines: Option<u32>,
    ) -> Result<(), GitError> {
        let file_diff = self.diff_for_selection(path, false, context_lines)?;
        let patch = build_patch(path, &file_diff, selections, ApplyDirection::Forward)?;
        self.apply_patch(&patch, &["--cached"])
    }

    /// Unstage selected hunks/lines from the index. See
    /// [`Repository::stage_hunks`] for `context_lines`.
    pub fn unstage_hunks(
        &self,
        path: &str,
        selections: &[HunkSelection],
        context_lines: Option<u32>,
    ) -> Result<(), GitError> {
        let file_diff = self.diff_for_selection(path, true, context_lines)?;
        let patch = build_patch(path, &file_diff, selections, ApplyDirection::Reverse)?;
        self.apply_patch(&patch, &["--cached", "--reverse"])
    }

    /// Discard selected hunks/lines from the working directory. See
    /// [`Repository::stage_hunks`] for `context_lines`.
    pub fn discard_hunks(
        &self,
        path: &str,
        selections: &[HunkSelection],
        context_lines: Option<u32>,
    ) -> Result<(), GitError> {
        let file_diff = self.diff_for_selection(path, false, context_lines)?;
        let patch = build_patch(path, &file_diff, selections, ApplyDirection::Reverse)?;
        self.apply_patch(&patch, &["--reverse"])
    }

    /// Apply a unified diff patch via `git apply`.
    fn apply_patch(&self, patch_content: &str, extra_args: &[&str]) -> Result<(), GitError> {
        let mut tmp = tempfile::NamedTempFile::new()?;
        tmp.write_all(patch_content.as_bytes())?;
        tmp.flush()?;

        // Refuse to substitute an empty path (which would make `git apply`
        // silently read from stdin) when the temp path is non-UTF8.
        let tmp_path = tmp.path().to_str().ok_or_else(|| {
            GitError::Io(std::io::Error::other("temp patch path is not valid UTF-8"))
        })?;
        let mut args = vec!["apply"];
        args.extend(extra_args);
        args.push("--unidiff-zero");
        args.push(tmp_path);

        let result = self.git_cmd(&args)?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(format!(
                "git apply failed: {}",
                result.stderr
            )))
        }
    }
}

/// Append one diff line to the patch, preserving the no-trailing-newline state.
///
/// libgit2 yields the final line of a file that lacks an EOF newline with no
/// `\n` in its content (and reports the `\ No newline at end of file` info via
/// a separate origin that `collect_file_diffs` drops). For such a line we must
/// re-emit that marker so `git apply` knows the line genuinely has no trailing
/// newline. Fabricating a bare `\n` instead produces a patch that does not
/// apply — and, when it does, silently adds a newline that was never there.
fn push_patch_line(patch: &mut String, line: &DiffLineInfo) {
    patch.push(line.origin);
    patch.push_str(&line.content);
    if !line.content.ends_with('\n') {
        patch.push('\n');
        patch.push_str("\\ No newline at end of file\n");
    }
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
    direction: ApplyDirection,
) -> Result<String, GitError> {
    let mut patch = String::new();

    // File header
    let old_path = diff.old_path.as_deref().unwrap_or(path);
    patch.push_str(&format!("--- a/{old_path}\n"));
    patch.push_str(&format!("+++ b/{path}\n"));

    for sel in selections {
        if sel.hunk_index >= diff.hunks.len() {
            return Err(GitError::InvalidArgument(format!(
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
                    push_patch_line(&mut patch, line);
                }
            }
            Some(ranges) => {
                // Partial line selection within the hunk.
                let filtered = filter_hunk_lines(hunk, ranges, direction);
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
                    push_patch_line(&mut patch, line);
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
/// A non-selected changed line has to be rewritten so that the side of the
/// patch which must match the target still describes the target exactly. Which
/// side that is depends on the direction — see [`ApplyDirection`]:
///
/// | non-selected line | `Forward` | `Reverse` |
/// |---|---|---|
/// | `+` | omitted (not in the target yet) | context (already in the target) |
/// | `-` | context (still in the target) | omitted (already gone from the target) |
///
/// Context lines are always kept, in both directions.
///
/// Getting this backwards does not corrupt anything: the generated patch
/// describes a state the target is not in, `git apply` rejects it, and the
/// caller gets an error with the target untouched. It does make the operation
/// impossible, which is what it did for both reverse paths until this
/// parameter existed.
fn filter_hunk_lines(
    hunk: &DiffHunkInfo,
    ranges: &[(usize, usize)],
    direction: ApplyDirection,
) -> Vec<DiffLineInfo> {
    let mut result = Vec::new();

    for (i, line) in hunk.lines.iter().enumerate() {
        let is_selected = ranges.iter().any(|(start, end)| i >= *start && i <= *end);

        // Context lines are always included; selected changed lines are kept
        // verbatim, since they are the edit being applied or reverted.
        if line.origin == ' ' || is_selected {
            if matches!(line.origin, ' ' | '+' | '-') {
                result.push(line.clone());
            }
            continue;
        }

        match (line.origin, direction) {
            // Not in the target yet, so it cannot be described at all.
            ('+', ApplyDirection::Forward) => {}
            // Already in the target: carry it as context. Both line numbers
            // are set from `new_lineno` because that is the only one an
            // addition has — the opposite-side number is synthetic, and the
            // `-` arm below is synthetic in the mirror way. Nothing reads
            // them: `push_patch_line` emits only `origin` and `content`, and
            // the header counts come from counting origins. If a caller ever
            // starts trusting these numbers, both arms need revisiting.
            ('+', ApplyDirection::Reverse) => result.push(DiffLineInfo {
                origin: ' ',
                content: line.content.clone(),
                old_lineno: line.new_lineno,
                new_lineno: line.new_lineno,
            }),
            // Still in the target: carry it as context.
            ('-', ApplyDirection::Forward) => result.push(DiffLineInfo {
                origin: ' ',
                content: line.content.clone(),
                old_lineno: line.old_lineno,
                new_lineno: line.old_lineno,
            }),
            // Already gone from the target, so it cannot be described.
            ('-', ApplyDirection::Reverse) => {}
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
            truncated: false,
            binary: false,
        }
    }

    #[test]
    fn test_build_patch_full_hunk() {
        let diff = make_file_diff();
        let selections = vec![HunkSelection {
            hunk_index: 0,
            line_ranges: None,
        }];

        let patch = build_patch("test.txt", &diff, &selections, ApplyDirection::Forward).unwrap();

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

        let patch = build_patch("test.txt", &diff, &selections, ApplyDirection::Forward).unwrap();

        // The deletion at index 1 is not selected, so it becomes a context line.
        // old_count = 3 (context line1 + context-from-delete line2 + context line3)
        // new_count = 4 (context line1 + context-from-delete line2 + add + context line3)
        //
        // Assert the header, not just the bodies. The counts are the only part
        // of the hunk header `build_patch` *derives* rather than copies, and
        // they were previously spelled out in this comment and checked
        // nowhere — leaving `git apply`'s own consistency check as the only
        // thing standing between a miscount and a silent wrong patch.
        assert!(
            patch.contains("@@ -1,3 +1,4 @@\n"),
            "recomputed counts must match the filtered lines:\n{patch}"
        );
        assert!(patch.contains("+modified line 2\n"));
        // The non-selected deletion should become a context line.
        assert!(patch.contains(" line 2\n"));
    }

    /// The reverse polarity at unit level. It is covered end to end by the
    /// discard and unstage tests, but those go through `git apply`, so a
    /// failure there does not say whether the patch or the application was
    /// wrong. This pins the patch side on its own.
    #[test]
    fn test_filter_hunk_lines_reverse_inverts_both_polarities() {
        let diff = make_file_diff();
        let hunk = &diff.hunks[0];

        // Select the addition (index 2); the deletion at index 1 is not
        // selected. Reverse is the mirror of `Forward`: the unselected
        // deletion is already gone from the target so it is dropped, where
        // Forward would have kept it as context.
        let filtered = filter_hunk_lines(hunk, &[(2, 2)], ApplyDirection::Reverse);
        let shape: Vec<String> = filtered
            .iter()
            .map(|l| format!("{}{}", l.origin, l.content.trim()))
            .collect();
        assert_eq!(
            shape,
            [" line 1", "+modified line 2", " line 3"],
            "reverse must drop the unselected deletion, not turn it into context"
        );

        // And the other way round: select the deletion, leave the addition
        // unselected. Reverse keeps it as context because it *is* in the
        // target, where Forward would have omitted it.
        let filtered = filter_hunk_lines(hunk, &[(1, 1)], ApplyDirection::Reverse);
        let shape: Vec<String> = filtered
            .iter()
            .map(|l| format!("{}{}", l.origin, l.content.trim()))
            .collect();
        assert_eq!(
            shape,
            [" line 1", "-line 2", " modified line 2", " line 3"],
            "reverse must keep the unselected addition as context, not omit it"
        );
    }

    #[test]
    fn test_filter_hunk_lines_selected_add() {
        let diff = make_file_diff();
        let hunk = &diff.hunks[0];

        // Select the added line (index 2).
        let filtered = filter_hunk_lines(hunk, &[(2, 2)], ApplyDirection::Forward);

        let add_lines: Vec<_> = filtered.iter().filter(|l| l.origin == '+').collect();
        assert_eq!(add_lines.len(), 1);
        assert_eq!(add_lines[0].content, "modified line 2\n");
    }

    #[test]
    fn test_filter_hunk_lines_unselected_add() {
        let diff = make_file_diff();
        let hunk = &diff.hunks[0];

        // Select only the deletion (index 1), not the addition (index 2).
        let filtered = filter_hunk_lines(hunk, &[(1, 1)], ApplyDirection::Forward);

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
        let filtered = filter_hunk_lines(hunk, &[(2, 2)], ApplyDirection::Forward);

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

        let result = build_patch("test.txt", &diff, &selections, ApplyDirection::Forward);
        assert!(result.is_err());
    }

    /// **The bug: a selection is positional, so both sides have to agree on
    /// how the file was cut into hunks.**
    ///
    /// The UI can ask for the whole file as one hunk ("show whole file").
    /// Staging then re-derived its own diff at libgit2's default context of
    /// 3, which cuts the same file into several hunks with different line
    /// arrays — so "line 15 of hunk 0" named a different line on each side.
    /// The patch applied cleanly to the wrong one and reported success.
    #[test]
    fn test_stage_hunks_honours_the_context_the_selection_was_made_with() {
        let (dir, repo) = create_repo_with_file();

        // A file long enough that two distant edits are separate hunks at
        // the default context but one hunk when the whole file is asked for.
        let original: String = (1..=60).map(|i| format!("line {i}\n")).collect();
        let long = dir.path().join("long.txt");
        fs::write(&long, &original).unwrap();
        let git_repo = git2::Repository::open(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("long.txt")).unwrap();
        index.write().unwrap();
        let tree = git_repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = git_repo.head().unwrap().peel_to_commit().unwrap();
        git_repo
            .commit(Some("HEAD"), &sig, &sig, "add long", &tree, &[&parent])
            .unwrap();

        let edited = original
            .replace("line 10\n", "CHANGED 10\n")
            .replace("line 50\n", "CHANGED 50\n");
        fs::write(&long, &edited).unwrap();

        let expanded = repo
            .diff_single_file("long.txt", false, Some(crate::diff::FULL_FILE_CONTEXT))
            .unwrap()
            .expect("the edits must produce a diff");
        assert_eq!(
            expanded.hunks.len(),
            1,
            "full context must collapse the file into one hunk, or this proves nothing"
        );
        let default_ctx = repo
            .diff_single_file("long.txt", false, None)
            .unwrap()
            .unwrap();
        assert_eq!(
            default_ctx.hunks.len(),
            2,
            "the default context must split it, or the two sides cannot disagree"
        );

        // Select the whole (single) hunk exactly as the expanded UI would.
        repo.stage_hunks(
            "long.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: None,
            }],
            Some(crate::diff::FULL_FILE_CONTEXT),
        )
        .expect("staging a full-context selection must apply");

        // Both edits staged, and nothing left unstaged.
        let staged = repo
            .diff_single_file("long.txt", true, None)
            .unwrap()
            .expect("the staged side must carry the change");
        let staged_adds: Vec<&str> = staged
            .hunks
            .iter()
            .flat_map(|h| h.lines.iter())
            .filter(|l| l.origin == '+')
            .map(|l| l.content.trim())
            .collect();
        assert_eq!(
            staged_adds,
            ["CHANGED 10", "CHANGED 50"],
            "the lines the user selected are the lines that must be staged"
        );
        assert!(
            repo.diff_single_file("long.txt", false, None)
                .unwrap()
                .is_none(),
            "nothing may be left behind in the working tree"
        );
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
        repo.stage_hunks("test.txt", &selections, None).unwrap();

        // *Which* lines landed, not merely that something did. "The index is
        // non-empty" is true whether one hunk or both were staged, so it
        // passes a backend that re-cuts the file into a single whole-file
        // hunk and stages everything — the same context mismatch as
        // `test_stage_hunks_honours_the_context_the_selection_was_made_with`,
        // in mirror image and on the common path rather than the expanded one.
        let staged = repo
            .diff_single_file("test.txt", true, None)
            .unwrap()
            .expect("the index must carry the first hunk");
        let staged_adds: Vec<&str> = staged
            .hunks
            .iter()
            .flat_map(|h| h.lines.iter())
            .filter(|l| l.origin == '+')
            .map(|l| l.content.trim())
            .collect();
        assert_eq!(
            staged_adds,
            ["CHANGED TOP"],
            "only the selected hunk may be staged"
        );

        let left = repo
            .diff_single_file("test.txt", false, None)
            .unwrap()
            .expect("the second hunk must still be unstaged");
        assert!(
            left.hunks
                .iter()
                .flat_map(|h| h.lines.iter())
                .any(|l| l.content.trim() == "CHANGED BOTTOM"),
            "the hunk the user did not select must be left in the working tree"
        );
    }

    /// The wrong-data half of the same bug: a *partial* selection.
    ///
    /// `line_ranges` indexes into the hunk's line array, so a backend that
    /// cut the file differently resolves "line 15 of hunk 0" to some other
    /// line and stages that instead — cleanly, and reporting success.
    #[test]
    fn test_stage_hunks_partial_selection_stages_the_selected_line() {
        let (dir, repo) = create_repo_with_file();

        let original: String = (1..=60).map(|i| format!("line {i}\n")).collect();
        let long = dir.path().join("long.txt");
        fs::write(&long, &original).unwrap();
        let git_repo = git2::Repository::open(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("long.txt")).unwrap();
        index.write().unwrap();
        let tree = git_repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = git_repo.head().unwrap().peel_to_commit().unwrap();
        git_repo
            .commit(Some("HEAD"), &sig, &sig, "add long", &tree, &[&parent])
            .unwrap();

        let edited = original
            .replace("line 10\n", "line 10\nADDED A\n")
            .replace("line 50\n", "line 50\nADDED B\n");
        fs::write(&long, &edited).unwrap();

        // Pick "ADDED B" out of the expanded single-hunk view, the way the UI
        // does: by its index in that hunk's line array.
        let expanded = repo
            .diff_single_file("long.txt", false, Some(crate::diff::FULL_FILE_CONTEXT))
            .unwrap()
            .unwrap();
        assert_eq!(expanded.hunks.len(), 1);
        let idx = expanded.hunks[0]
            .lines
            .iter()
            .position(|l| l.content.trim() == "ADDED B")
            .expect("the added line must be in the expanded view");

        repo.stage_hunks(
            "long.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: Some(vec![(idx, idx)]),
            }],
            Some(crate::diff::FULL_FILE_CONTEXT),
        )
        .expect("staging one line of a full-context selection must apply");

        let staged = repo
            .diff_single_file("long.txt", true, None)
            .unwrap()
            .expect("something must be staged");
        let staged_adds: Vec<&str> = staged
            .hunks
            .iter()
            .flat_map(|h| h.lines.iter())
            .filter(|l| l.origin == '+')
            .map(|l| l.content.trim())
            .collect();
        assert_eq!(
            staged_adds,
            ["ADDED B"],
            "the line the user ticked is the line that must be staged"
        );
    }

    /// A diff line whose content lacks a trailing newline (the final line of a
    /// file with no EOF newline) must emit the literal `\ No newline at end of
    /// file` trailer, not a fabricated `\n`.
    #[test]
    fn test_build_patch_emits_no_newline_marker() {
        let diff = FileDiff {
            path: "f.txt".to_string(),
            old_path: None,
            status: "modified".to_string(),
            hunks: vec![DiffHunkInfo {
                header: "@@ -1,2 +1,2 @@".to_string(),
                old_start: 1,
                old_lines: 2,
                new_start: 1,
                new_lines: 2,
                lines: vec![
                    DiffLineInfo {
                        origin: ' ',
                        content: "line1\n".to_string(),
                        old_lineno: Some(1),
                        new_lineno: Some(1),
                    },
                    // No trailing newline → the EOF-newline marker is required.
                    DiffLineInfo {
                        origin: '-',
                        content: "line2".to_string(),
                        old_lineno: Some(2),
                        new_lineno: None,
                    },
                    DiffLineInfo {
                        origin: '+',
                        content: "CHANGED".to_string(),
                        old_lineno: None,
                        new_lineno: Some(2),
                    },
                ],
            }],
            additions: 1,
            deletions: 1,
            truncated: false,
            binary: false,
        };
        let selections = vec![HunkSelection {
            hunk_index: 0,
            line_ranges: None,
        }];
        let patch = build_patch("f.txt", &diff, &selections, ApplyDirection::Forward).unwrap();

        // Both changed lines lack a newline, so each is followed by the marker.
        assert_eq!(
            patch.matches("\\ No newline at end of file\n").count(),
            2,
            "expected one no-newline marker per changed last line:\n{patch}"
        );
        // The marker must follow the line it annotates.
        assert!(patch.contains("-line2\n\\ No newline at end of file\n"));
        assert!(patch.contains("+CHANGED\n\\ No newline at end of file\n"));
    }

    // -----------------------------------------------------------------------
    // Line-level polarity
    //
    // `filter_hunk_lines` decides what happens to a *non-selected* changed
    // line, and the right answer depends on whether the patch will be applied
    // forward or in reverse. The four tests immediately below are
    // **characterisation** tests of the forward path, which was always
    // correct. They are here so that inverting the wrong sign shows up
    // immediately: any fix must leave them green.
    //
    // Fixtures use one line per letter so a whole file fits in an assertion,
    // and a fresh repo per case — see `repo_committed_then_worktree`.
    // -----------------------------------------------------------------------

    /// Commit `committed` as `f.txt`, then leave `worktree` in the working
    /// tree. The index matches the commit.
    ///
    /// A fresh repo per case, deliberately. `discard_hunks` diffs the working
    /// tree against the **index**, so a `stage_hunks` earlier in the same repo
    /// moves the index and silently changes what the next assertion is
    /// measuring. Reusing one repo across cases already produced one false
    /// diagnosis — that discarding a whole hunk corrupted data, which it does
    /// not.
    fn repo_committed_then_worktree(
        committed: &str,
        worktree: &str,
    ) -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();

        fs::write(dir.path().join("f.txt"), committed).unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("f.txt")).unwrap();
        index.write().unwrap();
        let tree = git_repo.find_tree(index.write_tree().unwrap()).unwrap();
        git_repo
            .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        fs::write(dir.path().join("f.txt"), worktree).unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        (dir, repo)
    }

    /// Commit `committed`, then stage `staged`, so the change lives in the
    /// index and the working tree agrees with it. This is the state
    /// `unstage_hunks` operates on.
    fn repo_committed_then_staged(
        committed: &str,
        staged: &str,
    ) -> (tempfile::TempDir, Repository) {
        let (dir, repo) = repo_committed_then_worktree(committed, staged);
        let git_repo = git2::Repository::open(dir.path()).unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("f.txt")).unwrap();
        index.write().unwrap();
        (dir, repo)
    }

    /// Read `f.txt` out of the index, reopening the repo to do it.
    ///
    /// **The reopen is the point.** These operations shell out to `git apply
    /// --cached`, which writes the index from another process, while the
    /// `git2::Repository` held by our `Repository` keeps its own in-memory
    /// index snapshot and goes on answering with the pre-apply content. A test
    /// that reads the index through the same handle it staged with measures
    /// nothing at all — it reports the committed content whether the staging
    /// worked, staged the wrong lines, or never happened.
    fn index_content(dir: &Path) -> String {
        Repository::open(dir)
            .unwrap()
            .get_file_index("f.txt")
            .unwrap()
    }

    /// Position of the line carrying this origin and content within hunk 0.
    ///
    /// Selections are positional, so the tests must look the index up rather
    /// than hardcode it — a hardcoded index silently names a different line if
    /// libgit2 ever cuts the hunk differently.
    fn line_at(diff: &FileDiff, origin: char, content: &str) -> usize {
        diff.hunks[0]
            .lines
            .iter()
            .position(|l| l.origin == origin && l.content.trim() == content)
            .unwrap_or_else(|| {
                panic!(
                    "no `{origin}{content}` line in hunk 0; lines were {:?}",
                    diff.hunks[0]
                        .lines
                        .iter()
                        .map(|l| format!("{}{}", l.origin, l.content.trim()))
                        .collect::<Vec<_>>()
                )
            })
    }

    /// The mixed hunk the whole polarity story is told with: two deletions and
    /// two additions in one hunk.
    ///
    /// ```text
    /// [0] ' ' a
    /// [1] '-' b
    /// [2] '-' c
    /// [3] '+' X
    /// [4] '+' Y
    /// [5] ' ' d
    /// [6] ' ' e
    /// ```
    const MIXED_COMMITTED: &str = "a\nb\nc\nd\ne\n";
    const MIXED_WORKTREE: &str = "a\nX\nY\nd\ne\n";

    #[test]
    fn staging_one_added_line_stages_exactly_that_line() {
        let (dir, repo) = repo_committed_then_worktree(MIXED_COMMITTED, MIXED_WORKTREE);

        let diff = repo
            .diff_single_file("f.txt", false, None)
            .unwrap()
            .expect("the edit must produce a diff");
        let x = line_at(&diff, '+', "X");

        repo.stage_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: Some(vec![(x, x)]),
            }],
            None,
        )
        .expect("staging a single added line already worked and must keep working");

        // Only the addition is staged: the two deletions were not selected, so
        // b and c survive in the index.
        assert_eq!(
            index_content(dir.path()),
            "a\nb\nc\nX\nd\ne\n",
            "staging +X may not carry the unselected deletions with it"
        );
    }

    #[test]
    fn staging_one_deleted_line_stages_exactly_that_line() {
        let (dir, repo) = repo_committed_then_worktree(MIXED_COMMITTED, MIXED_WORKTREE);

        let diff = repo
            .diff_single_file("f.txt", false, None)
            .unwrap()
            .unwrap();
        let b = line_at(&diff, '-', "b");

        repo.stage_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: Some(vec![(b, b)]),
            }],
            None,
        )
        .expect("staging a single deleted line already worked and must keep working");

        // Only b goes. c stays, and neither addition is staged.
        assert_eq!(
            index_content(dir.path()),
            "a\nc\nd\ne\n",
            "staging -b may not carry the unselected additions with it"
        );
    }

    #[test]
    fn staging_a_whole_mixed_hunk_stages_all_of_it() {
        let (dir, repo) = repo_committed_then_worktree(MIXED_COMMITTED, MIXED_WORKTREE);

        repo.stage_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: None,
            }],
            None,
        )
        .expect("staging a whole hunk must apply");

        assert_eq!(index_content(dir.path()), MIXED_WORKTREE);
    }

    #[test]
    fn discarding_a_whole_mixed_hunk_restores_the_committed_content() {
        let (dir, repo) = repo_committed_then_worktree(MIXED_COMMITTED, MIXED_WORKTREE);

        repo.discard_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: None,
            }],
            None,
        )
        .expect("discarding a whole hunk must apply");

        assert_eq!(
            fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            MIXED_COMMITTED,
            "discarding the whole hunk must restore the committed content"
        );
    }

    // -----------------------------------------------------------------------
    // The bug: the two reverse-applied paths
    //
    // `filter_hunk_lines` was written for a forward patch, where a
    // non-selected `+` is not yet in the target (omit it) and a non-selected
    // `-` still is (keep it as context). Both `discard_hunks` (`--reverse`)
    // and `unstage_hunks` (`--cached --reverse`) apply in reverse, where the
    // polarity is the other way round: the non-selected `+` *is* in the target
    // and the non-selected `-` is not. The patch then describes a state the
    // target is not in, and `git apply` rejects it.
    //
    // It fails safely — nothing is written and the caller sees an error — but
    // the action is unusable.
    //
    // The last two cases use hunks of a single sign. They matter because they
    // prove each polarity is wrong *on its own*, not through interaction: a
    // fix that only inverts the handling of `-` still fails on an
    // additions-only hunk.
    // -----------------------------------------------------------------------

    #[test]
    fn discarding_one_added_line_reverts_only_that_line() {
        let (dir, repo) = repo_committed_then_worktree(MIXED_COMMITTED, MIXED_WORKTREE);

        let diff = repo
            .diff_single_file("f.txt", false, None)
            .unwrap()
            .unwrap();
        let x = line_at(&diff, '+', "X");

        repo.discard_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: Some(vec![(x, x)]),
            }],
            None,
        )
        .expect("discarding a single added line must apply");

        // X was added and is being discarded, so it goes. Y was also added but
        // was not selected, so it stays. b and c were deleted and not
        // selected, so they stay deleted.
        assert_eq!(
            fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "a\nY\nd\ne\n",
            "discarding +X must remove X and leave everything else alone"
        );
    }

    #[test]
    fn discarding_one_deleted_line_restores_only_that_line() {
        let (dir, repo) = repo_committed_then_worktree(MIXED_COMMITTED, MIXED_WORKTREE);

        let diff = repo
            .diff_single_file("f.txt", false, None)
            .unwrap()
            .unwrap();
        let b = line_at(&diff, '-', "b");

        repo.discard_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: Some(vec![(b, b)]),
            }],
            None,
        )
        .expect("discarding a single deleted line must apply");

        // b comes back. c stays deleted, and both additions survive.
        assert_eq!(
            fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "a\nb\nX\nY\nd\ne\n",
            "discarding -b must restore b and leave everything else alone"
        );
    }

    #[test]
    fn unstaging_one_added_line_unstages_only_that_line() {
        let (dir, repo) = repo_committed_then_staged(MIXED_COMMITTED, MIXED_WORKTREE);

        let diff = repo
            .diff_single_file("f.txt", true, None)
            .unwrap()
            .expect("the staged change must produce a diff");
        let x = line_at(&diff, '+', "X");

        repo.unstage_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: Some(vec![(x, x)]),
            }],
            None,
        )
        .expect("unstaging a single added line must apply");

        // Same shape as discarding +X, against the index instead of the tree.
        assert_eq!(
            index_content(dir.path()),
            "a\nY\nd\ne\n",
            "unstaging +X must remove X from the index and leave the rest staged"
        );
    }

    #[test]
    fn unstaging_one_deleted_line_restores_only_that_line() {
        let (dir, repo) = repo_committed_then_staged(MIXED_COMMITTED, MIXED_WORKTREE);

        let diff = repo.diff_single_file("f.txt", true, None).unwrap().unwrap();
        let b = line_at(&diff, '-', "b");

        repo.unstage_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: Some(vec![(b, b)]),
            }],
            None,
        )
        .expect("unstaging a single deleted line must apply");

        assert_eq!(
            index_content(dir.path()),
            "a\nb\nX\nY\nd\ne\n",
            "unstaging -b must put b back in the index and leave the rest staged"
        );
    }

    /// An additions-only hunk: no `-` line anywhere, so only the `+` polarity
    /// can be at fault. A fix that inverts just the deletion branch still
    /// fails here.
    #[test]
    fn discarding_one_line_of_an_additions_only_hunk_reverts_only_that_line() {
        let (dir, repo) = repo_committed_then_worktree("a\nb\nc\n", "a\nX\nY\nb\nc\n");

        let diff = repo
            .diff_single_file("f.txt", false, None)
            .unwrap()
            .unwrap();
        assert!(
            diff.hunks[0].lines.iter().all(|l| l.origin != '-'),
            "this fixture must produce an additions-only hunk, or it proves nothing"
        );
        let x = line_at(&diff, '+', "X");

        repo.discard_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: Some(vec![(x, x)]),
            }],
            None,
        )
        .expect("discarding one line of an additions-only hunk must apply");

        assert_eq!(
            fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "a\nY\nb\nc\n",
            "only X may be reverted; the unselected addition Y must survive"
        );
    }

    /// A deletions-only hunk: the mirror of the case above, isolating the `-`
    /// polarity.
    #[test]
    fn discarding_one_line_of_a_deletions_only_hunk_restores_only_that_line() {
        let (dir, repo) = repo_committed_then_worktree("a\nb\nc\nd\n", "a\nd\n");

        let diff = repo
            .diff_single_file("f.txt", false, None)
            .unwrap()
            .unwrap();
        assert!(
            diff.hunks[0].lines.iter().all(|l| l.origin != '+'),
            "this fixture must produce a deletions-only hunk, or it proves nothing"
        );
        let b = line_at(&diff, '-', "b");

        repo.discard_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: Some(vec![(b, b)]),
            }],
            None,
        )
        .expect("discarding one line of a deletions-only hunk must apply");

        assert_eq!(
            fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "a\nb\nd\n",
            "only b may come back; the unselected deletion c must stay deleted"
        );
    }

    /// Where the two known hazards in this file cross: a partial selection in
    /// reverse *and* a file with no EOF newline.
    ///
    /// Each half was handled separately — `push_patch_line` re-emits the
    /// `\ No newline at end of file` marker, and the polarity fix above makes
    /// reverse selections describe the target correctly — but the combination
    /// could not be reached before, because the selection failed on polarity
    /// long before the marker mattered. It is reachable now.
    ///
    /// The subtlety: the unselected `+Y` becomes a context line, and it is the
    /// final line of a file with no trailing newline, so the marker has to
    /// survive that rewrite. Dropping it silently appends a newline that was
    /// never in the file.
    #[test]
    fn discarding_one_added_line_keeps_a_missing_eof_newline_missing() {
        let (dir, repo) = repo_committed_then_worktree("a\nb\nc", "a\nX\nY");

        let diff = repo
            .diff_single_file("f.txt", false, None)
            .unwrap()
            .unwrap();
        let x = line_at(&diff, '+', "X");

        repo.discard_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: Some(vec![(x, x)]),
            }],
            None,
        )
        .expect("discarding one line of a file with no EOF newline must apply");

        let after = fs::read_to_string(dir.path().join("f.txt")).unwrap();
        assert_eq!(
            after, "a\nY",
            "X reverted, Y kept, and the missing EOF newline still missing"
        );
        assert!(
            !after.ends_with('\n'),
            "a newline the file never had must not be fabricated"
        );
    }

    /// The mirror: discard the *last* line of a no-EOF-newline file, so the
    /// selected line is itself the one carrying the marker.
    ///
    /// Note what the trailing newline does here, because it is easy to assert
    /// the wrong thing. The working tree is `a\nX\nY` with no final newline —
    /// which means `X` **does** end in a newline, since Y follows it. Reverting
    /// only Y therefore yields `a\nX\n`: a file that now ends in a newline
    /// where the previous state did not.
    ///
    /// That is correct rather than a leak. The patch's old side describes X
    /// exactly as the file holds it, and the only way to produce `a\nX` instead
    /// would be to also rewrite X's line ending — editing a line the user did
    /// not select. `git apply --reverse` behaves the same way on an
    /// equivalent hand-written patch.
    #[test]
    fn discarding_the_last_added_line_of_a_file_without_eof_newline() {
        let (dir, repo) = repo_committed_then_worktree("a\nb\nc", "a\nX\nY");

        let diff = repo
            .diff_single_file("f.txt", false, None)
            .unwrap()
            .unwrap();
        let y = line_at(&diff, '+', "Y");

        repo.discard_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: Some(vec![(y, y)]),
            }],
            None,
        )
        .expect("discarding the final line of a no-EOF-newline file must apply");

        assert_eq!(
            fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "a\nX\n",
            "only Y may be reverted; X keeps the newline it already had"
        );
    }

    /// End-to-end regression: staging the last hunk of a file that has no
    /// trailing EOF newline must succeed. Before the fix, `build_patch`
    /// fabricated a `\n` and `git apply --cached` rejected the patch.
    #[test]
    fn test_stage_hunk_no_eof_newline_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();

        // Commit a file WITHOUT a trailing newline at EOF.
        fs::write(dir.path().join("f.txt"), "line1\nline2\nline3").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("f.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = git_repo.find_tree(tree_id).unwrap();
        git_repo
            .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        let repo = Repository::open(dir.path()).unwrap();

        // Edit the last line, still with no trailing newline.
        fs::write(dir.path().join("f.txt"), "line1\nline2\nCHANGED").unwrap();

        let diffs = repo.diff_workdir().unwrap();
        let file_diff = diffs.iter().find(|d| d.path == "f.txt").unwrap();
        assert!(!file_diff.hunks.is_empty());

        // Staging the whole hunk must not error (the patch must apply cleanly).
        repo.stage_hunks(
            "f.txt",
            &[HunkSelection {
                hunk_index: 0,
                line_ranges: None,
            }],
            None,
        )
        .expect("staging the last hunk of a no-EOF-newline file should succeed");

        let index_diffs = repo.diff_index().unwrap();
        assert!(
            index_diffs.iter().any(|d| d.path == "f.txt"),
            "the change should now be staged"
        );
    }
}
