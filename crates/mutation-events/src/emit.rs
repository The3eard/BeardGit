//! `emit_mutation` — single entrypoint for broadcasting a mutation
//! event to the frontend. Keeps the event name ("project-mutated") and
//! payload shape centralized.

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use thiserror::Error;

use crate::{MutationFlags, MutationKind};

/// Errors raised while emitting a mutation event.
#[derive(Debug, Error)]
pub enum EmitError {
    #[error("failed to emit project-mutated: {0}")]
    Tauri(#[from] tauri::Error),
}

/// Payload serialized to the frontend. Field names stay snake_case so
/// TS can read them verbatim without a camelCase converter.
#[derive(Debug, Clone, Serialize)]
struct MutationEventPayload<'a> {
    project_path: &'a str,
    kind: MutationKind,
    flags: MutationFlags,
}

/// Fire `project-mutated` with the given kind + flags + project path.
///
/// Caller is responsible for supplying non-empty flags — this function
/// does not filter, because external emitters (watcher) may intentionally
/// emit with `MutationKind::External` and a single flag set to force a
/// refresh of one store family.
pub fn emit_mutation(
    app: &AppHandle,
    kind: MutationKind,
    flags: MutationFlags,
    project_path: &Path,
) -> Result<(), EmitError> {
    let payload = MutationEventPayload {
        project_path: &project_path.display().to_string(),
        kind,
        flags,
    };
    app.emit("project-mutated", payload)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_serializes_with_flat_fields() {
        let flags = MutationFlags {
            head_changed: true,
            refs_changed: true,
            ..Default::default()
        };
        let payload = MutationEventPayload {
            project_path: "/tmp/repo",
            kind: MutationKind::Commit,
            flags,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains(r#""project_path":"/tmp/repo""#));
        assert!(json.contains(r#""kind":{"type":"commit"}"#));
        assert!(json.contains(r#""flags":"#));
        assert!(json.contains(r#""head_changed":true"#));
    }
}
