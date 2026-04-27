//! Boolean deltas between two [`crate::Snapshot`] values.
//!
//! The TS side inspects these booleans and dispatches the minimal set
//! of store refreshers — see `src/lib/stores/mutations.ts`.

use serde::Serialize;

/// One flag per downstream store family. All default to `false`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct MutationFlags {
    pub refs_changed: bool,
    pub head_changed: bool,
    pub status_changed: bool,
    pub stashes_changed: bool,
    pub worktrees_changed: bool,
    pub remotes_changed: bool,
}

impl MutationFlags {
    /// `true` when no flag is set — emitters can skip the broadcast.
    pub fn is_empty(&self) -> bool {
        !(self.refs_changed
            || self.head_changed
            || self.status_changed
            || self.stashes_changed
            || self.worktrees_changed
            || self.remotes_changed)
    }

    /// Union — useful when merging two partial diffs.
    pub fn merge(self, other: Self) -> Self {
        Self {
            refs_changed: self.refs_changed || other.refs_changed,
            head_changed: self.head_changed || other.head_changed,
            status_changed: self.status_changed || other.status_changed,
            stashes_changed: self.stashes_changed || other.stashes_changed,
            worktrees_changed: self.worktrees_changed || other.worktrees_changed,
            remotes_changed: self.remotes_changed || other.remotes_changed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        assert!(MutationFlags::default().is_empty());
    }

    #[test]
    fn any_flag_set_is_not_empty() {
        let f = MutationFlags {
            head_changed: true,
            ..Default::default()
        };
        assert!(!f.is_empty());
    }

    #[test]
    fn merge_unions_flags() {
        let a = MutationFlags {
            refs_changed: true,
            ..Default::default()
        };
        let b = MutationFlags {
            status_changed: true,
            ..Default::default()
        };
        let m = a.merge(b);
        assert!(m.refs_changed && m.status_changed);
    }

    #[test]
    fn serializes_camel_case_fields() {
        let f = MutationFlags {
            head_changed: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&f).unwrap();
        assert!(json.contains(r#""head_changed":true"#));
    }
}
