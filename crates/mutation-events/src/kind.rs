//! `MutationKind` enum — exhaustive list of mutation sources.

use serde::Serialize;

/// Identifies which AI provider initiated a mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiSource {
    /// Anthropic Claude Code session.
    ClaudeCode,
    /// OpenAI Codex session.
    Codex,
    /// OpenCode session.
    OpenCode,
}

/// Kind of mutation emitted alongside [`crate::MutationFlags`].
///
/// Split into three families:
/// - user/plumbing git ops driven from the UI,
/// - `Ai { source }` for AI background runs that touched the repo,
/// - `External` for watcher-observed CLI / external-editor edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MutationKind {
    /// A new commit was created.
    Commit,
    /// The tip commit was amended.
    Amend,
    /// A commit was reverted via a new commit.
    Revert,
    /// One or more commits were cherry-picked onto the current branch.
    CherryPick,
    /// Local refs were pushed to a remote.
    Push,
    /// Remote changes were fetched and merged/rebased into the current branch.
    Pull,
    /// Remote refs were fetched without updating any local branch.
    Fetch,
    /// Another branch was merged into the current branch.
    Merge,
    /// The current branch was rebased onto another ref.
    Rebase,
    /// `HEAD` was moved via `git reset` (soft/mixed/hard).
    Reset,
    /// `HEAD` was moved to another branch, tag, or commit.
    Checkout,
    /// A local branch was created.
    BranchCreate,
    /// A local branch was deleted.
    BranchDelete,
    /// A local branch was renamed.
    BranchRename,
    /// A tag was created.
    TagCreate,
    /// A tag was deleted.
    TagDelete,
    /// A stash entry was pushed.
    Stash,
    /// A stash entry was popped back into the working tree.
    StashPop,
    /// A stash entry was dropped without applying it.
    StashDrop,
    /// A linked worktree was added.
    WorktreeCreate,
    /// A linked worktree was removed.
    WorktreeRemove,
    /// A remote was added.
    RemoteAdd,
    /// A remote was removed.
    RemoteRemove,
    /// An AI provider session finished a turn that touched the repo.
    Ai {
        /// Which AI provider produced the mutation.
        source: AiSource,
    },
    /// A watcher-observed change from outside the app (CLI, editor, etc.).
    External,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_commit_as_tagged_snake_case() {
        let json = serde_json::to_string(&MutationKind::Commit).unwrap();
        assert_eq!(json, r#"{"type":"commit"}"#);
    }

    #[test]
    fn serializes_ai_variant_with_source() {
        let json = serde_json::to_string(&MutationKind::Ai {
            source: AiSource::ClaudeCode,
        })
        .unwrap();
        assert_eq!(json, r#"{"type":"ai","source":"claude_code"}"#);
    }

    #[test]
    fn serializes_external_as_bare_tag() {
        let json = serde_json::to_string(&MutationKind::External).unwrap();
        assert_eq!(json, r#"{"type":"external"}"#);
    }
}
