//! Thin Tauri command wrappers for remote repository configuration.
//!
//! The forge logic (GitHub / GitLab CLI calls, JSON parsing, diffing) lives in
//! `cli-provider` behind the [`ForgeRepoConfig`] seam; the shared data types
//! live in `forge-provider`. This module keeps only forge detection (which
//! needs a live `git2` repo) and the 8 `#[tauri::command]` glue functions.
//!
//! ## Provider detection
//!
//! [`detect_forge`] parses a repository's `origin` remote URL and returns the
//! matching [`ForgeKind`]. Non-forge remotes (`bitbucket.org`, plain git
//! servers, file URLs, …) return `None` — the caller renders a graceful "not
//! supported" state instead of erroring.

use cli_provider::command_runner::SystemRunner;
use cli_provider::repo_config::{self, CliForgeRepoConfig, ForgeCliStatus, ForgeRepoConfig};
use forge_provider::{BranchProtection, ForgeKind, Label, RemoteRepoConfig, RemoteRepoConfigPatch};
use git_engine::Repository;
use tauri::State;
use tracing::instrument;

use super::helpers::extract_origin_url;
use crate::ipc_error::IpcError;
use crate::state::AppState;

// ───────────────────────────────────────────────────────────────────────────
// Forge detection (needs a live repo)
// ───────────────────────────────────────────────────────────────────────────

/// Detect which forge backend a repository talks to based on its `origin`
/// remote URL. Returns `None` for non-GitHub/GitLab remotes or repos with no
/// `origin`.
pub fn detect_forge(repo: &Repository) -> Option<ForgeKind> {
    let url = extract_origin_url(repo)?;
    repo_config::detect_forge_from_url(&url)
}

/// Detect the forge kind plus the canonical hostname of a repository's
/// `origin` remote, used to scope `gh`/`glab auth status -h <host>` so
/// multi-instance configs aren't poisoned by an unrelated host's auth failure.
pub fn detect_forge_with_host(repo: &Repository) -> Option<(ForgeKind, String)> {
    let url = extract_origin_url(repo)?;
    let kind = repo_config::detect_forge_from_url(&url)?;
    let host = repo_config::extract_remote_host(&url)?;
    Some((kind, host))
}

// ───────────────────────────────────────────────────────────────────────────
// Tauri commands
// ───────────────────────────────────────────────────────────────────────────

/// Tauri command: load the remote repo configuration for a given repository
/// path. Detects the forge from `origin` and calls the matching loader.
#[tauri::command]
#[instrument(skip(_state), name = "cmd::repo_config::load")]
pub async fn load_remote_repo_config(
    repo_path: String,
    _state: State<'_, AppState>,
) -> Result<RemoteRepoConfig, IpcError> {
    let path = std::path::PathBuf::from(&repo_path);

    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        let cfg = CliForgeRepoConfig::new(SystemRunner::new(), forge);
        cfg.load(&path).map_err(|e| {
            // Surface CLI failures in the log file — these were previously
            // invisible outside the UI, which made "repo settings doesn't
            // load" reports impossible to diagnose after the fact.
            tracing::warn!(forge = ?forge, error = %e, "repo config load failed");
            e.to_string()
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

/// Tauri command: apply a `RemoteRepoConfigPatch` to the remote repo at
/// `repo_path`. On GitLab, loads the current config first so the helper can
/// compute the full `--topics` replacement list; on GitHub that extra CLI call
/// is skipped. Partial failures are returned inside `ApplyResult::failures`.
#[tauri::command]
#[instrument(skip(_state, patch), name = "cmd::repo_config::apply")]
pub async fn apply_remote_repo_config(
    repo_path: String,
    patch: RemoteRepoConfigPatch,
    _state: State<'_, AppState>,
) -> Result<cli_provider::repo_config::ApplyResult, IpcError> {
    let path = std::path::PathBuf::from(&repo_path);

    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        let cfg = CliForgeRepoConfig::new(SystemRunner::new(), forge);

        let current_topics = if matches!(forge, ForgeKind::GitLab) {
            cfg.load(&path).map(|c| c.topics).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok::<_, String>(cfg.apply(&path, &patch, &current_topics))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

/// Tauri command: create a new label on the remote repo.
#[tauri::command]
#[instrument(skip(_state, label), name = "cmd::repo_config::create_label")]
pub async fn create_label(
    repo_path: String,
    label: Label,
    _state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        let cfg = CliForgeRepoConfig::new(SystemRunner::new(), forge);
        cfg.create_label(&path, &label).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

/// Tauri command: update an existing label on the remote repo.
#[tauri::command]
#[instrument(
    skip(_state, label),
    name = "cmd::repo_config::update_label",
    fields(old_name = %old_name)
)]
pub async fn update_label(
    repo_path: String,
    old_name: String,
    label: Label,
    _state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        let cfg = CliForgeRepoConfig::new(SystemRunner::new(), forge);
        cfg.update_label(&path, &old_name, &label)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

/// Tauri command: delete a label by name.
#[tauri::command]
#[instrument(skip(_state), name = "cmd::repo_config::delete_label", fields(name = %name))]
pub async fn delete_label(
    repo_path: String,
    name: String,
    _state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        let cfg = CliForgeRepoConfig::new(SystemRunner::new(), forge);
        cfg.delete_label(&path, &name).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

/// Tauri command: load GitHub branch-protection rules for a branch.
///
/// Returns `Ok(None)` when the branch is not protected. GitLab is not
/// supported; calling this command on a GitLab repo returns an error string
/// the frontend turns into a "not supported on this provider" empty state.
#[tauri::command]
#[instrument(
    skip(_state),
    name = "cmd::repo_config::get_branch_protection",
    fields(branch = %branch)
)]
pub async fn get_branch_protection(
    repo_path: String,
    branch: String,
    _state: State<'_, AppState>,
) -> Result<Option<BranchProtection>, IpcError> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        match forge {
            ForgeKind::GitHub => {
                let runner = SystemRunner::new();
                repo_config::get_branch_protection_github(&runner, &path, &branch)
                    .map_err(|e| e.to_string())
            }
            ForgeKind::GitLab => {
                Err("Branch protection is not supported on GitLab yet".to_string())
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

/// Tauri command: write GitHub branch-protection rules for a branch.
#[tauri::command]
#[instrument(
    skip(_state, rules),
    name = "cmd::repo_config::set_branch_protection",
    fields(branch = %branch)
)]
pub async fn set_branch_protection(
    repo_path: String,
    branch: String,
    rules: BranchProtection,
    _state: State<'_, AppState>,
) -> Result<(), IpcError> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let forge = detect_forge(&repo)
            .ok_or_else(|| "Repository is not hosted on GitHub or GitLab".to_string())?;
        match forge {
            ForgeKind::GitHub => {
                let runner = SystemRunner::new();
                repo_config::set_branch_protection_github(&runner, &path, &branch, &rules)
                    .map_err(|e| e.to_string())
            }
            ForgeKind::GitLab => {
                Err("Branch protection is not supported on GitLab yet".to_string())
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

/// Tauri command: probe the forge CLI availability + auth state for the repo
/// at `repo_path`. Never returns a hard error — every failure mode maps to a
/// structured [`ForgeCliStatus`] variant the frontend renders as an empty
/// state.
#[tauri::command]
#[instrument(skip(_state), name = "cmd::repo_config::probe_cli")]
pub async fn probe_forge_cli_status(
    repo_path: String,
    _state: State<'_, AppState>,
) -> Result<ForgeCliStatus, IpcError> {
    let path = std::path::PathBuf::from(&repo_path);
    tokio::task::spawn_blocking(move || {
        let repo = Repository::open(&path).map_err(|e| e.to_string())?;
        let detected = detect_forge_with_host(&repo);
        let (forge, host) = match &detected {
            Some((k, h)) => (Some(*k), Some(h.as_str())),
            None => (None, None),
        };
        let runner = SystemRunner::new();
        Ok::<_, String>(repo_config::probe_forge_cli_status_with(
            &runner, forge, host, &path,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_forge_from_repository_with_github_origin() {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        git_repo
            .remote("origin", "https://github.com/test/repo.git")
            .unwrap();
        drop(git_repo);
        let repo = Repository::open(dir.path()).unwrap();
        assert_eq!(detect_forge(&repo), Some(ForgeKind::GitHub));
    }

    #[test]
    fn detect_forge_from_repository_with_gitlab_origin() {
        let dir = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(dir.path()).unwrap();
        git_repo
            .remote("origin", "git@gitlab.com:team/app.git")
            .unwrap();
        drop(git_repo);
        let repo = Repository::open(dir.path()).unwrap();
        assert_eq!(detect_forge(&repo), Some(ForgeKind::GitLab));
    }

    #[test]
    fn detect_forge_from_repository_without_origin_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        assert!(detect_forge(&repo).is_none());
    }
}
