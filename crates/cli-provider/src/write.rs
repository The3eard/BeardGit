//! MR/PR write operations: create, edit, merge, close.

use crate::CliProvider;
use crate::error::CliError;
use crate::types::{MergeStrategy, MrPr};
use provider::ProviderKind;

#[allow(clippy::too_many_arguments)]
impl CliProvider {
    /// Create a new MR/PR.
    ///
    /// Creates a merge request (GitLab) or pull request (GitHub) from `source`
    /// to `target` branch with the given metadata. Returns the newly created
    /// MR/PR summary.
    pub fn create_mr_pr(
        &self,
        source: &str,
        target: &str,
        title: &str,
        body: &str,
        draft: bool,
        labels: &[String],
        reviewers: &[String],
    ) -> Result<MrPr, CliError> {
        match self.kind {
            ProviderKind::GitHub => {
                self.create_github_pr(source, target, title, body, draft, labels, reviewers)
            }
            ProviderKind::GitLab => {
                self.create_gitlab_mr(source, target, title, body, draft, labels, reviewers)
            }
        }
    }

    /// Create a GitHub pull request.
    fn create_github_pr(
        &self,
        source: &str,
        target: &str,
        title: &str,
        body: &str,
        draft: bool,
        labels: &[String],
        reviewers: &[String],
    ) -> Result<MrPr, CliError> {
        let mut args = vec![
            "pr", "create", "--head", source, "--base", target, "--title", title, "--body", body,
        ];
        if draft {
            args.push("--draft");
        }
        let labels_str = labels.join(",");
        if !labels.is_empty() {
            args.extend(["--label", &labels_str]);
        }
        let reviewers_str = reviewers.join(",");
        if !reviewers.is_empty() {
            args.extend(["--reviewer", &reviewers_str]);
        }
        // gh pr create returns the URL on stdout
        let url_output = self.run(&args)?;
        let url = url_output.trim();
        // Extract PR number from URL (last path segment)
        let number: u64 = url
            .rsplit('/')
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| CliError::JsonError("Could not parse PR number from URL".to_string()))?;
        self.list_mr_prs(None, 1).and_then(|list| {
            list.into_iter()
                .find(|p| p.number == number)
                .ok_or_else(|| CliError::CommandFailed("Created PR not found in list".to_string()))
        })
    }

    /// Create a GitLab merge request.
    fn create_gitlab_mr(
        &self,
        source: &str,
        target: &str,
        title: &str,
        body: &str,
        draft: bool,
        labels: &[String],
        reviewers: &[String],
    ) -> Result<MrPr, CliError> {
        let mut args = vec![
            "mr",
            "create",
            "--source-branch",
            source,
            "--target-branch",
            target,
            "--title",
            title,
            "--description",
            body,
            "--no-editor",
        ];
        if draft {
            args.push("--draft");
        }
        let labels_str = labels.join(",");
        if !labels.is_empty() {
            args.extend(["--label", &labels_str]);
        }
        // glab expects --reviewer per user
        let reviewer_refs: Vec<&str> = reviewers.iter().map(|r| r.as_str()).collect();
        for r in &reviewer_refs {
            args.extend(["--reviewer", r]);
        }
        let output = self.run(&args)?;
        // Parse MR URL from output and extract number (last numeric path segment)
        let number = output
            .lines()
            .find_map(|line| line.rsplit('/').next().and_then(|s| s.parse::<u64>().ok()))
            .ok_or_else(|| CliError::JsonError("Could not parse MR number".to_string()))?;
        // Fetch the created MR for structured data
        self.get_mr_pr_detail(number).map(|d| d.summary)
    }

    /// Edit an existing MR/PR's title and description.
    ///
    /// At least one of `title` or `body` should be provided, otherwise
    /// the command is a no-op.
    pub fn edit_mr_pr(
        &self,
        number: u64,
        title: Option<&str>,
        body: Option<&str>,
    ) -> Result<(), CliError> {
        let num_str = number.to_string();
        match self.kind {
            ProviderKind::GitHub => {
                let mut args = vec!["pr", "edit", &num_str];
                if let Some(t) = title {
                    args.extend(["--title", t]);
                }
                if let Some(b) = body {
                    args.extend(["--body", b]);
                }
                self.run(&args)?;
                Ok(())
            }
            ProviderKind::GitLab => {
                let mut args = vec!["mr", "update", &num_str];
                if let Some(t) = title {
                    args.extend(["--title", t]);
                }
                if let Some(b) = body {
                    args.extend(["--description", b]);
                }
                self.run(&args)?;
                Ok(())
            }
        }
    }

    /// Merge a MR/PR with the given strategy.
    ///
    /// Supported strategies: merge (default), squash, rebase.
    /// GitLab CLI does not support `--rebase`; rebase strategy is configured
    /// on the MR itself.
    pub fn merge_mr_pr(&self, number: u64, strategy: MergeStrategy) -> Result<(), CliError> {
        let num_str = number.to_string();
        match self.kind {
            ProviderKind::GitHub => {
                let mut args = vec!["pr", "merge", &num_str];
                match strategy {
                    MergeStrategy::Merge => args.push("--merge"),
                    MergeStrategy::Squash => args.push("--squash"),
                    MergeStrategy::Rebase => args.push("--rebase"),
                }
                self.run(&args)?;
                Ok(())
            }
            ProviderKind::GitLab => {
                let mut args = vec!["mr", "merge", &num_str];
                if strategy == MergeStrategy::Squash {
                    args.push("--squash");
                }
                // GitLab CLI doesn't support --rebase flag; rebase is configured on the MR
                self.run(&args)?;
                Ok(())
            }
        }
    }

    /// Close a MR/PR without merging.
    pub fn close_mr_pr(&self, number: u64) -> Result<(), CliError> {
        let num_str = number.to_string();
        match self.kind {
            ProviderKind::GitHub => {
                self.run(&["pr", "close", &num_str])?;
                Ok(())
            }
            ProviderKind::GitLab => {
                self.run(&["mr", "close", &num_str])?;
                Ok(())
            }
        }
    }
}
