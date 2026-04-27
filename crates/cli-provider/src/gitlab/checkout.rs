//! MR checkout for GitLab (delegates to `glab mr checkout`).
//!
//! Keeps the stdout parser [`parse_glab_checkout_output`] colocated with the
//! feature that invokes it.

use forge_provider::{CheckoutResult, ForgeError};

use super::GitLabCli;

impl GitLabCli {
    pub(super) fn checkout_mr_pr_impl(&self, number: u64) -> Result<CheckoutResult, ForgeError> {
        let n = number.to_string();
        let stdout = self.run(&["mr", "checkout", &n])?;
        Ok(parse_glab_checkout_output(&stdout))
    }
}

/// Parse stdout from `glab mr checkout N` into a [`CheckoutResult`].
///
/// Pure heuristic parser — looks for glab's "Checking out branch 'X' from
/// merge request !N" line and its "Adding remote <name>" line.
fn parse_glab_checkout_output(stdout: &str) -> CheckoutResult {
    let mut branch_name = String::new();
    let mut remote_added: Option<String> = None;
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("Checking out branch '")
            && let Some(end) = rest.find('\'')
        {
            branch_name = rest[..end].to_string();
        }
        if let Some(rest) = line.strip_prefix("Adding remote ") {
            remote_added = Some(rest.trim().to_string());
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
    fn parse_glab_checkout_simple_output() {
        let out = "Checking out branch 'feature-y' from merge request !7\n";
        let r = parse_glab_checkout_output(out);
        assert_eq!(r.branch_name, "feature-y");
        assert!(!r.is_fork);
        assert!(r.remote_added.is_none());
    }

    #[test]
    fn parse_glab_checkout_fork_adds_remote() {
        let out =
            "Adding remote fork-user\nChecking out branch 'fork-feature' from merge request !7\n";
        let r = parse_glab_checkout_output(out);
        assert_eq!(r.branch_name, "fork-feature");
        assert!(r.is_fork);
        assert_eq!(r.remote_added.as_deref(), Some("fork-user"));
    }

    #[test]
    fn parse_glab_checkout_empty_stdout() {
        let r = parse_glab_checkout_output("");
        assert_eq!(r.branch_name, "");
        assert!(!r.is_fork);
        assert!(r.remote_added.is_none());
    }
}
