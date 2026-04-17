//! MR/PR checkout for GitHub (delegates to `gh pr checkout`).
//!
//! Keeps the stdout parser [`parse_gh_checkout_output`] colocated with the
//! feature that invokes it.

use forge_provider::{CheckoutResult, ForgeError};

use super::GitHubCli;

impl GitHubCli {
    pub(super) fn checkout_mr_pr_impl(&self, number: u64) -> Result<CheckoutResult, ForgeError> {
        let n = number.to_string();
        let stdout = self.run(&["pr", "checkout", &n])?;
        Ok(parse_gh_checkout_output(&stdout))
    }
}

/// Parse stdout from `gh pr checkout N` into a [`CheckoutResult`].
///
/// Pure heuristic parser — looks for the branch name in git's standard
/// "Switched to ..." lines and detects fork remote additions in `gh`'s
/// `Added remote '<name>'` line.
fn parse_gh_checkout_output(stdout: &str) -> CheckoutResult {
    let mut branch_name = String::new();
    let mut remote_added: Option<String> = None;
    for line in stdout.lines() {
        let after_new = line.strip_prefix("Switched to a new branch '");
        let after_existing = line.strip_prefix("Switched to branch '");
        if let Some(rest) = after_new.or(after_existing)
            && let Some(end) = rest.find('\'')
        {
            branch_name = rest[..end].to_string();
        }
        if let Some(rest) = line.strip_prefix("Added remote '")
            && let Some(end) = rest.find('\'')
        {
            remote_added = Some(rest[..end].to_string());
        }
    }
    let is_fork = remote_added.is_some();
    CheckoutResult {
        branch_name,
        is_fork,
        remote_added,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gh_checkout_simple_output() {
        let out = "From github.com:foo/bar\n   abc..def  pull/42/head -> origin/pull/42/head\nSwitched to a new branch 'feature-x'\n";
        let r = parse_gh_checkout_output(out);
        assert_eq!(r.branch_name, "feature-x");
        assert!(!r.is_fork);
        assert_eq!(r.remote_added, None);
    }

    #[test]
    fn parse_gh_checkout_existing_branch() {
        let out = "Switched to branch 'existing-feature'\n";
        let r = parse_gh_checkout_output(out);
        assert_eq!(r.branch_name, "existing-feature");
        assert!(!r.is_fork);
    }

    #[test]
    fn parse_gh_checkout_fork_adds_remote() {
        let out = "Added remote 'contributor'\nFrom github.com:contributor/bar\nSwitched to a new branch 'contributor-feature'\n";
        let r = parse_gh_checkout_output(out);
        assert_eq!(r.branch_name, "contributor-feature");
        assert!(r.is_fork);
        assert_eq!(r.remote_added.as_deref(), Some("contributor"));
    }

    #[test]
    fn parse_gh_checkout_empty_stdout_yields_empty_branch() {
        let r = parse_gh_checkout_output("");
        assert_eq!(r.branch_name, "");
        assert!(!r.is_fork);
        assert!(r.remote_added.is_none());
    }
}
