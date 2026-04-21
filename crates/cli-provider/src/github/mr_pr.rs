//! MR/PR CRUD + comments for the GitHub CLI provider.
//!
//! Covers list / get / diff / create / edit / merge / close / approve /
//! request-changes / add-comment / add-inline-comment.

use forge_provider::{
    Comment, CreateMrPrInput, EditMrPrPatch, ForgeError, MergeStrategy, MrPr, MrPrDetail,
    MrPrDiffFile, MrPrFilter, ReviewStatus,
};

use super::{GitHubCli, state_to_gh_str};
use crate::parsers::{GITHUB_FIELDS, parse_github_comment, parse_mr_pr};

impl GitHubCli {
    pub(super) fn list_mr_prs_impl(
        &self,
        filter: MrPrFilter,
        limit: u32,
    ) -> Result<Vec<MrPr>, ForgeError> {
        let args = build_gh_mr_pr_list_args(&filter, limit);
        let argv: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let raw: Vec<serde_json::Value> = self.run_json(&argv)?;
        Ok(raw.iter().map(|i| parse_mr_pr(i, &GITHUB_FIELDS)).collect())
    }

    pub(super) fn get_mr_pr_impl(&self, number: u64) -> Result<MrPrDetail, ForgeError> {
        let num_str = number.to_string();
        let raw: serde_json::Value = self.run_json(&[
            "pr", "view", &num_str,
            "--json",
            "number,title,state,author,headRefName,baseRefName,url,isDraft,labels,reviewRequests,createdAt,updatedAt,body,mergeable,reviewDecision,additions,deletions,changedFiles,comments",
        ])?;

        let summary = parse_mr_pr(&raw, &GITHUB_FIELDS);

        let review_status = match raw["reviewDecision"].as_str().unwrap_or("") {
            "APPROVED" => ReviewStatus::Approved,
            "CHANGES_REQUESTED" => ReviewStatus::ChangesRequested,
            "REVIEW_REQUIRED" => ReviewStatus::Pending,
            _ => ReviewStatus::Pending,
        };

        let comments: Vec<Comment> = raw["comments"]
            .as_array()
            .map(|arr| arr.iter().map(parse_github_comment).collect())
            .unwrap_or_default();

        let mergeable = match raw["mergeable"].as_str().unwrap_or("") {
            "MERGEABLE" => Some(true),
            "CONFLICTING" => Some(false),
            _ => None,
        };

        Ok(MrPrDetail {
            summary,
            body: raw["body"].as_str().unwrap_or("").to_string(),
            comments,
            review_status,
            mergeable,
        })
    }

    pub(super) fn get_mr_pr_diff_impl(&self, number: u64) -> Result<Vec<MrPrDiffFile>, ForgeError> {
        let stdout = self.run(&[
            "api",
            &format!("repos/{{owner}}/{{repo}}/pulls/{number}/files"),
            "--paginate",
        ])?;
        parse_diff_files(&stdout, MAX_DIFF_PAYLOAD_BYTES)
    }

    pub(super) fn create_mr_pr_impl(&self, input: CreateMrPrInput) -> Result<MrPr, ForgeError> {
        let mut args = vec![
            "pr",
            "create",
            "--head",
            &input.source,
            "--base",
            &input.target,
            "--title",
            &input.title,
            "--body",
            &input.body,
        ];
        if input.draft {
            args.push("--draft");
        }
        let labels_str = input.labels.join(",");
        if !input.labels.is_empty() {
            args.extend(["--label", &labels_str]);
        }
        let reviewers_str = input.reviewers.join(",");
        if !input.reviewers.is_empty() {
            args.extend(["--reviewer", &reviewers_str]);
        }
        let url_output = self.run(&args)?;
        let url = url_output.trim();
        let number: u64 = url
            .rsplit('/')
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ForgeError::Cli("could not parse PR number from URL".into()))?;
        self.list_mr_prs_impl(MrPrFilter::default(), 1)
            .and_then(|list| {
                list.into_iter()
                    .find(|p| p.number == number)
                    .ok_or_else(|| ForgeError::Cli("created PR not found in list".into()))
            })
    }

    pub(super) fn edit_mr_pr_impl(
        &self,
        number: u64,
        patch: EditMrPrPatch,
    ) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        let mut args = vec!["pr", "edit", &num_str];
        if let Some(t) = &patch.title {
            args.extend(["--title", t.as_str()]);
        }
        if let Some(b) = &patch.body {
            args.extend(["--body", b.as_str()]);
        }
        self.run(&args)?;
        Ok(())
    }

    pub(super) fn merge_mr_pr_impl(
        &self,
        number: u64,
        strategy: MergeStrategy,
    ) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        let mut args = vec!["pr", "merge", &num_str];
        match strategy {
            MergeStrategy::Merge => args.push("--merge"),
            MergeStrategy::Squash => args.push("--squash"),
            MergeStrategy::Rebase => args.push("--rebase"),
        }
        self.run(&args)?;
        Ok(())
    }

    pub(super) fn close_mr_pr_impl(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["pr", "close", &num_str])?;
        Ok(())
    }

    pub(super) fn approve_mr_pr_impl(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["pr", "review", &num_str, "--approve"])?;
        Ok(())
    }

    pub(super) fn request_changes_impl(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&[
            "pr",
            "review",
            &num_str,
            "--request-changes",
            "--body",
            body,
        ])?;
        Ok(())
    }

    pub(super) fn add_mr_pr_comment_impl(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["pr", "comment", &num_str, "--body", body])?;
        Ok(())
    }

    pub(super) fn add_mr_pr_inline_comment_impl(
        &self,
        number: u64,
        path: &str,
        line: u64,
        body: &str,
    ) -> Result<(), ForgeError> {
        let json_body = serde_json::json!({
            "body": body,
            "path": path,
            "line": line,
            "side": "RIGHT",
        });
        let api_path = format!("repos/{{owner}}/{{repo}}/pulls/{number}/comments");
        self.run_with_stdin(
            &["api", &api_path, "--method", "POST", "--input", "-"],
            &json_body.to_string(),
        )?;
        Ok(())
    }
}

// ─── diff parsing ───────────────────────────────────────────────────────

/// Upper bound on the JSON payload returned by `gh api
/// repos/…/pulls/{n}/files --paginate`.
///
/// GitHub can return a few thousand file entries for large PRs (e.g.
/// vendored dependency bumps), which has caused multi-minute hangs in
/// `serde_json::from_str` on the Rust side. 50 MB is well above every
/// real-world PR we've observed while keeping worst-case parse time
/// bounded.
pub(crate) const MAX_DIFF_PAYLOAD_BYTES: usize = 50 * 1024 * 1024;

/// Parse stdout from `gh api repos/…/pulls/{n}/files --paginate` into a
/// list of [`MrPrDiffFile`]s.
///
/// Short-circuits with `ForgeError::Cli("diff payload too large …")` if
/// the payload exceeds `max_bytes`, so the UI surfaces a clear error
/// rather than sitting on an unbounded parse. `max_bytes` is passed in
/// (instead of always using [`MAX_DIFF_PAYLOAD_BYTES`]) so tests can
/// exercise the size guard with tiny fixtures.
pub(crate) fn parse_diff_files(
    stdout: &str,
    max_bytes: usize,
) -> Result<Vec<MrPrDiffFile>, ForgeError> {
    if stdout.len() > max_bytes {
        return Err(ForgeError::Cli(format!(
            "diff payload too large ({} bytes, cap {max_bytes})",
            stdout.len()
        )));
    }
    let files: Vec<serde_json::Value> =
        serde_json::from_str(stdout).map_err(|e| ForgeError::Cli(e.to_string()))?;
    Ok(files
        .iter()
        .map(|f| MrPrDiffFile {
            path: f["filename"].as_str().unwrap_or("").to_string(),
            old_path: f["previous_filename"].as_str().map(|s| s.to_string()),
            status: f["status"].as_str().unwrap_or("modified").to_string(),
            additions: f["additions"].as_u64().unwrap_or(0),
            deletions: f["deletions"].as_u64().unwrap_or(0),
            patch: f["patch"].as_str().map(|s| s.to_string()),
        })
        .collect())
}

// ─── argv builders ──────────────────────────────────────────────────────

/// Build argv for `gh pr list` from an [`MrPrFilter`] + limit.
///
/// Extracted so the CLI-flag layout can be unit-tested without spawning
/// `gh`. The returned vector always includes `--json` (fixed field list),
/// `--limit`, and — when the corresponding filter field is set — one of
/// `--state`, `--author`, `--label`, `--search`.
pub(crate) fn build_gh_mr_pr_list_args(filter: &MrPrFilter, limit: u32) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "pr".into(),
        "list".into(),
        "--json".into(),
        "number,title,state,author,headRefName,baseRefName,url,isDraft,labels,reviewRequests,createdAt,updatedAt,additions,deletions,changedFiles".into(),
        "--limit".into(),
        limit.to_string(),
    ];
    // `gh pr list` defaults to `--state open`. Pass `--state all` when the
    // caller hasn't specified a state so closed + merged PRs show up too.
    args.push("--state".into());
    args.push(match filter.state {
        Some(s) => state_to_gh_str(s).into(),
        None => "all".into(),
    });
    if let Some(a) = &filter.author {
        args.push("--author".into());
        args.push(a.clone());
    }
    if let Some(l) = &filter.label {
        args.push("--label".into());
        args.push(l.clone());
    }
    if let Some(t) = &filter.text {
        args.push("--search".into());
        args.push(t.clone());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_provider::{ForgeError, MrPrState};

    #[test]
    fn parse_diff_files_rejects_payload_above_cap() {
        // Simulate an oversized `gh api .../files --paginate` response: a
        // valid JSON array whose serialized form exceeds the cap we pass
        // in. `parse_diff_files` must bail with `ForgeError::Cli` carrying
        // the phrase "too large" rather than attempt to deserialize.
        let big_payload = format!("[{}]", "\"x\",".repeat(100).trim_end_matches(','));
        let cap = 16;
        assert!(big_payload.len() > cap);
        let err = parse_diff_files(&big_payload, cap).expect_err("must reject oversized payload");
        match err {
            ForgeError::Cli(msg) => assert!(
                msg.contains("too large"),
                "error message should mention 'too large', got: {msg}"
            ),
            other => panic!("expected ForgeError::Cli, got {other:?}"),
        }
    }

    #[test]
    fn parse_diff_files_parses_small_payload() {
        let json = r#"[
            {
                "filename": "src/lib.rs",
                "status": "modified",
                "additions": 5,
                "deletions": 2,
                "patch": "@@ -1 +1 @@"
            }
        ]"#;
        let files = parse_diff_files(json, 10 * 1024).expect("must parse");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/lib.rs");
        assert_eq!(files[0].status, "modified");
        assert_eq!(files[0].additions, 5);
        assert_eq!(files[0].deletions, 2);
        assert_eq!(files[0].patch.as_deref(), Some("@@ -1 +1 @@"));
        assert!(files[0].old_path.is_none());
    }

    #[test]
    fn parse_diff_files_captures_previous_filename_for_renames() {
        let json = r#"[
            {
                "filename": "new.rs",
                "previous_filename": "old.rs",
                "status": "renamed",
                "additions": 0,
                "deletions": 0
            }
        ]"#;
        let files = parse_diff_files(json, 10 * 1024).expect("must parse");
        assert_eq!(files[0].old_path.as_deref(), Some("old.rs"));
        assert!(files[0].patch.is_none());
    }

    #[test]
    fn parse_diff_files_rejects_invalid_json() {
        let err = parse_diff_files("{not json}", 10 * 1024).expect_err("must fail");
        assert!(matches!(err, ForgeError::Cli(_)));
    }

    #[test]
    fn build_gh_mr_pr_list_args_default_requests_all_states() {
        let f = MrPrFilter::default();
        let args = build_gh_mr_pr_list_args(&f, 30);
        assert!(args.contains(&"--limit".to_string()));
        assert!(args.contains(&"30".to_string()));
        // Default must include closed + merged PRs (gh's default is open-only).
        assert!(args.windows(2).any(|w| w == ["--state", "all"]));
        assert!(!args.contains(&"--author".to_string()));
        assert!(!args.contains(&"--label".to_string()));
        assert!(!args.contains(&"--search".to_string()));
    }

    #[test]
    fn build_gh_mr_pr_list_args_with_state() {
        let f = MrPrFilter {
            state: Some(MrPrState::Open),
            ..Default::default()
        };
        let args = build_gh_mr_pr_list_args(&f, 30);
        assert!(args.windows(2).any(|w| w == ["--state", "open"]));
    }

    #[test]
    fn build_gh_mr_pr_list_args_pushes_author_label_text() {
        let f = MrPrFilter {
            state: None,
            author: Some("alice".into()),
            label: Some("bug".into()),
            text: Some("flaky test".into()),
        };
        let args = build_gh_mr_pr_list_args(&f, 25);
        assert!(args.windows(2).any(|w| w == ["--author", "alice"]));
        assert!(args.windows(2).any(|w| w == ["--label", "bug"]));
        assert!(args.windows(2).any(|w| w == ["--search", "flaky test"]));
        // --state defaults to "all" when the filter doesn't set it.
        assert!(args.windows(2).any(|w| w == ["--state", "all"]));
    }
}
