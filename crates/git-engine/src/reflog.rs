//! Reflog access — reads HEAD's reference log via libgit2.
//!
//! Each entry records an action that moved HEAD (commit, checkout, rebase, etc.)
//! along with the old and new OIDs, a message, and author information.

use serde::Serialize;

use crate::error::GitError;
use crate::repository::Repository;

/// A single entry from the HEAD reflog.
#[derive(Debug, Clone, Serialize)]
pub struct ReflogEntry {
    /// The new OID after this action.
    pub oid: String,
    /// The previous OID before this action.
    pub prev_oid: String,
    /// The action type extracted from the reflog message (e.g. "commit", "checkout", "rebase").
    pub action: String,
    /// The summary/message from the reflog entry.
    pub summary: String,
    /// The author name who performed the action.
    pub author: String,
    /// The author email.
    pub email: String,
    /// Unix timestamp (seconds) of when this action occurred.
    pub timestamp: i64,
}

impl Repository {
    /// Read the HEAD reflog, returning up to `limit` entries (most recent first).
    ///
    /// Uses libgit2's `Reflog` API to iterate over the reflog for `"HEAD"`.
    /// Each entry's message is parsed to extract the action type (the word before
    /// the first colon or space).
    pub fn get_reflog(&self, limit: usize) -> Result<Vec<ReflogEntry>, GitError> {
        let reflog = self.inner().reflog("HEAD")?;
        let count = reflog.len().min(limit);
        let mut entries = Vec::with_capacity(count);

        for i in 0..count {
            let entry = reflog
                .get(i)
                .ok_or_else(|| GitError::Git(git2::Error::from_str("reflog entry missing")))?;

            let new_oid = entry.id_new().to_string();
            let old_oid = entry.id_old().to_string();
            let message = entry.message().unwrap_or("").to_string();
            let sig = entry.committer();
            let author = sig.name().unwrap_or("").to_string();
            let email = sig.email().unwrap_or("").to_string();
            let timestamp = sig.when().seconds();

            // Extract action from message: "checkout: moving from ..." -> "checkout"
            // "commit: fix bug" -> "commit"
            // "rebase (finish): ..." -> "rebase"
            // "pull: Fast-forward" -> "pull"
            let action = parse_reflog_action(&message);
            let summary = parse_reflog_summary(&message);

            entries.push(ReflogEntry {
                oid: new_oid,
                prev_oid: old_oid,
                action,
                summary,
                author,
                email,
                timestamp,
            });
        }

        Ok(entries)
    }
}

/// Extract the action word from a reflog message.
///
/// Examples:
/// - `"commit: fix bug"` -> `"commit"`
/// - `"checkout: moving from main to feature"` -> `"checkout"`
/// - `"rebase (finish): returning to refs/heads/main"` -> `"rebase"`
/// - `"pull: Fast-forward"` -> `"pull"`
/// - `"reset: moving to HEAD~1"` -> `"reset"`
/// - `"merge feature: Fast-forward"` -> `"merge"`
/// - `"commit (amend): updated message"` -> `"commit"`
/// - `"commit (initial): first commit"` -> `"commit"`
fn parse_reflog_action(message: &str) -> String {
    // Take everything before the first colon or space-paren
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }

    // Find the first word (before : or space)
    let end = trimmed.find([':', ' ']).unwrap_or(trimmed.len());
    let action = &trimmed[..end];

    action.to_lowercase()
}

/// Extract the summary from a reflog message (everything after the first ": ").
fn parse_reflog_summary(message: &str) -> String {
    if let Some(idx) = message.find(": ") {
        message[idx + 2..].to_string()
    } else {
        message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reflog_action() {
        assert_eq!(parse_reflog_action("commit: fix bug"), "commit");
        assert_eq!(
            parse_reflog_action("checkout: moving from main to feature"),
            "checkout"
        );
        assert_eq!(
            parse_reflog_action("rebase (finish): returning to refs/heads/main"),
            "rebase"
        );
        assert_eq!(parse_reflog_action("pull: Fast-forward"), "pull");
        assert_eq!(parse_reflog_action("reset: moving to HEAD~1"), "reset");
        assert_eq!(parse_reflog_action("merge feature: Fast-forward"), "merge");
        assert_eq!(
            parse_reflog_action("commit (amend): updated message"),
            "commit"
        );
        assert_eq!(parse_reflog_action(""), "unknown");
    }

    #[test]
    fn test_parse_reflog_summary() {
        assert_eq!(parse_reflog_summary("commit: fix bug"), "fix bug");
        assert_eq!(
            parse_reflog_summary("checkout: moving from main to feature"),
            "moving from main to feature"
        );
        assert_eq!(parse_reflog_summary("no colon here"), "no colon here");
    }
}
