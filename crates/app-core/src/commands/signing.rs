//! Commit-signing diagnostics: config status, signature presence, lazy
//! verification, and the "Test signing" action.
//!
//! These are all read-only/diagnostic — none mutate the repository, so they
//! carry no [`MutationGuard`][mutation_events::MutationGuard]. The signing of
//! actual commits happens in `commit.rs` / the git-engine commit paths.

use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::state::AppState;

/// Return the active repo's effective signing status for the commit box and
/// settings: `{ enabled, format, key_present }`. `key_present` is diagnostic
/// only and never blocks committing.
#[tauri::command]
pub fn get_signing_config(state: State<'_, AppState>) -> Result<git_engine::SigningStatus, String> {
    with_active_repo(&state, |repo| {
        repo.signing_status().map_err(|e| e.to_string())
    })
}

/// Presence (not validity) of a commit's embedded signature via `git2`.
/// Cheap enough to call for the commit open in the detail pane.
#[tauri::command]
pub fn get_commit_signature(
    oid: String,
    state: State<'_, AppState>,
) -> Result<git_engine::CommitSignature, String> {
    with_active_repo(&state, |repo| {
        repo.commit_signature(&oid).map_err(|e| e.to_string())
    })
}

/// Lazily verify a single commit's signature by shelling to
/// `git verify-commit`. Runs off the async runtime (subprocess) and is meant
/// to be called on demand for the commit open in the detail pane.
#[tauri::command]
#[instrument(skip(state), name = "cmd::signing::verify")]
pub async fn verify_commit_signature(
    oid: String,
    state: State<'_, AppState>,
) -> Result<git_engine::SignatureVerification, String> {
    let repo_path = get_active_project_path(&state)?;
    run_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.verify_commit_signature(&oid)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Exercise the user's signing config end-to-end by signing a throwaway commit
/// in a temp repo. Reports success or the exact git/gpg/ssh stderr. Shells out,
/// so it runs off the async runtime.
#[tauri::command]
#[instrument(skip(state), name = "cmd::signing::test")]
pub async fn test_signing(
    state: State<'_, AppState>,
) -> Result<git_engine::SigningTestResult, String> {
    let repo_path = get_active_project_path(&state)?;
    run_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.test_signing().map_err(|e| e.to_string())
    })
    .await
}
