//! Git clean operations — preview and remove untracked files.
//!
//! Extends [`Repository`] with dry-run listing and safe file removal.
//! Uses `git clean -n` for preview and `std::fs` for actual deletion,
//! with path validation to ensure all targets are within the repository root.

use serde::Serialize;

use crate::error::GitError;
use crate::repository::Repository;

/// A single untracked item that would be removed by `git clean`.
#[derive(Debug, Clone, Serialize)]
pub struct CleanItem {
    /// Repository-relative path of the file or directory.
    pub path: String,
    /// `true` when this item is a directory.
    pub is_directory: bool,
    /// `true` when this item is an ignored file (only present with include/only-ignored flags).
    pub is_ignored: bool,
}

impl Repository {
    /// Preview which files/directories would be removed by `git clean`.
    ///
    /// Runs `git clean -n` with the appropriate flags and parses the output.
    /// Does not actually delete anything.
    ///
    /// # Parameters
    /// - `include_directories` — include untracked directories (`-d` flag).
    /// - `include_ignored` — also remove ignored files (`-x` flag).
    /// - `only_ignored` — remove *only* ignored files (`-X` flag).
    ///
    /// `include_ignored` and `only_ignored` are mutually exclusive. If both are
    /// `true`, `only_ignored` takes precedence.
    pub fn clean_dry_run(
        &self,
        include_directories: bool,
        include_ignored: bool,
        only_ignored: bool,
    ) -> Result<Vec<CleanItem>, GitError> {
        let mut args = vec!["clean", "-n"];
        if include_directories {
            args.push("-d");
        }
        // -X (only ignored) takes precedence over -x (include ignored)
        if only_ignored {
            args.push("-X");
        } else if include_ignored {
            args.push("-x");
        }

        let result = self.git_cmd(&args)?;
        if !result.success {
            return Err(GitError::RepoNotFound(result.stderr));
        }

        Ok(parse_clean_output(&result.stdout, only_ignored))
    }

    /// Remove the specified paths from the working directory.
    ///
    /// Each path is validated to be within the repository root before deletion.
    /// Returns the number of successfully removed items.
    ///
    /// # Safety
    /// This permanently deletes files. There is no undo. Callers should confirm
    /// with the user before invoking this method.
    pub fn clean_paths(&self, paths: &[String]) -> Result<u32, GitError> {
        let repo_root = self.path().canonicalize().map_err(GitError::Io)?;
        let mut removed = 0u32;

        for rel_path in paths {
            let full = repo_root.join(rel_path);
            // Canonicalize to resolve symlinks, then verify it's inside the repo
            let canonical = match full.canonicalize() {
                Ok(p) => p,
                Err(_) => continue, // file already gone
            };
            if !canonical.starts_with(&repo_root) {
                continue; // path traversal — skip silently
            }

            if canonical.is_dir() {
                if std::fs::remove_dir_all(&canonical).is_ok() {
                    removed += 1;
                }
            } else if std::fs::remove_file(&canonical).is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }
}

/// Parse `git clean -n` output into [`CleanItem`] structs.
///
/// Each line has the form:
/// - `Would remove path/to/file`
/// - `Would remove path/to/dir/`
fn parse_clean_output(output: &str, is_ignored_mode: bool) -> Vec<CleanItem> {
    output
        .lines()
        .filter_map(|line| {
            let path = line.strip_prefix("Would remove ")?;
            let path = path.trim();
            if path.is_empty() {
                return None;
            }
            let is_directory = path.ends_with('/');
            let clean_path = if is_directory {
                path.trim_end_matches('/')
            } else {
                path
            };
            Some(CleanItem {
                path: clean_path.to_string(),
                is_directory,
                is_ignored: is_ignored_mode,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clean_output_files() {
        let output = "Would remove foo.txt\nWould remove bar.log\n";
        let items = parse_clean_output(output, false);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].path, "foo.txt");
        assert!(!items[0].is_directory);
        assert!(!items[0].is_ignored);
        assert_eq!(items[1].path, "bar.log");
    }

    #[test]
    fn test_parse_clean_output_directories() {
        let output = "Would remove build/\nWould remove .cache/\n";
        let items = parse_clean_output(output, false);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].path, "build");
        assert!(items[0].is_directory);
        assert_eq!(items[1].path, ".cache");
        assert!(items[1].is_directory);
    }

    #[test]
    fn test_parse_clean_output_ignored_mode() {
        let output = "Would remove node_modules/\n";
        let items = parse_clean_output(output, true);
        assert_eq!(items.len(), 1);
        assert!(items[0].is_ignored);
    }

    #[test]
    fn test_parse_clean_output_empty() {
        let items = parse_clean_output("", false);
        assert!(items.is_empty());
    }

    #[test]
    fn test_parse_clean_output_mixed() {
        let output = "Would remove src/temp.rs\nWould remove dist/\nWould remove .env\n";
        let items = parse_clean_output(output, false);
        assert_eq!(items.len(), 3);
        assert!(!items[0].is_directory);
        assert!(items[1].is_directory);
        assert!(!items[2].is_directory);
    }

    // ── Integration tests ──────────────────────────────────────────────────

    fn create_test_repo() -> (tempfile::TempDir, crate::repository::Repository) {
        let tmp = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(tmp.path()).unwrap();
        {
            let mut cfg = git_repo.config().unwrap();
            cfg.set_str("user.name", "Test").unwrap();
            cfg.set_str("user.email", "test@test.com").unwrap();
        }
        // Need at least one commit for clean to work
        std::fs::write(tmp.path().join("tracked.txt"), "hello").unwrap();
        {
            let mut index = git_repo.index().unwrap();
            index.add_path(std::path::Path::new("tracked.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = git_repo.find_tree(tree_id).unwrap();
            let sig = git_repo.signature().unwrap();
            git_repo
                .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }
        drop(git_repo);
        let repo = crate::repository::Repository::open(tmp.path()).unwrap();
        (tmp, repo)
    }

    #[test]
    fn test_clean_dry_run_shows_untracked() {
        let (tmp, repo) = create_test_repo();
        std::fs::write(tmp.path().join("untracked.txt"), "junk").unwrap();

        let items = repo.clean_dry_run(false, false, false).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, "untracked.txt");
    }

    #[test]
    fn test_clean_dry_run_with_directories() {
        let (tmp, repo) = create_test_repo();
        std::fs::create_dir(tmp.path().join("tempdir")).unwrap();
        std::fs::write(tmp.path().join("tempdir/file.txt"), "junk").unwrap();

        // Without -d, directory should NOT appear
        let items_no_d = repo.clean_dry_run(false, false, false).unwrap();
        assert!(items_no_d.iter().all(|i| i.path != "tempdir"));

        // With -d, directory should appear
        let items_with_d = repo.clean_dry_run(true, false, false).unwrap();
        assert!(items_with_d.iter().any(|i| i.path == "tempdir"));
    }

    #[test]
    fn test_clean_paths_removes_files() {
        let (tmp, repo) = create_test_repo();
        std::fs::write(tmp.path().join("junk.txt"), "junk").unwrap();
        assert!(tmp.path().join("junk.txt").exists());

        let removed = repo.clean_paths(&["junk.txt".to_string()]).unwrap();
        assert_eq!(removed, 1);
        assert!(!tmp.path().join("junk.txt").exists());
    }

    #[test]
    fn test_clean_paths_removes_directories() {
        let (tmp, repo) = create_test_repo();
        std::fs::create_dir(tmp.path().join("junkdir")).unwrap();
        std::fs::write(tmp.path().join("junkdir/file.txt"), "junk").unwrap();

        let removed = repo.clean_paths(&["junkdir".to_string()]).unwrap();
        assert_eq!(removed, 1);
        assert!(!tmp.path().join("junkdir").exists());
    }

    #[test]
    fn test_clean_paths_skips_missing() {
        let (_tmp, repo) = create_test_repo();
        let removed = repo.clean_paths(&["nonexistent.txt".to_string()]).unwrap();
        assert_eq!(removed, 0);
    }
}
