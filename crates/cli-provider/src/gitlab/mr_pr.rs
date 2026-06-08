//! MR/PR CRUD + comments for the GitLab CLI provider.
//!
//! Covers list / get / diff / create / edit / merge / close / approve /
//! request-changes / add-comment / add-inline-comment.

use forge_provider::{
    Comment, CreateMrPrInput, EditMrPrPatch, ForgeError, MergeStrategy, MrPr, MrPrDetail,
    MrPrDiffFile, MrPrFilter, MrPrState, ReviewStatus,
};

use super::GitLabCli;
use crate::parsers::{GITLAB_FIELDS, parse_gitlab_comment, parse_mr_pr};

impl GitLabCli {
    pub(super) fn list_mr_prs_impl(
        &self,
        filter: MrPrFilter,
        limit: u32,
    ) -> Result<Vec<MrPr>, ForgeError> {
        let args = build_glab_mr_pr_list_args(&filter, limit);
        let argv: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let raw: Vec<serde_json::Value> = self.run_json(&argv)?;
        Ok(raw.iter().map(|i| parse_mr_pr(i, &GITLAB_FIELDS)).collect())
    }

    pub(super) fn get_mr_pr_impl(&self, number: u64) -> Result<MrPrDetail, ForgeError> {
        let num_str = number.to_string();
        let raw: serde_json::Value = self.run_json(&["mr", "view", &num_str, "-F", "json"])?;
        let summary = parse_mr_pr(&raw, &GITLAB_FIELDS);

        // Fetch discussions separately to get discussion IDs for resolve
        // support. Each discussion groups one or more notes under a shared
        // `id`, which is the identifier the resolve/unresolve API takes.
        let discussions_path = format!("projects/:id/merge_requests/{number}/discussions");
        let discussions: Vec<serde_json::Value> = self
            .run_json(&["api", &discussions_path])
            .unwrap_or_default();

        let mut comments: Vec<Comment> = Vec::new();
        for disc in &discussions {
            let disc_id = disc["id"].as_str().map(|s| s.to_string());
            if let Some(notes) = disc["notes"].as_array() {
                for note in notes {
                    let mut c = parse_gitlab_comment(note);
                    c.discussion_id = disc_id.clone();
                    comments.push(c);
                }
            }
        }

        // Fallback: if the discussions endpoint failed or returned nothing,
        // fall back to the `notes` field from the MR view — discussion_id
        // stays `None`, resolve buttons will not render but comments still do.
        if comments.is_empty()
            && let Some(arr) = raw["notes"].as_array()
        {
            comments = arr.iter().map(parse_gitlab_comment).collect();
        }

        let merge_status = raw["merge_status"].as_str().unwrap_or("");
        let mergeable = match merge_status {
            "can_be_merged" => Some(true),
            "cannot_be_merged" => Some(false),
            _ => None,
        };
        Ok(MrPrDetail {
            summary,
            body: raw["description"].as_str().unwrap_or("").to_string(),
            comments,
            review_status: ReviewStatus::Pending,
            mergeable,
        })
    }

    pub(super) fn get_mr_pr_diff_impl(&self, number: u64) -> Result<Vec<MrPrDiffFile>, ForgeError> {
        // `--paginate` so an MR touching > 20 files isn't silently truncated
        // (the /diffs endpoint defaults to 20 per page and glab does not
        // auto-paginate). The wall-clock + payload caps mirror the GitHub path
        // so a huge MR can't hang the subprocess or feed an unbounded string
        // into serde_json.
        let stdout = self.run_with_timeout(
            &[
                "api",
                &format!("projects/:id/merge_requests/{number}/diffs"),
                "--paginate",
            ],
            crate::github::DIFF_FETCH_TIMEOUT,
        )?;
        let cap = crate::github::MAX_DIFF_PAYLOAD_BYTES;
        if stdout.len() > cap {
            return Err(ForgeError::Cli(format!(
                "diff payload too large ({} bytes, cap {cap})",
                stdout.len()
            )));
        }
        let raw: Vec<serde_json::Value> =
            serde_json::from_str(&stdout).map_err(|e| ForgeError::Cli(e.to_string()))?;
        Ok(raw
            .iter()
            .map(|f| {
                let patch = f["diff"].as_str().map(|s| s.to_string());
                let (additions, deletions) =
                    patch.as_deref().map(count_patch_changes).unwrap_or((0, 0));
                MrPrDiffFile {
                    path: f["new_path"].as_str().unwrap_or("").to_string(),
                    old_path: Some(f["old_path"].as_str().unwrap_or("").to_string()),
                    status: if f["new_file"].as_bool().unwrap_or(false) {
                        "added".to_string()
                    } else if f["deleted_file"].as_bool().unwrap_or(false) {
                        "deleted".to_string()
                    } else if f["renamed_file"].as_bool().unwrap_or(false) {
                        "renamed".to_string()
                    } else {
                        "modified".to_string()
                    },
                    additions,
                    deletions,
                    patch,
                }
            })
            .collect())
    }

    pub(super) fn create_mr_pr_impl(&self, input: CreateMrPrInput) -> Result<MrPr, ForgeError> {
        let mut args = vec![
            "mr",
            "create",
            "--source-branch",
            &input.source,
            "--target-branch",
            &input.target,
            "--title",
            &input.title,
            "--description",
            &input.body,
            "--no-editor",
        ];
        if input.draft {
            args.push("--draft");
        }
        let labels_str = input.labels.join(",");
        if !input.labels.is_empty() {
            args.extend(["--label", &labels_str]);
        }
        let reviewer_refs: Vec<&str> = input.reviewers.iter().map(|r| r.as_str()).collect();
        for r in &reviewer_refs {
            args.extend(["--reviewer", r]);
        }
        let output = self.run(&args)?;
        let number = output
            .lines()
            .find_map(|line| line.rsplit('/').next().and_then(|s| s.parse::<u64>().ok()))
            .ok_or_else(|| ForgeError::Cli("could not parse MR number".into()))?;
        self.get_mr_pr_impl(number).map(|d| d.summary)
    }

    pub(super) fn edit_mr_pr_impl(
        &self,
        number: u64,
        patch: EditMrPrPatch,
    ) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        let mut args = vec!["mr", "update", &num_str];
        if let Some(t) = &patch.title {
            args.extend(["--title", t.as_str()]);
        }
        if let Some(b) = &patch.body {
            args.extend(["--description", b.as_str()]);
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
        let mut args = vec!["mr", "merge", &num_str];
        if strategy == MergeStrategy::Squash {
            args.push("--squash");
        }
        // glab has no --rebase flag; rebase is configured on the MR itself.
        self.run(&args)?;
        Ok(())
    }

    pub(super) fn close_mr_pr_impl(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["mr", "close", &num_str])?;
        Ok(())
    }

    pub(super) fn approve_mr_pr_impl(&self, number: u64) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["mr", "approve", &num_str])?;
        Ok(())
    }

    pub(super) fn request_changes_impl(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        // GitLab has no "request changes" concept — post body as a comment.
        self.add_mr_pr_comment_impl(number, body)
    }

    pub(super) fn add_mr_pr_comment_impl(&self, number: u64, body: &str) -> Result<(), ForgeError> {
        let num_str = number.to_string();
        self.run(&["mr", "note", &num_str, "--message", body])?;
        Ok(())
    }

    pub(super) fn add_mr_pr_inline_comment_impl(
        &self,
        number: u64,
        path: &str,
        line: u64,
        body: &str,
        base_sha: &str,
        head_sha: &str,
    ) -> Result<(), ForgeError> {
        let json_body = build_gitlab_inline_comment_body(path, line, body, base_sha, head_sha);
        let api_path = format!("projects/:id/merge_requests/{number}/discussions");
        self.run_with_stdin(
            &["api", &api_path, "--method", "POST", "--input", "-"],
            &json_body,
        )?;
        Ok(())
    }

    pub(super) fn reply_to_review_comment_impl(
        &self,
        number: u64,
        discussion_id: &str,
        body: &str,
    ) -> Result<(), ForgeError> {
        let json_body = serde_json::json!({ "body": body }).to_string();
        let api_path =
            format!("projects/:id/merge_requests/{number}/discussions/{discussion_id}/notes");
        self.run_with_stdin(
            &["api", &api_path, "--method", "POST", "--input", "-"],
            &json_body,
        )?;
        Ok(())
    }
}

/// Build the JSON body for a GitLab inline-discussion POST.
///
/// GitLab's `position` object needs the full `diff_refs` trio:
/// `base_sha`, `head_sha`, `start_sha`. For single-ref PRs `start_sha`
/// equals `base_sha`.
pub(super) fn build_gitlab_inline_comment_body(
    path: &str,
    line: u64,
    body: &str,
    base_sha: &str,
    head_sha: &str,
) -> String {
    serde_json::json!({
        "body": body,
        "position": {
            "position_type": "text",
            "new_path": path,
            "new_line": line,
            "base_sha": base_sha,
            "head_sha": head_sha,
            "start_sha": base_sha,
        }
    })
    .to_string()
}

// ─── argv builders ──────────────────────────────────────────────────────

/// Build argv for `glab mr list` from an [`MrPrFilter`] + limit.
///
/// Extracted so the CLI-flag layout can be unit-tested without spawning
/// `glab`. `glab` dropped `--state <value>` in favour of boolean flags
/// (`--all`, `--closed`, `--merged`, default = opened). Match what
/// the installed binary accepts — both 1.46.x (bundled) and 1.92.x
/// (current Homebrew release) use the boolean form. `--author`, `--label`,
/// and `--search` are appended when the corresponding filter field is set.
pub(crate) fn build_glab_mr_pr_list_args(filter: &MrPrFilter, limit: u32) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "mr".into(),
        "list".into(),
        "--per-page".into(),
        limit.to_string(),
    ];
    match filter.state {
        None => args.push("--all".into()),
        Some(MrPrState::Open) => {} // glab's default is "opened"
        Some(MrPrState::Closed) => args.push("--closed".into()),
        Some(MrPrState::Merged) => args.push("--merged".into()),
    }
    args.push("-F".into());
    args.push("json".into());
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

/// Count `+` / `-` lines in a unified diff hunk.
///
/// The GitLab `projects/:id/merge_requests/{n}/diffs` endpoint returns the
/// raw patch text but no per-file additions/deletions counts (unlike
/// `gh pr diff` which does). Parse it locally so the UI can show the
/// `+N -N` badge on each changed file.
///
/// - `+` / `-` at the start of a content line → addition / deletion
/// - `+++` / `---` are file-header markers — skip
/// - `@@ ... @@` hunk headers (and everything else) contribute nothing
fn count_patch_changes(patch: &str) -> (u64, u64) {
    let mut additions: u64 = 0;
    let mut deletions: u64 = 0;
    for line in patch.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        match line.as_bytes().first() {
            Some(b'+') => additions += 1,
            Some(b'-') => deletions += 1,
            _ => {}
        }
    }
    (additions, deletions)
}

#[cfg(test)]
mod tests {
    use super::{build_glab_mr_pr_list_args, count_patch_changes};
    use forge_provider::{MrPrFilter, MrPrState};

    #[test]
    fn build_glab_mr_pr_list_args_default_uses_all_flag() {
        let f = MrPrFilter::default();
        let args = build_glab_mr_pr_list_args(&f, 50);
        assert!(args.contains(&"--all".to_string()));
        assert!(!args.contains(&"--author".to_string()));
        assert!(!args.contains(&"--label".to_string()));
        assert!(!args.contains(&"--search".to_string()));
    }

    #[test]
    fn build_glab_mr_pr_list_args_open_omits_state_flag() {
        let f = MrPrFilter {
            state: Some(MrPrState::Open),
            ..Default::default()
        };
        let args = build_glab_mr_pr_list_args(&f, 50);
        // glab's default = opened, so we push no state flag
        assert!(!args.contains(&"--all".to_string()));
        assert!(!args.contains(&"--closed".to_string()));
        assert!(!args.contains(&"--merged".to_string()));
    }

    #[test]
    fn build_glab_mr_pr_list_args_closed_uses_closed_flag() {
        let f = MrPrFilter {
            state: Some(MrPrState::Closed),
            ..Default::default()
        };
        let args = build_glab_mr_pr_list_args(&f, 50);
        assert!(args.contains(&"--closed".to_string()));
    }

    #[test]
    fn build_glab_mr_pr_list_args_merged_uses_merged_flag() {
        let f = MrPrFilter {
            state: Some(MrPrState::Merged),
            ..Default::default()
        };
        let args = build_glab_mr_pr_list_args(&f, 50);
        assert!(args.contains(&"--merged".to_string()));
    }

    #[test]
    fn build_glab_mr_pr_list_args_pushes_author_label_text() {
        let f = MrPrFilter {
            state: None,
            author: Some("alice".into()),
            label: Some("bug".into()),
            text: Some("flaky test".into()),
        };
        let args = build_glab_mr_pr_list_args(&f, 25);
        assert!(args.windows(2).any(|w| w == ["--author", "alice"]));
        assert!(args.windows(2).any(|w| w == ["--label", "bug"]));
        assert!(args.windows(2).any(|w| w == ["--search", "flaky test"]));
    }

    #[test]
    fn counts_simple_hunk() {
        let patch = "\
@@ -1,3 +1,4 @@\n\
 unchanged\n\
-old line\n\
+new line one\n\
+new line two\n";
        assert_eq!(count_patch_changes(patch), (2, 1));
    }

    #[test]
    fn skips_file_headers() {
        let patch = "\
--- a/foo.txt\n\
+++ b/foo.txt\n\
@@ -0,0 +1,2 @@\n\
+line a\n\
+line b\n";
        assert_eq!(count_patch_changes(patch), (2, 0));
    }

    #[test]
    fn handles_empty_patch() {
        assert_eq!(count_patch_changes(""), (0, 0));
    }

    #[test]
    fn counts_multiple_hunks() {
        let patch = "\
@@ -1,2 +1,1 @@\n\
-old\n\
-older\n\
+new\n\
@@ -10,2 +10,3 @@\n\
 context\n\
+added\n\
-removed\n";
        assert_eq!(count_patch_changes(patch), (2, 3));
    }

    #[test]
    fn gitlab_inline_comment_payload_includes_diff_refs() {
        // The JSON body for a GitLab inline comment must include the full
        // diff_refs trio in `position`, otherwise GitLab rejects with
        // "position" is invalid.
        let base = "bbbb1111";
        let head = "aaaa2222";
        let json = super::build_gitlab_inline_comment_body("src/foo.ts", 42, "nice", base, head);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["body"], "nice");
        assert_eq!(v["position"]["new_path"], "src/foo.ts");
        assert_eq!(v["position"]["new_line"], 42);
        assert_eq!(v["position"]["position_type"], "text");
        assert_eq!(v["position"]["base_sha"], base);
        assert_eq!(v["position"]["head_sha"], head);
        assert_eq!(v["position"]["start_sha"], base); // same-ref PRs use base as start
    }
}
