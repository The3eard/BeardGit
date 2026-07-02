//! Raw file content retrieval for CodeMirror diff views.
//!
//! Provides three methods on [`Repository`] to fetch file content from:
//! - a specific commit (by OID)
//! - the working directory
//! - the index (staged version)
//!
//! Also hosts the shared [`validate_repo_relative_path`] helper used by
//! every workdir-mutating method (writes / creates / renames / deletes)
//! to refuse absolute paths, parent-traversal, and any path that would
//! resolve outside the repository's working tree.

use std::io::Write;
use std::path::{Component, Path, PathBuf};

/// Maximum size, in bytes, of a single blob returned by
/// [`Repository::get_file_at_commit`]. Beyond this, the call returns
/// [`GitError::FileTooLarge`] and the frontend renders a placeholder
/// instead of allocating a 50 MB string and asking CodeMirror to diff
/// it. 5 MB matches the cap on `commit_file_diff` in `diff.rs`.
pub const MAX_FILE_AT_COMMIT_BYTES: usize = 5 * 1024 * 1024;

use crate::error::GitError;
use crate::repository::Repository;

/// Validate a repo-relative path and return the absolute joined form.
///
/// Rejects:
/// - absolute paths (e.g. `/etc/passwd`, `C:\Windows`),
/// - any path containing a `..` component,
/// - empty paths or paths whose normalized form falls outside `repo_root`.
///
/// The check is purely lexical so it works for paths that don't yet exist
/// (e.g. when creating a new file). Symlinks within the working tree are
/// not resolved here.
pub fn validate_repo_relative_path(repo_root: &Path, rel_path: &str) -> Result<PathBuf, GitError> {
    if rel_path.is_empty() {
        return Err(GitError::InvalidPath("path is empty".into()));
    }

    let candidate = Path::new(rel_path);
    if candidate.is_absolute() {
        return Err(GitError::InvalidPath(format!(
            "absolute paths are not allowed: {rel_path}"
        )));
    }

    // Refuse `..` and any windows-prefix / root-dir component.
    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(segment) => normalized.push(segment),
            Component::CurDir => continue,
            Component::ParentDir => {
                return Err(GitError::InvalidPath(format!(
                    "path contains '..': {rel_path}"
                )));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(GitError::InvalidPath(format!(
                    "absolute paths are not allowed: {rel_path}"
                )));
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err(GitError::InvalidPath(format!(
            "path resolves to repo root: {rel_path}"
        )));
    }

    Ok(repo_root.join(normalized))
}

impl Repository {
    /// Returns the raw content of a file at a specific commit.
    ///
    /// # Parameters
    /// - `oid` – Full or abbreviated commit SHA.
    /// - `path` – Repo-relative file path.
    ///
    /// # Errors
    /// Returns [`GitError`] if the OID is invalid, the path does not exist in
    /// the commit tree, or the blob content is not valid UTF-8 (lossy decode).
    pub fn get_file_at_commit(&self, oid: &str, path: &str) -> Result<String, GitError> {
        let obj = self.inner().revparse_single(oid)?;
        let commit = obj.peel_to_commit()?;
        let tree = commit.tree()?;
        let entry = tree.get_path(std::path::Path::new(path))?;
        let blob = self.inner().find_blob(entry.id())?;
        let content = blob.content();
        if content.len() > MAX_FILE_AT_COMMIT_BYTES {
            return Err(GitError::FileTooLarge {
                size: content.len(),
            });
        }
        let sniff_len = content.len().min(8192);
        if content[..sniff_len].contains(&0u8) {
            return Err(GitError::Binary);
        }
        Ok(String::from_utf8_lossy(content).into_owned())
    }

    /// Returns the raw content of a file from the working directory.
    ///
    /// Applies the same guard as [`Repository::get_file_at_commit`]: files
    /// larger than [`MAX_FILE_AT_COMMIT_BYTES`] return
    /// [`GitError::FileTooLarge`], and files with a NUL byte in the first
    /// 8 KB return [`GitError::Binary`] — so the diff view renders a
    /// placeholder instead of loading a 50 MB blob onto the webview thread.
    ///
    /// # Parameters
    /// - `path` – Repo-relative file path.
    ///
    /// # Errors
    /// Returns [`GitError::Io`] if the file does not exist or cannot be read,
    /// [`GitError::FileTooLarge`] when oversized, or [`GitError::Binary`]
    /// when binary.
    pub fn get_file_workdir(&self, path: &str) -> Result<String, GitError> {
        let workdir = self.path().to_path_buf();
        let full_path = workdir.join(path);
        let content = std::fs::read(&full_path).map_err(GitError::Io)?;
        if content.len() > MAX_FILE_AT_COMMIT_BYTES {
            return Err(GitError::FileTooLarge {
                size: content.len(),
            });
        }
        let sniff_len = content.len().min(8192);
        if content[..sniff_len].contains(&0u8) {
            return Err(GitError::Binary);
        }
        Ok(String::from_utf8_lossy(&content).into_owned())
    }

    /// Returns the raw content of a file from the index (staged version).
    ///
    /// # Parameters
    /// - `path` – Repo-relative file path.
    ///
    /// # Errors
    /// Returns [`GitError::RepoNotFound`] if the file is not staged, or
    /// [`GitError::Git`] if the blob cannot be resolved.
    pub fn get_file_index(&self, path: &str) -> Result<String, GitError> {
        let index = self.inner().index()?;
        let entry = index
            .get_path(std::path::Path::new(path), 0)
            .ok_or_else(|| GitError::RepoNotFound(format!("File not in index: {path}")))?;
        let blob = self.inner().find_blob(entry.id)?;
        let content = blob.content();
        if content.len() > MAX_FILE_AT_COMMIT_BYTES {
            return Err(GitError::FileTooLarge {
                size: content.len(),
            });
        }
        let sniff_len = content.len().min(8192);
        if content[..sniff_len].contains(&0u8) {
            return Err(GitError::Binary);
        }
        Ok(String::from_utf8_lossy(content).into_owned())
    }

    /// Atomically write `content` to a file in the working directory.
    ///
    /// The write goes through a sibling tempfile + `std::fs::rename` so a
    /// crash mid-write never leaves a half-written file at the target
    /// path. Parent directories are created on demand. The path is
    /// validated up-front via [`validate_repo_relative_path`] — absolute
    /// paths and `..` segments are rejected with [`GitError::InvalidPath`].
    ///
    /// # Parameters
    /// - `rel_path` – Repo-relative, forward-slashed file path.
    /// - `content` – New file content as a UTF-8 string.
    ///
    /// # Errors
    /// - [`GitError::InvalidPath`] when `rel_path` fails validation.
    /// - [`GitError::Io`] for any underlying I/O failure (parent creation,
    ///   tempfile creation, rename, etc.).
    pub fn write_file_workdir(&self, rel_path: &str, content: &str) -> Result<(), GitError> {
        let full_path = validate_repo_relative_path(self.path(), rel_path)?;

        if let Some(parent) = full_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }

        // Stage the write into a sibling tempfile. Using the parent dir
        // (rather than the system temp dir) keeps the rename atomic on
        // every supported FS — `rename` across mounts is not.
        let parent = full_path
            .parent()
            .ok_or_else(|| GitError::InvalidPath(format!("path has no parent: {rel_path}")))?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
        tmp.write_all(content.as_bytes())?;
        tmp.flush()?;
        tmp.persist(&full_path).map_err(|e| GitError::Io(e.error))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::repository::Repository;

    fn init_repo_with_file(dir: &std::path::Path) -> Repository {
        let git_repo = git2::Repository::init(dir).unwrap();
        let mut config = git_repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        fs::write(dir.join("hello.txt"), "initial content").unwrap();
        let mut index = git_repo.index().unwrap();
        index.add_path(std::path::Path::new("hello.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        {
            let tree = git_repo.find_tree(tree_id).unwrap();
            let sig = git_repo.signature().unwrap();
            git_repo
                .commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }

        drop(config);
        drop(git_repo);
        Repository::open(dir).unwrap()
    }

    #[test]
    fn test_get_file_at_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        let head = repo.inner().head().unwrap().peel_to_commit().unwrap();
        let content = repo
            .get_file_at_commit(&head.id().to_string(), "hello.txt")
            .unwrap();
        assert_eq!(content, "initial content");
    }

    #[test]
    fn test_get_file_workdir() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        fs::write(tmp.path().join("hello.txt"), "modified content").unwrap();
        let content = repo.get_file_workdir("hello.txt").unwrap();
        assert_eq!(content, "modified content");
    }

    #[test]
    fn test_get_file_index() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        fs::write(tmp.path().join("hello.txt"), "staged content").unwrap();
        let mut index = repo.inner().index().unwrap();
        index.add_path(std::path::Path::new("hello.txt")).unwrap();
        index.write().unwrap();
        let content = repo.get_file_index("hello.txt").unwrap();
        assert_eq!(content, "staged content");
    }

    #[test]
    fn test_get_file_at_commit_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        let head = repo.inner().head().unwrap().peel_to_commit().unwrap();
        let result = repo.get_file_at_commit(&head.id().to_string(), "nonexistent.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_file_workdir_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        let result = repo.get_file_workdir("does_not_exist.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_file_index_empty_index() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        // File exists on disk but is not staged — index only holds what was added
        // After init_repo_with_file the initial commit was made, index is clean.
        // Asking for a file never staged should return an error.
        let result = repo.get_file_index("unstaged.txt");
        assert!(result.is_err());
    }

    #[test]
    fn validate_repo_relative_path_rejects_absolute() {
        let tmp = tempfile::tempdir().unwrap();
        let err = super::validate_repo_relative_path(tmp.path(), "/etc/passwd").unwrap_err();
        assert!(matches!(err, crate::error::GitError::InvalidPath(_)));
    }

    #[test]
    fn validate_repo_relative_path_rejects_parent_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let err = super::validate_repo_relative_path(tmp.path(), "../escape.txt").unwrap_err();
        assert!(matches!(err, crate::error::GitError::InvalidPath(_)));
    }

    #[test]
    fn validate_repo_relative_path_rejects_embedded_parent_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let err = super::validate_repo_relative_path(tmp.path(), "sub/../../escape").unwrap_err();
        assert!(matches!(err, crate::error::GitError::InvalidPath(_)));
    }

    #[test]
    fn validate_repo_relative_path_rejects_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let err = super::validate_repo_relative_path(tmp.path(), "").unwrap_err();
        assert!(matches!(err, crate::error::GitError::InvalidPath(_)));
    }

    #[test]
    fn validate_repo_relative_path_accepts_normal_path() {
        let tmp = tempfile::tempdir().unwrap();
        let p = super::validate_repo_relative_path(tmp.path(), "src/main.rs").unwrap();
        assert!(p.starts_with(tmp.path()));
        assert!(p.ends_with("main.rs"));
    }

    #[test]
    fn write_file_workdir_round_trips_content() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        repo.write_file_workdir("notes/todo.md", "first\nsecond\n")
            .unwrap();
        let on_disk = std::fs::read_to_string(tmp.path().join("notes/todo.md")).unwrap();
        assert_eq!(on_disk, "first\nsecond\n");
    }

    #[test]
    fn write_file_workdir_creates_parent_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        repo.write_file_workdir("a/b/c/deep.txt", "hello").unwrap();
        assert!(tmp.path().join("a/b/c/deep.txt").exists());
    }

    #[test]
    fn write_file_workdir_rejects_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        let err = repo
            .write_file_workdir("../escape.txt", "nope")
            .unwrap_err();
        assert!(matches!(err, crate::error::GitError::InvalidPath(_)));
    }

    #[test]
    fn write_file_workdir_overwrites_existing_atomically() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        // Pre-existing file must end up replaced by the new content; the
        // temp sibling used for the atomic rename should not linger.
        std::fs::write(tmp.path().join("hello.txt"), "before").unwrap();
        repo.write_file_workdir("hello.txt", "after").unwrap();
        let on_disk = std::fs::read_to_string(tmp.path().join("hello.txt")).unwrap();
        assert_eq!(on_disk, "after");
        let leftover_tmp = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(".tmp"))
            .count();
        assert_eq!(leftover_tmp, 0, "no stray tempfile should remain");
    }

    #[test]
    fn get_file_at_commit_returns_binary_error_for_blobs_with_nul() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        // Overwrite with binary content (PNG header start, includes 0x00).
        fs::write(
            tmp.path().join("bin.bin"),
            [0x89, b'P', b'N', b'G', 0x00, 0x01],
        )
        .unwrap();
        let head_sha = {
            let git_repo = git2::Repository::open(tmp.path()).unwrap();
            let mut index = git_repo.index().unwrap();
            index.add_path(std::path::Path::new("bin.bin")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = git_repo.find_tree(tree_id).unwrap();
            let sig = git_repo.signature().unwrap();
            let parent = git_repo.head().unwrap().peel_to_commit().unwrap();
            let oid = git_repo
                .commit(Some("HEAD"), &sig, &sig, "add bin", &tree, &[&parent])
                .unwrap();
            oid.to_string()
        };

        let err = repo
            .get_file_at_commit(&head_sha, "bin.bin")
            .expect_err("binary blob must error");
        assert!(matches!(err, crate::error::GitError::Binary));
    }

    #[test]
    fn get_file_workdir_returns_binary_error_for_nul() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        fs::write(tmp.path().join("bin.bin"), [0x00, 0x01, 0x02, 0x03]).unwrap();
        let err = repo
            .get_file_workdir("bin.bin")
            .expect_err("binary workdir file must error");
        assert!(matches!(err, crate::error::GitError::Binary));
    }

    #[test]
    fn get_file_workdir_returns_too_large_error() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        // 5 MB + 1 of NUL-free bytes trips the size cap before the sniff.
        let big = vec![b'a'; super::MAX_FILE_AT_COMMIT_BYTES + 1];
        fs::write(tmp.path().join("big.txt"), &big).unwrap();
        let err = repo
            .get_file_workdir("big.txt")
            .expect_err("oversized workdir file must error");
        assert!(matches!(err, crate::error::GitError::FileTooLarge { .. }));
    }

    #[test]
    fn get_file_index_returns_binary_error_for_nul() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_file(tmp.path());
        fs::write(tmp.path().join("bin.bin"), [0x00, 0x01, 0x02, 0x03]).unwrap();
        let mut index = repo.inner().index().unwrap();
        index.add_path(std::path::Path::new("bin.bin")).unwrap();
        index.write().unwrap();
        let err = repo
            .get_file_index("bin.bin")
            .expect_err("binary staged file must error");
        assert!(matches!(err, crate::error::GitError::Binary));
    }
}
