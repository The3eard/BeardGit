//! Submodule management — list, init, update, and deinit git submodules.
//!
//! Uses libgit2's Submodule API for read operations (listing, status) and
//! the git CLI for write operations (init, update, deinit) since those
//! may involve network fetches and complex state changes.

use serde::Serialize;
use tracing::instrument;

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

                // The three `WD_*` flags mean different things, and conflating
                // them made `Outdated` unreachable: libgit2 raises
                // `WD_MODIFIED` for "the submodule's HEAD is not the commit the
                // superproject records", which is precisely outdated — so a
                // moved HEAD was reported as dirty and the OID comparison
                // below never ran. Dirty is about the submodule's own index and
                // working tree, and stays first: uncommitted work is the more
                // urgent thing to surface when both are true.
                if status_flags.intersects(
                    git2::SubmoduleStatus::WD_INDEX_MODIFIED
                        | git2::SubmoduleStatus::WD_WD_MODIFIED,
                ) {
                    status = SubmoduleStatus::Dirty;
                } else if status_flags.contains(git2::SubmoduleStatus::WD_MODIFIED)
                    || workdir_oid.as_deref() != Some(&registered_oid)
                {
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
    #[instrument(skip(self), fields(path = %path))]
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
    #[instrument(skip(self), fields(path = %path))]
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

    /// Add a new submodule to the repository.
    ///
    /// Equivalent to `git submodule add <url> <path>`.
    // `url` can embed credentials — log only the destination path.
    #[instrument(skip_all, fields(path = %path))]
    pub fn add_submodule(&self, url: &str, path: &str) -> Result<(), GitError> {
        let result = self.git_cmd(&["submodule", "add", url, path])?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }

    /// Remove a submodule completely (deinit, remove from index, delete directory).
    ///
    /// Equivalent to `git submodule deinit -f <path> && git rm -f <path>`.
    #[instrument(skip(self), fields(path = %path))]
    pub fn remove_submodule(&self, path: &str) -> Result<(), GitError> {
        // Deinit first
        let result = self.git_cmd(&["submodule", "deinit", "--force", path])?;
        if !result.success {
            return Err(GitError::CliError(result.stderr));
        }
        // Remove from index and working tree
        let result = self.git_cmd(&["rm", "-f", path])?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }

    /// Return the absolute path to a submodule's working directory.
    ///
    /// Resolves `<repo_root>/<submodule_path>` and refuses a result that falls
    /// outside the repository. The value is handed straight to `open_project`,
    /// so a `..` in the argument would open a project tab on an arbitrary
    /// directory.
    pub fn submodule_abs_path(&self, submodule_path: &str) -> Result<String, GitError> {
        let joined = self.path().join(submodule_path);
        let abs = joined.canonicalize().map_err(|_| {
            GitError::RepoNotFound(format!(
                "Submodule path does not exist: {}",
                joined.display()
            ))
        })?;
        // Both sides canonicalised: on macOS the repo root itself is often
        // reached through a symlink (`/var` → `/private/var`), and a resolved
        // path never starts with an unresolved root.
        let root = self.path().canonicalize().map_err(GitError::Io)?;
        if !abs.starts_with(&root) {
            return Err(GitError::RepoNotFound(format!(
                "Submodule path escapes the repository: {}",
                abs.display()
            )));
        }
        Ok(abs.to_string_lossy().to_string())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Path of the one submodule every fixture below registers.
    const SUB_PATH: &str = "libs/sub";

    /// Run `git` in `dir` and assert it succeeded.
    ///
    /// `protocol.file.allow=always` is not optional: cloning a submodule from
    /// a local path uses the `file` transport, which git refuses for
    /// submodules by default (CVE-2022-39253). Every fixture invocation that
    /// can reach a clone needs it.
    ///
    /// `-c` identity rather than the ambient config so the fixture doesn't
    /// depend on the developer's `~/.gitconfig`.
    fn git(dir: &std::path::Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args([
                "-c",
                "protocol.file.allow=always",
                "-c",
                "user.name=Test User",
                "-c",
                "user.email=test@example.com",
                "-c",
                "core.autocrlf=false",
            ])
            .args(args)
            .current_dir(dir)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Commit `content` to `file` in the repo at `dir`.
    fn commit_file(dir: &std::path::Path, file: &str, content: &str, message: &str) {
        std::fs::write(dir.join(file), content).unwrap();
        git(dir, &["add", file]);
        git(dir, &["commit", "-m", message]);
    }

    /// A superproject carrying one **real** submodule at [`SUB_PATH`], checked
    /// out at the commit the superproject records.
    ///
    /// Returns `(superproject, submodule source)`. Both `TempDir`s must stay
    /// alive for the duration of a test: the second one is the submodule's
    /// `origin`, and dropping it deletes the directory `.gitmodules` points at.
    fn create_test_repo_with_submodule() -> (tempfile::TempDir, tempfile::TempDir) {
        let sub_remote = tempfile::tempdir().unwrap();
        git(sub_remote.path(), &["init", "-q", "."]);
        commit_file(sub_remote.path(), "sub.txt", "sub content", "init sub");

        let super_dir = tempfile::tempdir().unwrap();
        git(super_dir.path(), &["init", "-q", "."]);
        commit_file(super_dir.path(), "main.txt", "main content", "init super");

        let sub_url = sub_remote.path().to_string_lossy().to_string();
        git(
            super_dir.path(),
            &["submodule", "add", "-q", "--", &sub_url, SUB_PATH],
        );
        git(super_dir.path(), &["commit", "-m", "add submodule"]);

        (super_dir, sub_remote)
    }

    /// The single submodule of a fixture repo.
    fn only_submodule(repo: &Repository) -> SubmoduleInfo {
        let mut subs = repo.list_submodules().expect("list_submodules");
        assert_eq!(subs.len(), 1, "fixture registers exactly one submodule");
        subs.remove(0)
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
    fn list_submodules_on_repo_without_submodules_is_empty() {
        let (_tmp, path) = crate::test_support::create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        assert!(repo.list_submodules().unwrap().is_empty());
    }

    #[test]
    fn list_submodules_reports_a_checked_out_submodule_as_clean() {
        let (super_dir, sub_remote) = create_test_repo_with_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();

        let sub = only_submodule(&repo);
        assert_eq!(sub.name, SUB_PATH);
        assert_eq!(sub.path, SUB_PATH);
        assert_eq!(sub.url, sub_remote.path().to_string_lossy());
        assert_eq!(
            sub.oid.as_deref(),
            Some(sub.registered_oid.as_str()),
            "a freshly added submodule sits at the recorded commit"
        );
        assert_eq!(sub.status, SubmoduleStatus::Clean);
    }

    #[test]
    fn list_submodules_reports_a_modified_working_tree_as_dirty() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        std::fs::write(super_dir.path().join(SUB_PATH).join("sub.txt"), "edited").unwrap();

        let repo = Repository::open(super_dir.path()).unwrap();
        let sub = only_submodule(&repo);

        assert_eq!(sub.status, SubmoduleStatus::Dirty);
        assert_eq!(
            sub.oid.as_deref(),
            Some(sub.registered_oid.as_str()),
            "an uncommitted edit does not move the submodule's HEAD"
        );
    }

    #[test]
    fn list_submodules_reports_a_moved_head_as_outdated() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        let sub_wd = super_dir.path().join(SUB_PATH);
        // Commit inside the submodule: its HEAD moves, the gitlink the
        // superproject recorded stays behind. This is the "outdated" case —
        // and the one that separates it from "dirty", since the submodule's
        // own working tree is clean afterwards.
        commit_file(&sub_wd, "sub.txt", "second revision", "move sub HEAD");

        let repo = Repository::open(super_dir.path()).unwrap();
        let sub = only_submodule(&repo);

        assert_ne!(
            sub.oid.as_deref(),
            Some(sub.registered_oid.as_str()),
            "the fixture must actually move the submodule's HEAD"
        );
        assert_eq!(sub.status, SubmoduleStatus::Outdated);
    }

    #[test]
    fn list_submodules_reports_a_deinitialized_submodule_as_uninitialized() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        git(super_dir.path(), &["submodule", "deinit", "-f", SUB_PATH]);

        let repo = Repository::open(super_dir.path()).unwrap();
        let sub = only_submodule(&repo);

        assert_eq!(sub.status, SubmoduleStatus::Uninitialized);
        assert_eq!(sub.oid, None);
        assert!(
            !sub.registered_oid.is_empty(),
            "the superproject still records the commit it expects"
        );
    }

    #[test]
    fn test_submodule_abs_path_not_found() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();
        let result = repo.submodule_abs_path("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn submodule_abs_path_resolves_a_registered_submodule() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();

        let abs = repo.submodule_abs_path(SUB_PATH).unwrap();

        assert!(std::path::Path::new(&abs).join(".git").exists());
    }

    #[test]
    fn submodule_abs_path_rejects_a_path_outside_the_repository() {
        let (super_dir, _sub_remote) = create_test_repo_with_submodule();
        let repo = Repository::open(super_dir.path()).unwrap();

        // The parent of a tempdir exists, so this fails containment rather
        // than existence — the case a bare `exists()` check let through.
        let err = repo.submodule_abs_path("..").expect_err("must be refused");

        assert!(
            matches!(err, GitError::RepoNotFound(ref m) if m.contains("escapes the repository")),
            "unexpected error: {err:?}"
        );
    }
}
