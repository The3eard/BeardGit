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

/// Directory entries with these names are always skipped, regardless of
/// `respect_gitignore`.
///
/// Deliberately short, and shorter than it is tempting to make it. This
/// list wins over the repo's own `.gitignore`, so anything on it is
/// unreachable in the editor — a tracked file inside a skipped directory
/// simply does not exist as far as the user is concerned. That is only
/// acceptable for names that are never hand-authored source.
///
/// Which is why `build`, `dist`, `out`, `bin`, `obj`, `vendor` and `Pods`
/// are **not** here despite being the usual suspects: `bin/` routinely
/// holds committed scripts, Go's `vendor/` is committed dependency
/// *source*, checking `Pods/` in is a mainstream CocoaPods workflow and it
/// is dependency source by the same argument, and plenty of projects keep
/// real files in `build/`. When those directories
/// are genuinely build output the repo ignores them, and `respect_gitignore`
/// — now on by default — takes care of it with an escape hatch the user
/// controls. The names below have no such ambiguity.
const ALWAYS_SKIP_DIR_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    // Tool caches and virtualenvs. None is ever authored by hand, and all
    // of them are large enough to make a directory listing useless.
    ".gradle",
    ".venv",
    "__pycache__",
    ".next",
    ".turbo",
    "DerivedData",
];

/// How many directory entries [`Repository::search_workdir_files`] will
/// look at before giving up on the rest of the tree.
///
/// A bound on work, not on the answer. Reaching it on a real repository
/// means something enormous and unignored is in the way; because the walk
/// is breadth-first, what has been examined by then is the shallow part of
/// the tree, whose matches are the ones that would have ranked first
/// anyway.
const SEARCH_SCAN_CEILING: usize = 50_000;

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
/// Covers [`ALWAYS_SKIP_DIR_NAMES`] and the ai-worktree subdir under
/// `.beardgit/`. `file_type` is the *resolved* type: a symlink to a
/// directory is a directory here, so `node_modules -> ../shared/node_modules`
/// is skipped like the real thing.
fn should_skip_entry(rel_path: &str, file_type: &std::fs::FileType, name: &str) -> bool {
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
    /// Always one level. `None` lists the repo root, `Some(dir)` lists that
    /// directory's immediate children — the caller expands one folder at a
    /// time.
    ///
    /// It used to walk the whole tree when `prefix` was `None`, capped at a
    /// caller-supplied maximum. The cap ran against a depth-first walk and
    /// stopped it dead wherever it happened to be, so directories came back
    /// listed but childless and the frontend drew folders that expanded to
    /// nothing — "I can't open the `main` folder, it isn't listed". There
    /// is no budget to spend now: a level is a level.
    ///
    /// # Parameters
    /// - `prefix` – Directory to list. `None` or `""` means the repo root.
    /// - `max_entries` – Guard against a single pathological directory, not
    ///   a tree budget. One directory with more children than this is
    ///   returned truncated; nothing else is affected.
    /// - `respect_gitignore` – When `true`, entries that match the repo's
    ///   gitignore patterns (via
    ///   [`git2::Repository::status_should_ignore`]) are filtered out.
    ///
    /// Always skipped, regardless of `respect_gitignore`: see
    /// [`ALWAYS_SKIP_DIR_NAMES`], plus `.beardgit/ai-worktrees/`.
    /// Symlinks are followed: a link to a directory lists as a directory
    /// (and expands to the target's children), a link to a file lists as a
    /// file. Only dangling links are dropped.
    ///
    /// Sort order: directories first, then files; within each group,
    /// alphabetical case-insensitive.
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

        if let Ok(read) = std::fs::read_dir(&start) {
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
                );
            }
        }

        // Directories first, then files, each group case-insensitive by
        // path. Sorting by path rather than `name` costs nothing within one
        // level and keeps the comparison identical to the search results,
        // which do span directories.
        out.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.path.to_lowercase().cmp(&b.path.to_lowercase()),
        });

        Ok(out)
    }

    /// Find files whose repo-relative path contains `query`, case-insensitively.
    ///
    /// This is the other half of dropping the recursive listing. The tree
    /// filter used to run in the browser over whatever the truncated walk
    /// had returned, so typing a name that existed but had not survived the
    /// cap found nothing — and the footer told the user to "refine the
    /// filter to see more", which asked the backend for exactly nothing.
    /// Refining a filter has to be able to reach files the tree has not
    /// expanded, so the walk lives here.
    ///
    /// Files only: a directory is not something the user is searching *for*
    /// in a file finder, and returning both makes the result list read as
    /// two interleaved things.
    ///
    /// `limit` caps the *result*, not the walk. Stopping the walk at the
    /// first `limit` matches would repeat the mistake this method exists to
    /// undo, one level down: the matches you get would be whichever
    /// directory the walk entered first, and a file two levels from the
    /// root would be hidden behind thirty in one deep folder. The walk
    /// collects everything, ranks it, and then truncates — bounded by
    /// [`SEARCH_SCAN_CEILING`] entries examined, which is about the walk's
    /// cost rather than about the answer.
    pub fn search_workdir_files(
        &self,
        query: &str,
        limit: usize,
        respect_gitignore: bool,
    ) -> Result<Vec<WorkdirTreeEntry>, GitError> {
        let needle = query.trim().to_lowercase();
        let mut out: Vec<WorkdirTreeEntry> = Vec::new();
        if needle.is_empty() || limit == 0 {
            return Ok(out);
        }

        let repo_root = self.path().to_path_buf();
        // Breadth-first: if the ceiling is ever reached, what has been
        // examined is the shallow part of the tree, which is also the part
        // whose matches rank highest.
        let mut queue: Vec<PathBuf> = vec![repo_root.clone()];
        let mut head = 0usize;
        let mut examined = 0usize;

        while head < queue.len() && examined < SEARCH_SCAN_CEILING {
            let dir = queue[head].clone();
            head += 1;

            let read = match std::fs::read_dir(&dir) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for entry in read.flatten() {
                if examined >= SEARCH_SCAN_CEILING {
                    break;
                }
                examined += 1;
                let mut one: Vec<WorkdirTreeEntry> = Vec::new();
                push_entry(
                    &repo_root,
                    respect_gitignore,
                    self.inner(),
                    &entry,
                    &mut one,
                );
                let Some(found) = one.pop() else { continue };
                if found.is_directory {
                    // A symlinked directory is listed when the user expands
                    // it, but the walk does not follow it: a link back up
                    // the tree would otherwise spend the whole scan ceiling
                    // going in circles.
                    let is_link = entry.file_type().map(|t| t.is_symlink()).unwrap_or(false);
                    if !is_link {
                        queue.push(entry.path());
                    }
                } else if found.path.to_lowercase().contains(&needle) {
                    out.push(found);
                }
            }
        }

        // Shallowest first: a match on `src/a.ts` is almost always more
        // interesting than one on `src/very/deep/nested/a.ts`, and ordering
        // by path alone buries the former under whatever sorts earlier.
        out.sort_by(|a, b| {
            a.path
                .matches('/')
                .count()
                .cmp(&b.path.matches('/').count())
                .then_with(|| a.path.to_lowercase().cmp(&b.path.to_lowercase()))
        });
        out.truncate(limit);
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
) {
    let Ok(link_type) = entry.file_type() else {
        return;
    };
    let name = entry.file_name().to_string_lossy().into_owned();
    let full = entry.path();
    // `DirEntry::file_type` does not follow symlinks; the listing wants what
    // the link points at. `metadata` resolves the chain and fails on a
    // dangling link, which is the one case the tree drops.
    let file_type = if link_type.is_symlink() {
        match std::fs::metadata(&full) {
            Ok(meta) => meta.file_type(),
            Err(_) => return,
        }
    } else {
        link_type
    };
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
            path: rel,
            name,
            is_directory: true,
            size: None,
        });
    } else if file_type.is_file() {
        // `std::fs::metadata`, not `entry.metadata()`: the latter reports the
        // link itself, and a link's size is the length of its target path.
        let size = std::fs::metadata(&full).ok().map(|m| m.len());
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
        // One level: `sub`'s contents are `sub`'s business.
        assert!(!names.contains(&"b.txt"));
    }

    /// Acceptance criterion for the lazy tree: expanding a folder reads
    /// that folder and stops.
    ///
    /// This started life with a wall-clock bound and that was theatre. On
    /// this fixture the one-level listing measures ~5ms and the recursive
    /// walk it replaced ~14ms — 2.8× slower and still 36× inside the 500ms
    /// the assertion allowed, so it could not tell the fix from the bug.
    /// The count can: 1,001 is this directory, and any descent adds the
    /// 1,200 files below it. Gitignore stays on because the per-entry
    /// `status_should_ignore` call is the expensive one, and measuring the
    /// cheap configuration would be its own kind of theatre.
    #[test]
    fn list_workdir_tree_does_not_descend_into_subdirectories() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let big = path.join("big");
        fs::create_dir_all(&big).unwrap();
        for i in 0..1_000 {
            fs::write(big.join(format!("f{i:04}.ts")), "x").unwrap();
        }
        // Depth the walk would have to cross if it ever recursed again.
        let mut deep = path.join("big");
        for level in 0..6 {
            deep = deep.join(format!("level{level}"));
            fs::create_dir_all(&deep).unwrap();
            for i in 0..200 {
                fs::write(deep.join(format!("n{i:03}.ts")), "x").unwrap();
            }
        }

        let repo = Repository::open(&path).unwrap();
        let entries = repo.list_workdir_tree(Some("big"), 5_000, true).unwrap();

        assert_eq!(
            entries.len(),
            1_001,
            "1000 files plus the `level0` directory — anything more means the \
             interactive path walked the subtree"
        );
        assert!(
            entries.iter().all(|e| !e.path.contains("level0/")),
            "a descendant of `level0` reached a one-level listing"
        );
    }

    #[test]
    fn search_workdir_files_reaches_files_the_tree_has_not_expanded() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::create_dir_all(path.join("src/deeply/nested/place")).unwrap();
        fs::write(path.join("src/deeply/nested/place/needle.ts"), "x").unwrap();
        fs::write(path.join("unrelated.ts"), "x").unwrap();

        let repo = Repository::open(&path).unwrap();
        let hits = repo.search_workdir_files("needle", 50, false).unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "src/deeply/nested/place/needle.ts");
        assert!(!hits[0].is_directory);
    }

    #[test]
    fn search_workdir_files_matches_on_the_whole_path_case_insensitively() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::create_dir_all(path.join("Components")).unwrap();
        fs::write(path.join("Components/button.ts"), "x").unwrap();
        fs::write(path.join("readme.md"), "x").unwrap();

        let repo = Repository::open(&path).unwrap();

        // Matches a directory segment, not just the file name.
        let by_dir = repo.search_workdir_files("components/", 50, false).unwrap();
        assert_eq!(by_dir.len(), 1);
        assert_eq!(by_dir[0].name, "button.ts");

        assert_eq!(
            repo.search_workdir_files("BUTTON", 50, false)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn search_workdir_files_returns_shallowest_matches_first() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::create_dir_all(path.join("a/b/c")).unwrap();
        fs::write(path.join("target.ts"), "x").unwrap();
        fs::write(path.join("a/target.ts"), "x").unwrap();
        fs::write(path.join("a/b/c/target.ts"), "x").unwrap();

        let repo = Repository::open(&path).unwrap();
        let hits = repo.search_workdir_files("target.ts", 50, false).unwrap();

        let paths: Vec<&str> = hits.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, ["target.ts", "a/target.ts", "a/b/c/target.ts"]);
    }

    #[test]
    fn search_workdir_files_honours_skips_gitignore_and_the_empty_query() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::write(path.join(".gitignore"), "secret.key\n").unwrap();
        fs::write(path.join("secret.key"), "x").unwrap();
        fs::create_dir_all(path.join("node_modules/pkg")).unwrap();
        fs::write(path.join("node_modules/pkg/secret.key"), "x").unwrap();

        let repo = Repository::open(&path).unwrap();

        // The skip list wins even with gitignore off.
        let all = repo.search_workdir_files("secret", 50, false).unwrap();
        assert_eq!(all.len(), 1, "node_modules must never be walked");
        assert_eq!(all[0].path, "secret.key");

        assert!(
            repo.search_workdir_files("secret", 50, true)
                .unwrap()
                .is_empty()
        );
        // An empty query is not "match everything".
        assert!(
            repo.search_workdir_files("   ", 50, false)
                .unwrap()
                .is_empty()
        );
        assert!(
            repo.search_workdir_files("secret", 0, false)
                .unwrap()
                .is_empty()
        );
    }

    /// The cap truncates a *ranked* list, it does not stop the walk.
    ///
    /// Stopping early would repeat, one level down, the bug the tree
    /// listing was just rescued from: the results you get would be
    /// whichever directory happened to be read first. Here one directory
    /// holds far more matches than the cap, and the single match sitting at
    /// the repo root still has to come back — it outranks all of them.
    #[test]
    fn search_workdir_files_cap_truncates_by_rank_not_by_walk_order() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let noisy = path.join("aaa_first_alphabetically");
        fs::create_dir_all(&noisy).unwrap();
        for f in 0..40 {
            fs::write(noisy.join(format!("hit{f:02}.ts")), "x").unwrap();
        }
        fs::write(path.join("hit-at-the-root.ts"), "x").unwrap();

        let repo = Repository::open(&path).unwrap();
        let hits = repo.search_workdir_files("hit", 5, false).unwrap();

        assert_eq!(hits.len(), 5, "the cap must be honoured");
        assert_eq!(
            hits[0].path, "hit-at-the-root.ts",
            "the shallowest match must survive a directory with 40 of its own"
        );
    }

    /// A symlinked directory used to vanish from the tree — "the tree does
    /// not know what to do with symlinks". It lists as a directory, and
    /// expanding it lists the target's children under the link's path.
    #[cfg(unix)]
    #[test]
    fn list_workdir_tree_follows_symlinks() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::create_dir_all(path.join("real")).unwrap();
        fs::write(path.join("real/inner.txt"), "i").unwrap();
        fs::write(path.join("real-file.txt"), "f").unwrap();
        std::os::unix::fs::symlink(path.join("real"), path.join("linkdir")).unwrap();
        std::os::unix::fs::symlink(path.join("real-file.txt"), path.join("linkfile.txt")).unwrap();
        std::os::unix::fs::symlink(path.join("missing"), path.join("dangling")).unwrap();

        let repo = Repository::open(&path).unwrap();
        let root = repo.list_workdir_tree(None, 100, false).unwrap();

        let linkdir = root
            .iter()
            .find(|e| e.name == "linkdir")
            .expect("linkdir listed");
        assert!(linkdir.is_directory);
        let linkfile = root
            .iter()
            .find(|e| e.name == "linkfile.txt")
            .expect("linkfile listed");
        assert!(!linkfile.is_directory);
        assert_eq!(
            linkfile.size,
            Some(1),
            "size is the target's, not the link's"
        );
        assert!(!root.iter().any(|e| e.name == "dangling"));

        let children = repo.list_workdir_tree(Some("linkdir"), 100, false).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].path, "linkdir/inner.txt");
    }

    /// The search walk must not follow a symlinked directory: a link back up
    /// the tree is a cycle, and the scan ceiling is the only thing that
    /// would stop it.
    #[cfg(unix)]
    #[test]
    fn search_workdir_files_does_not_descend_into_symlinked_directories() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        fs::create_dir_all(path.join("real")).unwrap();
        fs::write(path.join("real/needle.txt"), "n").unwrap();
        std::os::unix::fs::symlink(&path, path.join("real/loop")).unwrap();
        std::os::unix::fs::symlink(path.join("real"), path.join("linkdir")).unwrap();

        let repo = Repository::open(&path).unwrap();
        let hits = repo.search_workdir_files("needle", 50, false).unwrap();

        let paths: Vec<&str> = hits.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, ["real/needle.txt"]);
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

    /// **The bug behind "I can't open the `main` folder, it isn't listed".**
    ///
    /// The listing used to walk the whole tree depth-first and stop dead at
    /// `max_entries`, so the cap did not trim evenly — it stopped wherever
    /// the walk happened to be, and whole directories came back present but
    /// childless. The frontend then drew a folder that expanded to nothing.
    /// With five directories of fifty files and a cap of sixty, three of
    /// the five used to arrive empty.
    ///
    /// One level at a time, the shape cannot happen: the root listing
    /// returns directories without claiming to know their contents, and
    /// each one answers for itself in full.
    #[test]
    fn list_workdir_tree_never_returns_a_childless_directory() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        for d in 0..5 {
            let dir = path.join(format!("dir{d}"));
            fs::create_dir_all(&dir).unwrap();
            for f in 0..50 {
                fs::write(dir.join(format!("f{f}.txt")), "x").unwrap();
            }
        }

        let repo = Repository::open(&path).unwrap();
        let root = repo.list_workdir_tree(None, 60, false).unwrap();

        // Every directory is present, and none of them carries children:
        // the root level does not speak for what is inside.
        for d in 0..5 {
            let name = format!("dir{d}");
            assert!(
                root.iter().any(|e| e.path == name && e.is_directory),
                "{name} missing from the root listing"
            );
            assert!(
                !root.iter().any(|e| e.path.starts_with(&format!("{name}/"))),
                "{name} leaked descendants into a one-level listing"
            );
        }

        // And each one answers in full when asked, cap or no cap.
        for d in 0..5 {
            let name = format!("dir{d}");
            let children = repo.list_workdir_tree(Some(&name), 1_000, false).unwrap();
            assert_eq!(
                children.len(),
                50,
                "{name} returned {} of its 50 files",
                children.len()
            );
        }
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
