//! `MutationKind` enum — exhaustive list of mutation sources.

use serde::Serialize;

/// Identifies which AI provider initiated a mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiSource {
    ClaudeCode,
    Codex,
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
    Commit,
    Amend,
    Revert,
    CherryPick,
    Push,
    Pull,
    Fetch,
    Merge,
    Rebase,
    Reset,
    Checkout,
    BranchCreate,
    BranchDelete,
    BranchRename,
    TagCreate,
    TagDelete,
    Stash,
    StashPop,
    StashDrop,
    WorktreeCreate,
    WorktreeRemove,
    RemoteAdd,
    RemoteRemove,
    Ai { source: AiSource },
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
