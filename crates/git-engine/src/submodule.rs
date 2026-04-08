//! Submodule management — list, init, update, and deinit git submodules.
//!
//! Uses libgit2's Submodule API for read operations (listing, status) and
//! the git CLI for write operations (init, update, deinit) since those
//! may involve network fetches and complex state changes.

use serde::Serialize;

use crate::error::GitError;
use crate::repository::Repository;

/// Status of a submodule relative to the superproject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmoduleStatus {
    /// Registered in `.gitmodules` but not yet initialized (`git submodule init` needed).
    Uninitialized,
    /// Checked out at the exact commit the superproject expects.
    Clean,
    /// Checked out but at a different commit than the superproject expects.
    Outdated,
    /// Has local modifications in its working tree.
    Dirty,
}

/// Information about a single submodule.
#[derive(Debug, Clone, Serialize)]
pub struct SubmoduleInfo {
    /// Submodule logical name (from `.gitmodules`).
    pub name: String,
    /// Relative path within the superproject working tree.
    pub path: String,
    /// Remote URL configured for this submodule.
    pub url: String,
    /// Current HEAD OID of the submodule working tree, or `None` if uninitialized.
    pub oid: Option<String>,
    /// The OID the superproject expects (recorded in the index/tree).
    pub registered_oid: String,
    /// Computed status of the submodule.
    pub status: SubmoduleStatus,
}

impl Repository {
    /// List all submodules registered in the repository.
    ///
    /// Uses libgit2's `Submodule` API for fast, no-fork reads. Status is
    /// computed by comparing the workdir HEAD, the index entry, and the
    /// presence of `.git` in the submodule directory.
    pub fn list_submodules(&self) -> Result<Vec<SubmoduleInfo>, GitError> {
        let sm_list = self.inner().submodules()?;

        let mut submodules = Vec::new();
        for sm in &sm_list {
            let name = sm.name().unwrap_or("").to_string();
            let path = sm.path().to_string_lossy().to_string();
            let url = sm.url().unwrap_or("").to_string();
            let registered_oid = sm.index_id().map(|id| id.to_string()).unwrap_or_default();

            // Determine status by checking submodule status flags
            let status_flags = self
                .inner()
                .submodule_status(&name, git2::SubmoduleIgnore::Unspecified)?;

            let oid;
            let status;

            if status_flags.contains(git2::SubmoduleStatus::WD_UNINITIALIZED) {
                oid = None;
                status = SubmoduleStatus::Uninitialized;
            } else {
                let workdir_oid = sm.workdir_id().map(|id| id.to_string());
                oid = workdir_oid.clone();

                if status_flags.contains(git2::SubmoduleStatus::WD_MODIFIED)
                    || status_flags.contains(git2::SubmoduleStatus::WD_INDEX_MODIFIED)
                    || status_flags.contains(git2::SubmoduleStatus::WD_WD_MODIFIED)
                {
                    status = SubmoduleStatus::Dirty;
                } else if workdir_oid.as_deref() != Some(&registered_oid) {
                    status = SubmoduleStatus::Outdated;
                } else {
                    status = SubmoduleStatus::Clean;
                }
            }

            submodules.push(SubmoduleInfo {
                name,
                path,
                url,
                oid,
                registered_oid,
                status,
            });
        }

        Ok(submodules)
    }

    /// Initialize a submodule (registers it and clones the repo).
    ///
    /// Equivalent to `git submodule init <path>`.
    pub fn init_submodule(&self, path: &str) -> Result<(), GitError> {
        let result = self.git_cmd(&["submodule", "init", path])?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }

    /// Deinitialize a submodule (removes its working tree and config).
    ///
    /// Equivalent to `git submodule deinit [-f] <path>`.
    pub fn deinit_submodule(&self, path: &str, force: bool) -> Result<(), GitError> {
        let mut args = vec!["submodule", "deinit"];
        if force {
            args.push("--force");
        }
        args.push(path);
        let result = self.git_cmd(&args)?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }

    /// Return the absolute path to a submodule's working directory.
    ///
    /// Resolves `<repo_root>/<submodule_path>` to an absolute path.
    pub fn submodule_abs_path(&self, submodule_path: &str) -> Result<String, GitError> {
        let abs = self.path().join(submodule_path);
        if abs.exists() {
            Ok(abs.to_string_lossy().to_string())
        } else {
            Err(GitError::RepoNotFound(format!(
                "Submodule path does not exist: {}",
                abs.display()
            )))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_repo_with_submodule() -> (tempfile::TempDir, tempfile::TempDir) {
        // Create "remote" repo to act as submodule source
        let sub_remote = tempfile::tempdir().unwrap();
        let sub_git = git2::Repository::init(sub_remote.path()).unwrap();
        {
            let mut cfg = sub_git.config().unwrap();
            cfg.set_str("user.name", "Test").unwrap();
            cfg.set_str("user.email", "test@test.com").unwrap();
        }
        std::fs::write(sub_remote.path().join("sub.txt"), "sub content").unwrap();
        {
            let mut index = sub_git.index().unwrap();
            index.add_path(std::path::Path::new("sub.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = sub_git.find_tree(tree_id).unwrap();
            let sig = sub_git.signature().unwrap();
            sub_git
                .commit(Some("HEAD"), &sig, &sig, "init sub", &tree, &[])
                .unwrap();
        }

        // Create superproject
        let super_dir = tempfile::tempdir().unwrap();
        let super_git = git2::Repository::init(super_dir.path()).unwrap();
        {
            let mut cfg = super_git.config().unwrap();
            cfg.set_str("user.name", "Test").unwrap();
            cfg.set_str("user.email", "test@test.com").unwrap();
        }
        std::fs::write(super_dir.path().join("main.txt"), "main content").unwrap();
        {
            let mut index = super_git.index().unwrap();
            index.add_path(std::path::Path::new("main.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = super_git.find_tree(tree_id).unwrap();
            let sig = super_git.signature().unwrap();
            super_git
                .commit(Some("HEAD"), &sig, &sig, "init super", &tree, &[])
                .unwrap();
        }

        (super_dir, sub_remote)
    }

    #[test]
    fn test_submodule_status_serialization() {
        let json = serde_json::to_string(&SubmoduleStatus::Clean).unwrap();
        assert_eq!(json, "\"clean\"");
        let json = serde_json::to_string(&SubmoduleStatus::Uninitialized).unwrap();
        assert_eq!(json, "\"uninitialized\"");
        let json = serde_json::to_string(&SubmoduleStatus::Outdated).unwrap();
        assert_eq!(json, "\"outdated\"");
        let json = serde_json::to_string(&SubmoduleStatus::Dirty).unwrap();
        assert_eq!(json, "\"dirty\"");
    }

    #[test]
    fn test_list_submodules_empty() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();
        let subs = repo.list_submodules().unwrap();
        assert!(subs.is_empty());
    }

    #[test]
    fn test_submodule_abs_path_not_found() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();
        let result = repo.submodule_abs_path("nonexistent");
        assert!(result.is_err());
    }
}
