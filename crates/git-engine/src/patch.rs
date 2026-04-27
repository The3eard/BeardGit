//! Patch creation and application via the git CLI.
//!
//! Extends [`Repository`] with methods for creating patches from commits or
//! the working tree, previewing patch stats, and applying patches with
//! optional 3-way merge fallback.

use serde::Serialize;

use crate::error::GitError;
use crate::repository::Repository;

/// Per-file diff statistics from a patch.
#[derive(Debug, Clone, Serialize)]
pub struct PatchStat {
    /// Relative file path affected by the patch.
    pub path: String,
    /// Number of lines inserted.
    pub insertions: u32,
    /// Number of lines deleted.
    pub deletions: u32,
}

/// Result of previewing a patch before applying it.
#[derive(Debug, Clone, Serialize)]
pub struct PatchPreview {
    /// Whether `git apply --check` succeeded (patch applies cleanly).
    pub applies_cleanly: bool,
    /// Per-file statistics parsed from `git apply --stat`.
    pub stats: Vec<PatchStat>,
    /// Total number of files affected.
    pub total_files: u32,
    /// Total lines inserted across all files.
    pub total_insertions: u32,
    /// Total lines deleted across all files.
    pub total_deletions: u32,
}

impl Repository {
    /// Create patch files for the given commit OIDs using `git format-patch`.
    ///
    /// Each commit produces one `.patch` file in `output_dir`. Files are named
    /// by git's default scheme (`0001-subject.patch`, etc.). Returns the list
    /// of created file paths.
    pub fn create_commit_patches(
        &self,
        oids: &[String],
        output_dir: &str,
    ) -> Result<Vec<String>, GitError> {
        let mut paths = Vec::new();
        for (i, oid) in oids.iter().enumerate() {
            let result = self.git_cmd(&[
                "format-patch",
                "-1",
                oid,
                "-o",
                output_dir,
                "--start-number",
                &(i + 1).to_string(),
            ])?;
            if !result.success {
                return Err(GitError::CliError(result.stderr));
            }
            // git format-patch prints the created file path to stdout
            for line in result.stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    paths.push(trimmed.to_string());
                }
            }
        }
        Ok(paths)
    }

    /// Create a patch from the working tree diff.
    ///
    /// When `staged_only` is `true`, generates a diff of staged changes only
    /// (`git diff --cached`). When `false`, generates a diff of all changes
    /// (`git diff`). Returns the raw patch text.
    pub fn create_working_tree_patch(&self, staged_only: bool) -> Result<String, GitError> {
        let args = if staged_only {
            vec!["diff", "--cached"]
        } else {
            vec!["diff"]
        };
        let result = self.git_cmd(&args)?;
        if result.stdout.trim().is_empty() {
            return Err(GitError::CliError(
                "No changes to create patch from".to_string(),
            ));
        }
        Ok(result.stdout)
    }

    /// Preview a patch file by running `git apply --stat` and `--check`.
    ///
    /// Returns a [`PatchPreview`] with per-file stats and whether the patch
    /// applies cleanly to the current working tree.
    pub fn preview_patch(&self, patch_path: &str) -> Result<PatchPreview, GitError> {
        // Get stats
        let stat_result = self.git_cmd(&["apply", "--stat", patch_path])?;
        let stats = parse_apply_stat(&stat_result.stdout);

        // Check if it applies cleanly
        let check_result = self.git_cmd(&["apply", "--check", patch_path])?;
        let applies_cleanly = check_result.success;

        let total_files = stats.len() as u32;
        let total_insertions = stats.iter().map(|s| s.insertions).sum();
        let total_deletions = stats.iter().map(|s| s.deletions).sum();

        Ok(PatchPreview {
            applies_cleanly,
            stats,
            total_files,
            total_insertions,
            total_deletions,
        })
    }

    /// Apply a patch file to the working tree.
    ///
    /// When `three_way` is `true`, uses `git apply --3way` which falls back to
    /// a 3-way merge when the patch does not apply cleanly. This may leave
    /// conflict markers in files that the user must resolve.
    pub fn apply_patch_file(&self, patch_path: &str, three_way: bool) -> Result<String, GitError> {
        let mut args = vec!["apply"];
        if three_way {
            args.push("--3way");
        }
        args.push(patch_path);
        let result = self.git_cmd(&args)?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }
}

/// Parse the output of `git apply --stat <patch>`.
///
/// Each line looks like: ` path/to/file | 10 ++++------`
/// The summary line at the end looks like: ` 3 files changed, 5 insertions(+), 2 deletions(-)`
fn parse_apply_stat(output: &str) -> Vec<PatchStat> {
    let mut stats = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        // Skip the summary line and empty lines
        if trimmed.is_empty()
            || trimmed.contains("files changed")
            || trimmed.contains("file changed")
        {
            continue;
        }
        // Parse lines like: " src/main.rs | 10 ++++------"
        if let Some((path_part, change_part)) = trimmed.split_once('|') {
            let path = path_part.trim().to_string();
            let change = change_part.trim();
            // Count + and - characters after the number
            let insertions = change.chars().filter(|c| *c == '+').count() as u32;
            let deletions = change.chars().filter(|c| *c == '-').count() as u32;
            stats.push(PatchStat {
                path,
                insertions,
                deletions,
            });
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_apply_stat_basic() {
        let output = " src/main.rs  | 10 ++++------\n src/lib.rs   |  3 +++\n 2 files changed, 7 insertions(+), 6 deletions(-)\n";
        let stats = parse_apply_stat(output);
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].path, "src/main.rs");
        assert_eq!(stats[0].insertions, 4);
        assert_eq!(stats[0].deletions, 6);
        assert_eq!(stats[1].path, "src/lib.rs");
        assert_eq!(stats[1].insertions, 3);
        assert_eq!(stats[1].deletions, 0);
    }

    #[test]
    fn test_parse_apply_stat_empty() {
        let stats = parse_apply_stat("");
        assert!(stats.is_empty());
    }

    #[test]
    fn test_parse_apply_stat_single_file() {
        let output = " README.md | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n";
        let stats = parse_apply_stat(output);
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].path, "README.md");
        assert_eq!(stats[0].insertions, 1);
        assert_eq!(stats[0].deletions, 1);
    }
}
