//! Rename a local branch via the `git branch -m` CLI.
//!
//! libgit2 exposes `Branch::rename`, but it doesn't fix up HEAD when
//! renaming the currently checked-out branch — shelling out to `git`
//! keeps parity with what a user would do from the command line and
//! picks up the symbolic-ref rewrite for free.

use tracing::instrument;

use crate::error::GitError;
use crate::repository::Repository;

impl Repository {
    /// Rename the local branch `old_name` to `new_name`.
    ///
    /// Equivalent to `git branch -m <old> <new>`. Works when `old_name`
    /// is the checked-out branch — git rewrites `HEAD` automatically.
    /// Returns an error when `old_name` does not exist, when `new_name`
    /// collides with an existing branch, or when `new_name` is not a
    /// valid ref name.
    #[instrument(skip(self), fields(old = %old_name, new = %new_name))]
    pub fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<(), GitError> {
        // `--` keeps a branch name beginning with `-` from being parsed as a flag.
        let result = self.git_cmd(&["branch", "-m", "--", old_name, new_name])?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Repository;
    use crate::test_support::create_repo_with_branches;

    #[test]
    fn renames_non_head_branch() {
        let (_tmp, path) = create_repo_with_branches(&["feat/old"]);
        let repo = Repository::open(&path).unwrap();
        repo.rename_branch("feat/old", "feat/new").unwrap();
        let names: Vec<String> = repo
            .branches()
            .unwrap()
            .into_iter()
            .map(|b| b.name)
            .collect();
        assert!(names.contains(&"feat/new".to_string()));
        assert!(!names.contains(&"feat/old".to_string()));
    }

    #[test]
    fn rename_nonexistent_errors() {
        let (_tmp, path) = create_repo_with_branches(&[]);
        let repo = Repository::open(&path).unwrap();
        assert!(repo.rename_branch("nope", "whatever").is_err());
    }

    #[test]
    fn renames_checked_out_branch() {
        let (_tmp, path) = create_repo_with_branches(&["feature"]);
        let repo = Repository::open(&path).unwrap();
        repo.checkout_branch("feature").unwrap();
        repo.rename_branch("feature", "feature-renamed").unwrap();
        assert_eq!(
            repo.get_current_branch().unwrap().as_deref(),
            Some("feature-renamed")
        );
    }
}
