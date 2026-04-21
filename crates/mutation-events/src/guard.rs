//! `MutationGuard` — scope wrapper that captures a snapshot on enter,
//! re-captures on exit, and emits `project-mutated` if any flag flipped.

use std::path::{Path, PathBuf};

use tauri::AppHandle;
use thiserror::Error;
use tracing::warn;

use crate::{emit_mutation, EmitError, MutationKind, Snapshot, SnapshotError};

/// Errors produced by the guard lifecycle.
#[derive(Debug, Error)]
pub enum GuardError {
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
    #[error(transparent)]
    Emit(#[from] EmitError),
}

/// RAII-style guard. Construct with [`MutationGuard::enter`] before a
/// mutating op; consume with [`MutationGuard::exit`] after.
pub struct MutationGuard {
    before: Snapshot,
    path: PathBuf,
}

impl MutationGuard {
    /// Capture the pre-mutation snapshot. Failure is non-fatal for the
    /// mutation itself — callers that want "fire and forget" can
    /// [`Result::ok`] this and skip the guard.
    pub fn enter(path: &Path) -> Result<Self, GuardError> {
        let before = Snapshot::capture(path)?;
        Ok(Self {
            before,
            path: path.to_path_buf(),
        })
    }

    /// Re-capture and emit. If snapshot re-capture fails, the original
    /// mutation has still succeeded — log and return the error so the
    /// caller can decide whether to surface it.
    pub fn exit(self, kind: MutationKind, app: &AppHandle) -> Result<(), GuardError> {
        let after = Snapshot::capture(&self.path)?;
        let flags = self.before.diff(&after);
        if flags.is_empty() {
            // No observable change — skip emit to keep the bus quiet.
            return Ok(());
        }
        emit_mutation(app, kind, flags, &self.path)
            .inspect_err(|err| warn!(?err, "failed to emit project-mutated"))?;
        Ok(())
    }
}
