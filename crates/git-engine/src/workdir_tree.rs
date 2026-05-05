//! Working-directory listing + light CRUD on repo-relative paths.
//!
//! Used by the in-app mini editor to:
//! - browse the working tree (with gitignore filtering and skip-list),
//! - create empty files / new directories,
//! - rename files or folders,
//! - delete files or folders.
//!
//! All four mutating methods reuse [`crate::file_content`]'s
//! [`validate_repo_relative_path`][crate::file_content::validate_repo_relative_path]
//! so callers cannot escape the working tree, write absolute paths, or
//! traverse via `..`.

use std::path::{Path, PathBuf};

use crate::error::GitError;
use crate::file_content::validate_repo_relative_path;
use crate::repository::Repository;

/// Directory entries with these names are always skipped during listing,
/// regardless of `respect_gitignore`. The set is short on purpose:
/// `.git/` is mandatory; the others are common enough that loading their
/// trees would dwarf any "useful" file in the listing.
const ALWAYS_SKIP_DIR_NAMES: &[&str] = &[".git", "node_modules", "target"];

/// One entry in the working-directory listing returned by
/// [`Repository::list_workdir_tree`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkdirTreeEntry {
    /// Repo-relative, forward-slashed.
    pub path: String,
    /// File name (last segment).
    pub name: String,
    /// `true` for directories, `false` for files.
    pub is_directory: bool,
    /// Size in bytes for files, `None` for directories or on stat error.
    pub size: Option<u64>,
}

/// Internal: should this directory entry be skipped wholesale?
///
/// Covers `.git/`, `node_modules/`, `target/`, and the ai-worktree
/// subdir under `.beardgit/`. Symlinks are skipped here too — a code repo
/// rarely has them and resolving them safely is its own can of worms.
fn should_skip_entry(rel_path: &str, file_type: &std::fs::FileType, name: &str) -> bool {
    if file_type.is_symlink() {
        return true;
    }
    if file_type.is_dir() {
        if ALWAYS_SKIP_DIR_NAMES.contains(&name) {
            return true;
        }
        // .beardgit/ai-worktrees/ — narrowly skip just the ai-worktrees
        // subtree, not the whole .beardgit dir (which carries config users
        // might want to see).
        if rel_path == ".beardgit/ai-worktrees" {
            return true;
        }
    }
    false
}

/// Convert an OS path that lives inside `repo_root` into the repo-relative
/// forward-slash form the rest of the codebase uses on the wire.
fn rel_forward_slash(repo_root: &Path, full: &Path) -> Option<String> {
    let rel = full.strip_prefix(repo_root).ok()?;
    let mut out = String::new();
    for (i, comp) in rel.components().enumerate() {
        if let std::path::Component::Normal(seg) = comp {
            if i > 0 {
                out.push('/');
            }
            out.push_str(&seg.to_string_lossy());
        } else {
            return None;
        }
    }
    Some(out)
}

impl Repository {
    /// List entries from the working directory.
    ///
    /// # Parameters
    /// - `prefix` – When `Some`, list only the immediate children of that
    ///   sub-directory (one level only). When `None`, walk the entire
    ///   working tree recursively.
    /// - `max_entries` – Soft cap. The walk stops once this many entries
    ///   have been collected; the result is returned truncated. Callers
    ///   compare the returned length against the cap to decide whether to
    ///   show a "results truncated" hint — listing never errors on
    ///   overflow.
    /// - `respect_gitignore` – When `true`, entries that match the repo's
    ///   gitignore patterns (via
    ///   [`git2::Repository::status_should_ignore`]) are filtered out.
    ///
    /// Always skipped, regardless of `respect_gitignore`:
    /// `.git/`, `node_modules/`, `target/`, `.beardgit/ai-worktrees/`.
    /// Symlinks are skipped silently.
    ///
    /// Sort order: directories first, then files; within each group,
    /// alphabetical case-insensitive by `name`.
    pub fn list_workdir_tree(
        &self,
        prefix: Option<&str>,
        max_entries: usize,
        respect_gitignore: bool,
    ) -> Result<Vec<WorkdirTreeEntry>, GitError> {
        let repo_root = self.path().to_path_buf();
        let mut out: Vec<WorkdirTreeEntry> = Vec::new();

        let start = match prefix {
            Some(p) if !p.is_empty() => validate_repo_relative_path(&repo_root, p)?,
            _ => repo_root.clone(),
        };

        if !start.exists() {
            return Ok(out);
        }
        if !start.is_dir() {
            return Err(GitError::InvalidPath(format!(
                "prefix is not a directory: {}",
                prefix.unwrap_or("")
            )));
        }

        if prefix.is_some() {
            // Single-level listing.
            let mut stack: Vec<PathBuf> = Vec::new();
            stack.push(start);
            while let Some(dir) = stack.pop() {
                let read = match std::fs::read_dir(&dir) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                for entry in read.flatten() {
                    if out.len() >= max_entries {
                        break;
                    }
                    push_entry(
                        &repo_root,
                        respect_gitignore,
                        self.inner(),
                        &entry,
                        &mut out,
                        false, // single-level walk: never recurse
                        &mut Vec::new(),
                    );
                }
            }
        } else {
            // Recursive walk.
            let mut stack: Vec<PathBuf> = Vec::new();
            stack.push(repo_root.clone());
            while let Some(dir) = stack.pop() {
                if out.len() >= max_entries {
                    break;
                }
                let read = match std::fs::read_dir(&dir) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                for entry in read.flatten() {
                    if out.len() >= max_entries {
                        break;
                    }
                    push_entry(
                        &repo_root,
                        respect_gitignore,
                        self.inner(),
                        &entry,
                        &mut out,
                        true,
                        &mut stack,
                    );
                }
            }
        }

        // Directories first, then files. Within each group sort by full
        // *path* (case-insensitive) rather than `name`, so a recursive
        // walk groups siblings under the same parent together — sorting
        // by `name` alone interleaves files at different depths and
        // produces a chaotic root order once the frontend builds a tree
        // from the leaf paths.
        out.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.path.to_lowercase().cmp(&b.path.to_lowercase()),
        });

        Ok(out)
    }

    /// Create a new file or directory at `rel_path`.
    ///
    /// Errors when the path already exists. For files, an empty file is
    /// created and any missing parent directories are created on demand.
    /// For directories, all missing parents are created via `create_dir_all`.
    pub fn create_workdir_path(&self, rel_path: &str, is_directory: bool) -> Result<(), GitError> {
        let full = validate_repo_relative_path(self.path(), rel_path)?;
        if full.exists() {
            return Err(GitError::InvalidPath(format!(
                "path already exists: {rel_path}"
            )));
        }

        if is_directory {
            std::fs::create_dir_all(&full)?;
        } else {
            if let Some(parent) = full.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&full)?;
        }
        Ok(())
    }

    /// Rename a file or directory inside the working tree.
    ///
    /// Errors when `to_rel` already exists or `from_rel` does not. Both
    /// paths are validated; the operation is otherwise a plain
    /// `std::fs::rename`.
    pub fn rename_workdir_path(&self, from_rel: &str, to_rel: &str) -> Result<(), GitError> {
        let from = validate_repo_relative_path(self.path(), from_rel)?;
        let to = validate_repo_relative_path(self.path(), to_rel)?;

        if !from.exists() {
            return Err(GitError::InvalidPath(format!(
                "source path does not exist: {from_rel}"
            )));
        }
        if to.exists() {
            return Err(GitError::InvalidPath(format!(
                "destination path already exists: {to_rel}"
            )));
        }
        if let Some(parent) = to.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&from, &to)?;
        Ok(())
    }

    /// Delete a file or directory inside the working tree.
    ///
    /// Files are removed via `remove_file`; directories via
    /// `remove_dir_all` (recursive). Errors when the path does not exist.
    pub fn delete_workdir_path(&self, rel_path: &str) -> Result<(), GitError> {
        let full = validate_repo_relative_path(self.path(), rel_path)?;
        if !full.exists() {
            return Err(GitError::InvalidPath(format!(
                "path does not exist: {rel_path}"
            )));
        }
        let meta = std::fs::symlink_metadata(&full)?;
        if meta.file_type().is_dir() {
            std::fs::remove_dir_all(&full)?;
        } else {
            std::fs::remove_file(&full)?;
        }
        Ok(())
    }
}

/// Internal helper: convert a `DirEntry` into a `WorkdirTreeEntry` and,
/// if recursing, push directories onto the walker stack.
fn push_entry(
    repo_root: &Path,
    respect_gitignore: bool,
    git_repo: &git2::Repository,
    entry: &std::fs::DirEntry,
    out: &mut Vec<WorkdirTreeEntry>,
    recurse: bool,
    stack: &mut Vec<PathBuf>,
) {
    let file_type = match entry.file_type() {
        Ok(t) => t,
        Err(_) => return,
    };
    let name = entry.file_name().to_string_lossy().into_owned();
    let full = entry.path();
    let rel = match rel_forward_slash(repo_root, &full) {
        Some(r) => r,
        None => return,
    };

    if should_skip_entry(&rel, &file_type, &name) {
        return;
    }

    if respect_gitignore {
        // status_should_ignore takes a path relative to the workdir.
        if let Ok(true) = git_repo.status_should_ignore(Path::new(&rel)) {
            return;
        }
    }

    if file_type.is_dir() {
        out.push(WorkdirTreeEntry {
            path: rel.clone(),
            name,
            is_directory: true,
            size: None,
        });
        if recurse {
            stack.push(full);
        }
    } else if file_type.is_file() {
        let size = entry.metadata().ok().map(|m| m.len());
        out.push(WorkdirTreeEntry {
            path: rel,
            name,
            is_directory: false,
            size,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::create_repo_with_n_commits;
    use std::fs;

    #[test]
    fn list_workdir_tree_returns_files_and_dirs() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::write(path.join("a.txt"), "a").unwrap();
        fs::create_dir_all(path.join("sub")).unwrap();
        fs::write(path.join("sub/b.txt"), "b").unwrap();

        let repo = Repository::open(&path).unwrap();
        let entries = repo.list_workdir_tree(None, 100, false).unwrap();

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"sub"));
        assert!(names.contains(&"b.txt"));
    }

    #[test]
    fn list_workdir_tree_skips_dot_git_and_target() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::create_dir_all(path.join("target/release")).unwrap();
        fs::write(path.join("target/release/exe"), "x").unwrap();
        fs::create_dir_all(path.join("node_modules/foo")).unwrap();
        fs::write(path.join("node_modules/foo/index.js"), "j").unwrap();

        let repo = Repository::open(&path).unwrap();
        let entries = repo.list_workdir_tree(None, 100, false).unwrap();

        for e in &entries {
            assert!(!e.path.starts_with(".git"), "should skip .git: {}", e.path);
            assert!(
                !e.path.starts_with("target"),
                "should skip target/: {}",
                e.path
            );
            assert!(
                !e.path.starts_with("node_modules"),
                "should skip node_modules/: {}",
                e.path
            );
        }
    }

    #[test]
    fn list_workdir_tree_respects_gitignore_when_requested() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::write(path.join(".gitignore"), "ignored.log\n").unwrap();
        fs::write(path.join("ignored.log"), "noise").unwrap();
        fs::write(path.join("kept.txt"), "good").unwrap();

        let repo = Repository::open(&path).unwrap();

        let with = repo.list_workdir_tree(None, 100, true).unwrap();
        let names_with: Vec<&str> = with.iter().map(|e| e.name.as_str()).collect();
        assert!(names_with.contains(&"kept.txt"));
        assert!(!names_with.contains(&"ignored.log"));

        let without = repo.list_workdir_tree(None, 100, false).unwrap();
        let names_without: Vec<&str> = without.iter().map(|e| e.name.as_str()).collect();
        assert!(names_without.contains(&"ignored.log"));
    }

    #[test]
    fn list_workdir_tree_truncates_at_max_entries() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        for i in 0..10 {
            fs::write(path.join(format!("f{i}.txt")), "x").unwrap();
        }
        let repo = Repository::open(&path).unwrap();
        let entries = repo.list_workdir_tree(None, 3, false).unwrap();
        assert!(entries.len() <= 3);
    }

    #[test]
    fn list_workdir_tree_with_prefix_lists_one_level() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::create_dir_all(path.join("sub/deeper")).unwrap();
        fs::write(path.join("sub/a.txt"), "a").unwrap();
        fs::write(path.join("sub/deeper/b.txt"), "b").unwrap();

        let repo = Repository::open(&path).unwrap();
        let entries = repo.list_workdir_tree(Some("sub"), 100, false).unwrap();

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"deeper"));
        // single-level walk: the deeper file must NOT show up here.
        assert!(!names.contains(&"b.txt"));
    }

    #[test]
    fn list_workdir_tree_sort_dirs_first_then_alpha() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::write(path.join("zfile.txt"), "z").unwrap();
        fs::write(path.join("afile.txt"), "a").unwrap();
        fs::create_dir_all(path.join("zdir")).unwrap();
        fs::create_dir_all(path.join("adir")).unwrap();

        let repo = Repository::open(&path).unwrap();
        let entries = repo.list_workdir_tree(None, 100, false).unwrap();

        let positions: std::collections::HashMap<&str, usize> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.name.as_str(), i))
            .collect();
        // Both directories come before either file.
        let last_dir = std::cmp::max(positions["adir"], positions["zdir"]);
        let first_file = std::cmp::min(positions["afile.txt"], positions["zfile.txt"]);
        assert!(
            last_dir < first_file,
            "directories must appear before files, got: {entries:?}"
        );
        // Alphabetical within group.
        assert!(positions["adir"] < positions["zdir"]);
        assert!(positions["afile.txt"] < positions["zfile.txt"]);
    }

    #[test]
    fn create_workdir_path_creates_file_then_errors_on_duplicate() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();

        repo.create_workdir_path("foo/bar.txt", false).unwrap();
        assert!(path.join("foo/bar.txt").is_file());

        let err = repo
            .create_workdir_path("foo/bar.txt", false)
            .expect_err("duplicate create must fail");
        assert!(matches!(err, GitError::InvalidPath(_)));
    }

    #[test]
    fn create_workdir_path_creates_directory() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        repo.create_workdir_path("new/dir/here", true).unwrap();
        assert!(path.join("new/dir/here").is_dir());
    }

    #[test]
    fn rename_and_delete_workdir_path_round_trip() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();

        fs::write(path.join("alpha.txt"), "a").unwrap();
        repo.rename_workdir_path("alpha.txt", "beta.txt").unwrap();
        assert!(!path.join("alpha.txt").exists());
        assert!(path.join("beta.txt").exists());

        repo.delete_workdir_path("beta.txt").unwrap();
        assert!(!path.join("beta.txt").exists());
    }

    #[test]
    fn rename_workdir_path_rejects_existing_destination() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        fs::write(path.join("a.txt"), "a").unwrap();
        fs::write(path.join("b.txt"), "b").unwrap();
        let err = repo
            .rename_workdir_path("a.txt", "b.txt")
            .expect_err("clobbering rename must fail");
        assert!(matches!(err, GitError::InvalidPath(_)));
    }

    #[test]
    fn delete_workdir_path_removes_directory_recursively() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        fs::create_dir_all(path.join("doomed/inner")).unwrap();
        fs::write(path.join("doomed/inner/x.txt"), "x").unwrap();
        repo.delete_workdir_path("doomed").unwrap();
        assert!(!path.join("doomed").exists());
    }

    #[test]
    fn workdir_crud_rejects_traversal() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        assert!(matches!(
            repo.create_workdir_path("../escape", false),
            Err(GitError::InvalidPath(_))
        ));
        assert!(matches!(
            repo.rename_workdir_path("a.txt", "../b.txt"),
            Err(GitError::InvalidPath(_))
        ));
        assert!(matches!(
            repo.delete_workdir_path("../escape"),
            Err(GitError::InvalidPath(_))
        ));
    }
}
