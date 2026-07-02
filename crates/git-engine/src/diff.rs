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
    /// commit touched a 50 MB minified blob, or the file blew the per-file
    /// byte/line budget in [`collect_file_diffs`]). The frontend renders a
    /// "diff too large to display" placeholder. Defaults to `false` so
    /// existing call sites and JSON payloads stay untouched.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
    /// `true` when the file is binary (libgit2 flagged the delta binary, or
    /// a NUL byte appeared in the first content chunk). Hunks are left empty
    /// and the frontend renders a "binary file — no preview" placeholder.
    /// Skipped from JSON when `false` so existing payloads stay untouched.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub binary: bool,
}

/// Maximum size, in bytes, of the raw `git diff` output rendered in
/// `commit_file_diff`. Also the per-file byte budget in
/// [`collect_file_diffs`]. Beyond this, hunk collection stops and the
/// file is marked `truncated`. 5 MB is enough for any honest commit while
/// keeping IPC + frontend rendering responsive.
pub const MAX_COMMIT_DIFF_BYTES: usize = 5 * 1024 * 1024;

/// Maximum number of diff lines collected for a single file in
/// [`collect_file_diffs`]. A generated file can stay under the byte cap
/// while still emitting hundreds of thousands of short lines (each one a
/// `String` + IPC row), so the line count is capped independently. Beyond
/// this, hunk collection stops and the file is marked `truncated`.
pub const MAX_FILE_DIFF_LINES: usize = 10_000;

/// Whole-response byte budget for the `FileDiff[]` list endpoints
/// (`get_diff_workdir` / `get_diff_index`). Once the accumulated line
/// content across files exceeds this, the remaining files come back with
/// empty hunks and `truncated: true` — see [`enforce_response_budget`].
/// 20 MB keeps a pathological working tree (many large changed files)
/// from ballooning a single IPC payload.
pub const MAX_DIFF_RESPONSE_BYTES: usize = 20 * 1024 * 1024;

/// Lightweight per-file change summary: name/status + line counts, no
/// hunks. Powers the Changes list without paying the cost of collecting
/// (and serializing) every hunk of every changed file on each mutation.
#[derive(Debug, Clone, Serialize)]
pub struct FileDiffStat {
    /// Repo-relative path of the file (new path for renames).
    pub path: String,
    /// Previous path of the file when it was renamed, otherwise `None`.
    pub old_path: Option<String>,
    /// Change type, as in [`FileDiff::status`].
    pub status: String,
    /// Number of added lines (0 for binary files).
    pub additions: usize,
    /// Number of deleted lines (0 for binary files).
    pub deletions: usize,
    /// `true` when the file is binary — line counts are 0 and the diff
    /// pane renders a placeholder when opened. Skipped from JSON when
    /// `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub binary: bool,
}

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
    ///
    /// Rename/copy detection is intentionally NOT enabled here (nor in
    /// [`Repository::diff_index`]): the hunk-staging patch builder
    /// (`hunk_staging::build_patch`) models a rename as a separate
    /// delete+add and does not emit `rename from`/`rename to` headers, so a
    /// detected-rename delta would produce a patch that `git apply` rejects.
    /// Callers that need rename status use `file_status_all` instead.
    pub fn diff_workdir(&self) -> Result<Vec<FileDiff>, GitError> {
        let repo = self.inner();
        // `recurse_untracked_dirs` matters: without it libgit2 collapses an
        // untracked directory into a single content-less `dir/` delta, so a
        // brand-new file inside a brand-new folder rendered as an empty diff
        // while `file_status_all` (which does recurse) still listed the file.
        let diff = repo.diff_index_to_workdir(
            None,
            Some(
                DiffOptions::new()
                    .include_untracked(true)
                    .recurse_untracked_dirs(true)
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
    ///
    /// Both endpoints are resolved with `revparse_single`, so branch names,
    /// tags, `HEAD`, and abbreviated/full SHAs are all accepted — not just raw
    /// OIDs (the compare view passes ref names and a merge-base OID here).
    pub fn diff_commits(
        &self,
        from_oid: &str,
        to_oid: &str,
    ) -> Result<Vec<CommitFileChange>, GitError> {
        let repo = self.inner();
        let from_commit = repo.revparse_single(from_oid)?.peel_to_commit()?;
        let to_commit = repo.revparse_single(to_oid)?.peel_to_commit()?;
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
        // Decide up-front whether this is a root commit (no parent) via
        // libgit2, then run the matching command. The previous code treated
        // ANY failure of `<oid>^..<oid>` as "must be a root commit" and fell
        // through to `diff-tree --root`, masking real failures (bad oid,
        // corrupt repo, transient git error) as an empty diff.
        let is_root = {
            let repo = self.inner();
            let commit = repo.find_commit(git2::Oid::from_str(oid)?)?;
            commit.parent_count() == 0
        };

        let result = if is_root {
            self.git_cmd(&["diff-tree", "-p", "--root", oid, "--", path])?
        } else {
            self.git_cmd(&[
                "diff",
                "--no-ext-diff",
                &format!("{oid}^..{oid}"),
                "--",
                path,
            ])?
        };

        if !result.success {
            return Err(GitError::CliError(result.stderr));
        }
        let output = result.stdout;

        if output.len() > MAX_COMMIT_DIFF_BYTES {
            return Ok(vec![FileDiff {
                path: path.to_string(),
                old_path: None,
                status: "modified".to_string(),
                hunks: Vec::new(),
                additions: 0,
                deletions: 0,
                truncated: true,
                binary: false,
            }]);
        }

        Ok(parse_unified_diff(&output))
    }

    /// Return the structured diff for **every** file touched by a commit in
    /// a single libgit2 walk — no per-file shell-out.
    ///
    /// `commit_file_diff(oid, path)` shells out to `git diff <oid>^..<oid>
    /// -- <path>` once per file. Detail panes that render N files used to
    /// dispatch N subprocess (each ~30–80 ms on macOS), giving 30+ files a
    /// ~1.5 s wall-clock cost. This variant runs a single
    /// `diff_tree_to_tree` and returns the path-keyed result, dropping that
    /// to ~80 ms regardless of file count.
    ///
    /// Root commits are diffed against an empty tree.
    pub fn commit_full_diff(
        &self,
        oid: &str,
    ) -> Result<std::collections::HashMap<String, FileDiff>, GitError> {
        let repo = self.inner();
        let oid = git2::Oid::from_str(oid)?;
        let commit = repo.find_commit(oid)?;
        let new_tree = commit.tree()?;
        let old_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let diff = repo.diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), None)?;
        let files = collect_file_diffs(&diff)?;
        Ok(files.into_iter().map(|f| (f.path.clone(), f)).collect())
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

    /// Full hunks/lines diff for a single file, on demand.
    ///
    /// `staged == false` diffs the working directory against the index
    /// (including untracked content, matching [`Repository::diff_workdir`]);
    /// `staged == true` diffs the index against HEAD. Returns `None` when the
    /// path has no change on that side. This is the lazy per-file path the
    /// Changes view calls when the user opens a file, so the whole
    /// `FileDiff[]` set is never materialized on every mutation. The
    /// returned diff carries the same per-file byte/line/binary caps as
    /// [`collect_file_diffs`].
    pub fn diff_single_file(&self, path: &str, staged: bool) -> Result<Option<FileDiff>, GitError> {
        let repo = self.inner();
        let mut opts = DiffOptions::new();
        opts.pathspec(path);
        let diff = if staged {
            let head_tree = repo.head()?.peel_to_tree()?;
            repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?
        } else {
            opts.include_untracked(true)
                .recurse_untracked_dirs(true)
                .show_untracked_content(true);
            repo.diff_index_to_workdir(None, Some(&mut opts))?
        };
        let diffs = collect_file_diffs(&diff)?;
        Ok(diffs.into_iter().find(|d| d.path == path))
    }

    /// Cheap per-file change stats (name/status + add/del counts) for the
    /// working directory, without materializing hunks or line content.
    ///
    /// Feeds the Changes list so a mutation refresh no longer streams every
    /// hunk of every changed file over IPC. Uses libgit2's per-delta line
    /// stats and only allocates the path strings.
    pub fn diff_stats_workdir(&self) -> Result<Vec<FileDiffStat>, GitError> {
        let repo = self.inner();
        let diff = repo.diff_index_to_workdir(
            None,
            Some(
                DiffOptions::new()
                    .include_untracked(true)
                    .recurse_untracked_dirs(true)
                    .show_untracked_content(true),
            ),
        )?;
        collect_file_stats(&diff)
    }

    /// Cheap per-file change stats for the index (staged changes) vs HEAD.
    /// See [`Repository::diff_stats_workdir`].
    pub fn diff_stats_index(&self) -> Result<Vec<FileDiffStat>, GitError> {
        let repo = self.inner();
        let head_tree = repo.head()?.peel_to_tree()?;
        let diff = repo.diff_tree_to_index(Some(&head_tree), None, None)?;
        collect_file_stats(&diff)
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
            binary: false,
        });
    }

    // Build an O(1) lookup map from file path to index in `files`.
    let file_index: HashMap<String, usize> = files
        .iter()
        .enumerate()
        .map(|(i, f)| (f.path.clone(), i))
        .collect();

    // Per-file running budgets, indexed parallel to `files`. Content lines
    // increment both; once either cap is breached the file is marked
    // `truncated` and no further lines are collected for it (see below).
    let mut file_bytes = vec![0usize; files.len()];
    let mut file_lines = vec![0usize; files.len()];

    // Second pass: collect hunks and lines using print callback.
    //
    // The callback fires once per diff line; previously each fire paid for a
    // full `String::from_utf8_lossy(...).to_string()` of the file path just
    // to look up the same `file_index` entry the previous line already
    // resolved. We cache the most-recent (file_oid, idx) pair so consecutive
    // lines on the same delta hit the cache and skip the path alloc — that
    // pattern accounts for ~all of the lines emitted by `print`.
    let last_file_lookup: std::cell::Cell<Option<(git2::Oid, usize)>> = std::cell::Cell::new(None);
    diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
        let new_id = delta.new_file().id();
        let file_idx = match last_file_lookup.get() {
            Some((cached_id, idx)) if cached_id == new_id => Some(idx),
            _ => {
                let file_path = delta
                    .new_file()
                    .path()
                    .unwrap_or(Path::new(""))
                    .to_string_lossy()
                    .to_string();
                let idx = file_index.get(&file_path).copied();
                if let Some(idx) = idx {
                    last_file_lookup.set(Some((new_id, idx)));
                }
                idx
            }
        };
        if let Some(idx) = file_idx {
            // Binary short-circuit: once flagged, ignore the rest of the
            // callbacks for this file — no content is collected.
            if files[idx].binary {
                return true;
            }
            // libgit2 flags binary deltas (and emits a `Binary` line marker
            // instead of content). Detect either and drop the file's hunks.
            if delta.flags().contains(git2::DiffFlags::BINARY)
                || matches!(line.origin_value(), git2::DiffLineType::Binary)
            {
                let file = &mut files[idx];
                file.binary = true;
                file.hunks.clear();
                file.additions = 0;
                file.deletions = 0;
                return true;
            }
            // Once the per-file budget is breached, stop collecting content
            // but keep the truncated flag so the UI shows a placeholder.
            if files[idx].truncated {
                return true;
            }

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
            let content_bytes = line.content();

            // NUL-check the first content chunk of the file (mirrors the
            // sniff in `file_content.rs`) as a fallback for content libgit2
            // did not itself flag binary.
            if file_lines[idx] == 0 && matches!(origin, '+' | '-' | ' ') {
                let sniff = &content_bytes[..content_bytes.len().min(8192)];
                if sniff.contains(&0u8) {
                    let file = &mut files[idx];
                    file.binary = true;
                    file.hunks.clear();
                    file.additions = 0;
                    file.deletions = 0;
                    return true;
                }
            }

            if matches!(origin, '+' | '-' | ' ') {
                // Enforce the per-file byte + line budget before pushing.
                if file_bytes[idx].saturating_add(content_bytes.len()) > MAX_COMMIT_DIFF_BYTES
                    || file_lines[idx] >= MAX_FILE_DIFF_LINES
                {
                    files[idx].truncated = true;
                    return true;
                }

                let content = String::from_utf8_lossy(content_bytes).to_string();
                let file = &mut files[idx];
                match origin {
                    '+' => file.additions += 1,
                    '-' => file.deletions += 1,
                    _ => {}
                }
                if let Some(hunk) = file.hunks.last_mut() {
                    hunk.lines.push(DiffLineInfo {
                        origin,
                        content,
                        old_lineno: line.old_lineno(),
                        new_lineno: line.new_lineno(),
                    });
                    file_bytes[idx] += content_bytes.len();
                    file_lines[idx] += 1;
                }
            }
        }
        true
    })?;

    Ok(files)
}

/// Enforce a whole-response byte budget across a list of file diffs.
///
/// Walks `files` in order accumulating the byte size of collected line
/// content. Once the running total exceeds `max_bytes`, every remaining
/// file (including the one that tipped the budget) has its hunks cleared
/// and `truncated` set — the frontend already renders a per-file
/// placeholder for truncated entries. Files already marked binary or
/// truncated contribute ~nothing and are left as-is.
///
/// Applied only at the `FileDiff[]` list endpoints
/// (`get_diff_workdir` / `get_diff_index`); the per-file caps in
/// [`collect_file_diffs`] already bound each individual file, so the
/// hunk-staging path (which re-derives a single file's diff) is
/// unaffected.
pub fn enforce_response_budget(files: &mut [FileDiff], max_bytes: usize) {
    let mut total = 0usize;
    let mut exceeded = false;
    for f in files.iter_mut() {
        if exceeded {
            f.hunks.clear();
            f.truncated = true;
            continue;
        }
        let file_bytes: usize = f
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .map(|l| l.content.len())
            .sum();
        total = total.saturating_add(file_bytes);
        if total > max_bytes {
            exceeded = true;
            f.hunks.clear();
            f.truncated = true;
        }
    }
}

/// Build lightweight per-file stats from a libgit2 diff without collecting
/// hunks or line content. Uses `Patch::line_stats` for add/del counts and
/// the (post-generation) binary flag; binary files report 0/0.
fn collect_file_stats(diff: &Diff) -> Result<Vec<FileDiffStat>, GitError> {
    let num_deltas = diff.deltas().len();
    let mut stats = Vec::with_capacity(num_deltas);
    for idx in 0..num_deltas {
        // `Patch::from_diff` loads the file content (populating the binary
        // flag) and lets us read line stats without retaining any hunk or
        // per-line strings — the whole point of the stats path.
        let patch = git2::Patch::from_diff(diff, idx)?;
        let delta = diff.get_delta(idx).unwrap();
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .unwrap_or(Path::new(""))
            .to_string_lossy()
            .to_string();
        let old_path = delta
            .old_file()
            .path()
            .map(|p| p.to_string_lossy().to_string());
        let status = delta_status_str(delta.status()).to_string();

        let (binary, additions, deletions) = match patch {
            Some(p) => {
                if p.delta().flags().contains(git2::DiffFlags::BINARY) {
                    (true, 0, 0)
                } else {
                    let (_ctx, add, del) = p.line_stats()?;
                    (false, add, del)
                }
            }
            // No textual patch (e.g. a pure mode change) — fall back to the
            // delta flag with zero counts.
            None => (delta.flags().contains(git2::DiffFlags::BINARY), 0, 0),
        };

        stats.push(FileDiffStat {
            path,
            old_path,
            status,
            additions,
            deletions,
            binary,
        });
    }
    Ok(stats)
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
                binary: false,
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
    fn test_diff_workdir_untracked_file_in_new_dir_has_content() {
        let (dir, repo) = create_repo_with_file();
        fs::create_dir(dir.path().join("newdir")).unwrap();
        fs::write(dir.path().join("newdir/inner.txt"), "inner content\n").unwrap();
        let diffs = repo.diff_workdir().unwrap();
        let new = diffs
            .iter()
            .find(|d| d.path == "newdir/inner.txt")
            .expect("untracked file inside an untracked dir must appear as its own entry");
        assert_eq!(new.status, "untracked");
        assert!(
            !new.hunks.is_empty(),
            "untracked files in new dirs must have diff content"
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

    /// A non-existent commit oid must surface an error rather than being
    /// silently swallowed into an empty diff by the root-commit fallback.
    #[test]
    fn test_commit_file_diff_bad_oid_errors() {
        let (_dir, repo) = create_repo_with_file();
        let result = repo.commit_file_diff("0000000000000000000000000000000000000000", "test.txt");
        assert!(
            result.is_err(),
            "a non-existent oid must error, not return an empty diff"
        );
    }

    /// An oversized file (more lines than `MAX_FILE_DIFF_LINES`) must be
    /// flagged `truncated` with line collection stopped at the cap, so a
    /// generated blob can't stream hundreds of thousands of rows over IPC.
    #[test]
    fn test_diff_workdir_truncates_oversized_file() {
        let (dir, repo) = create_repo_with_file();
        // A fresh untracked file with well over the line cap. `show_untracked_
        // content` means every line would otherwise become a `DiffLineInfo`.
        let big = "x\n".repeat(MAX_FILE_DIFF_LINES + 5_000);
        fs::write(dir.path().join("big.txt"), big).unwrap();

        let diffs = repo.diff_workdir().unwrap();
        let big = diffs
            .iter()
            .find(|d| d.path == "big.txt")
            .expect("oversized file must still appear in the diff list");
        assert!(big.truncated, "oversized file must be marked truncated");
        let collected: usize = big.hunks.iter().map(|h| h.lines.len()).sum();
        assert!(
            collected <= MAX_FILE_DIFF_LINES,
            "line collection must stop at the cap, got {collected}"
        );
    }

    /// A binary file (NUL bytes) must short-circuit before line collection:
    /// `binary` is set, hunks stay empty, and no add/del counts accrue.
    #[test]
    fn test_diff_workdir_binary_short_circuit() {
        let (dir, repo) = create_repo_with_file();
        // Bytes with an embedded NUL — libgit2 flags this binary (and our
        // NUL sniff is the belt-and-braces fallback).
        fs::write(
            dir.path().join("blob.bin"),
            [0x89, b'P', b'N', b'G', 0x00, 0x01, 0x02, 0x03],
        )
        .unwrap();

        let diffs = repo.diff_workdir().unwrap();
        let blob = diffs
            .iter()
            .find(|d| d.path == "blob.bin")
            .expect("binary file must still appear in the diff list");
        assert!(blob.binary, "binary file must be flagged");
        assert!(blob.hunks.is_empty(), "binary file must collect no hunks");
        assert_eq!(blob.additions, 0);
        assert_eq!(blob.deletions, 0);
    }

    /// The whole-response budget marks every file past the byte limit as
    /// `truncated` with hunks cleared, leaving earlier files intact.
    #[test]
    fn test_enforce_response_budget_truncates_after_limit() {
        fn one(path: &str, content: &str) -> FileDiff {
            FileDiff {
                path: path.to_string(),
                old_path: None,
                status: "modified".to_string(),
                hunks: vec![DiffHunkInfo {
                    header: "@@ -1 +1 @@".to_string(),
                    old_start: 1,
                    old_lines: 1,
                    new_start: 1,
                    new_lines: 1,
                    lines: vec![DiffLineInfo {
                        origin: '+',
                        content: content.to_string(),
                        old_lineno: None,
                        new_lineno: Some(1),
                    }],
                }],
                additions: 1,
                deletions: 0,
                truncated: false,
                binary: false,
            }
        }

        let mut files = vec![
            one("a", &"a".repeat(100)),
            one("b", &"b".repeat(100)),
            one("c", &"c".repeat(100)),
        ];
        // Budget of 150 bytes: "a" (100) fits, "b" tips the total over the
        // limit and is truncated along with everything after it.
        enforce_response_budget(&mut files, 150);
        assert!(!files[0].truncated, "first file stays within budget");
        assert!(!files[0].hunks.is_empty());
        assert!(files[1].truncated && files[1].hunks.is_empty());
        assert!(files[2].truncated && files[2].hunks.is_empty());
    }

    /// Stats report per-file add/del counts and status without materializing
    /// hunks (the `FileDiffStat` shape has no hunks by construction).
    #[test]
    fn test_diff_stats_workdir_counts() {
        let (dir, repo) = create_repo_with_file();
        // test.txt starts as "line 1\nline 2\nline 3\n". Change line 2 and
        // append a new line: 2 additions, 1 deletion.
        fs::write(
            dir.path().join("test.txt"),
            "line 1\nCHANGED\nline 3\nline 4\n",
        )
        .unwrap();

        let stats = repo.diff_stats_workdir().unwrap();
        let s = stats
            .iter()
            .find(|s| s.path == "test.txt")
            .expect("modified file must appear in stats");
        assert_eq!(s.status, "modified");
        assert_eq!(s.additions, 2);
        assert_eq!(s.deletions, 1);
        assert!(!s.binary);
    }

    /// Stats flag binary files (0/0 counts) so the list can mark them and the
    /// lazy per-file fetch renders a placeholder.
    #[test]
    fn test_diff_stats_flags_binary() {
        let (dir, repo) = create_repo_with_file();
        fs::write(dir.path().join("b.bin"), [0u8, 1, 2, 3, 0, 5]).unwrap();
        let stats = repo.diff_stats_workdir().unwrap();
        let s = stats
            .iter()
            .find(|s| s.path == "b.bin")
            .expect("binary file must appear in stats");
        assert!(s.binary, "binary file must be flagged in stats");
        assert_eq!(s.additions, 0);
        assert_eq!(s.deletions, 0);
    }

    /// The lazy single-file diff returns the file's hunks for both the
    /// workdir side and `None` for a path with no change.
    #[test]
    fn test_diff_single_file_workdir() {
        let (dir, repo) = create_repo_with_file();
        fs::write(dir.path().join("test.txt"), "line 1\nchanged\nline 3\n").unwrap();
        let d = repo
            .diff_single_file("test.txt", false)
            .unwrap()
            .expect("changed file must have a diff");
        assert_eq!(d.path, "test.txt");
        assert!(!d.hunks.is_empty());
        assert!(
            repo.diff_single_file("does-not-exist.txt", false)
                .unwrap()
                .is_none(),
            "a path with no change must return None"
        );
    }

    /// An untracked file is fetchable via the lazy single-file path (its
    /// content is included, matching `diff_workdir`).
    #[test]
    fn test_diff_single_file_untracked() {
        let (dir, repo) = create_repo_with_file();
        fs::write(dir.path().join("new.txt"), "hello\nworld\n").unwrap();
        let d = repo
            .diff_single_file("new.txt", false)
            .unwrap()
            .expect("untracked file must have a diff");
        assert_eq!(d.status, "untracked");
        assert!(d.additions > 0);
    }
}
