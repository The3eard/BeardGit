//! Shared types and helper functions used across command modules.
//!
//! Types are `pub` (needed in command return types); helper functions
//! are `pub(super)` (visible only within the `commands` module) except
//! for [`get_active_project_path`] which is `pub(crate)` because
//! `ai_commands.rs` imports it.

use std::path::PathBuf;

use tauri::State;

use crate::state::AppState;

// ─── Serializable response types ────────────────────────────────────────────

/// Basic repository metadata returned by [`super::repository::open_repo`].
#[derive(serde::Serialize)]
pub struct RepoInfo {
    /// Absolute path to the repository root.
    pub path: String,
    /// Name of the currently checked-out branch, if any.
    pub head_branch: Option<String>,
    /// SHA of the HEAD commit, if any.
    pub head_oid: Option<String>,
    /// Total number of local branches.
    pub branch_count: usize,
}

/// A slice of the commit graph used for virtual-scroll rendering.
#[derive(serde::Serialize)]
pub struct GraphViewport {
    pub nodes: Vec<graph_builder::LayoutNode>,
    pub lane_segments: Vec<graph_builder::LaneSegment>,
    pub merge_curves: Vec<graph_builder::MergeCurve>,
    pub total_count: usize,
    pub offset: usize,
    pub visible_lane_count: usize,
    pub total_lane_count: usize,
    /// Lane index of the HEAD commit, if present in the graph.
    pub head_lane: Option<usize>,
}

/// Lightweight project info for tab display (no graph data).
#[derive(serde::Serialize)]
pub struct ProjectInfo {
    /// Absolute filesystem path to the repository root.
    pub path: String,
    /// Repository name (last path segment).
    pub name: String,
    /// Current HEAD branch name, if any.
    pub head_branch: Option<String>,
    /// Number of uncommitted changes.
    pub change_count: usize,
}

/// A recently closed repo for the "+" dropdown.
#[derive(serde::Serialize)]
pub struct RecentRepo {
    /// Absolute filesystem path to the repository root.
    pub path: String,
    /// Repository name (last path segment).
    pub name: String,
}

/// Info about a configured git remote.
#[derive(serde::Serialize)]
pub struct RemoteInfo {
    /// Remote name (e.g. `"origin"`).
    pub name: String,
    /// Remote URL, if available.
    pub url: Option<String>,
}

// ─── Helper functions ───────────────────────────────────────────────────────

/// Execute a function with a reference to the active project's repository.
///
/// Locks `projects` and `active_index`, resolves the active [`ProjectSlot`],
/// and calls `f` with the loaded [`git_engine::Repository`]. Returns an error
/// string if no project is active, the index is out of bounds, or no repository
/// is loaded in the slot.
pub(super) fn with_active_repo<F, R>(state: &State<'_, AppState>, f: F) -> Result<R, String>
where
    F: FnOnce(&git_engine::Repository) -> Result<R, String>,
{
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let active = state.active_index.lock().map_err(|e| e.to_string())?;
    let idx = active.ok_or_else(|| "No active project".to_string())?;
    let slot = projects
        .get(idx)
        .ok_or_else(|| "Active project index out of bounds".to_string())?;
    let repo = slot
        .repo
        .as_ref()
        .ok_or_else(|| "No repository open".to_string())?;
    f(repo)
}

/// Get the filesystem path of the active project.
pub(crate) fn get_active_project_path(state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let active = state.active_index.lock().map_err(|e| e.to_string())?;
    let idx = active.ok_or_else(|| "No active project".to_string())?;
    let slot = projects
        .get(idx)
        .ok_or_else(|| "Active project index out of bounds".to_string())?;
    Ok(PathBuf::from(&slot.path))
}

/// Run a blocking closure on a dedicated thread and map errors to `String`.
pub(super) async fn run_blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

/// Extract the origin remote URL from a repository (synchronous, no await).
pub(super) fn extract_origin_url(repo: &git_engine::Repository) -> Option<String> {
    let git_repo = repo.inner();
    let remote = git_repo.find_remote("origin").ok()?;
    let url = remote.url()?.to_string();
    Some(url)
}

/// Create a `Box<dyn CiProvider>` from provider metadata and a token.
///
/// Centralizes the provider construction logic to avoid repeating the
/// match on `ProviderKind` throughout the codebase.
pub(super) fn create_ci_provider(
    kind: provider::ProviderKind,
    base_url: &str,
    token: &str,
) -> Box<dyn provider::CiProvider> {
    match kind {
        provider::ProviderKind::GitLab => {
            Box::new(gitlab_api::GitLabProvider::new(base_url, token))
        }
        provider::ProviderKind::GitHub => {
            Box::new(github_api::GitHubProvider::new(base_url, token))
        }
    }
}

/// Extract the active provider's CI client and project reference from state.
///
/// Reads `active_provider_index` to find the active
/// [`ProviderConnection`][crate::state::ProviderConnection], retrieves its
/// token from the credential store, and creates a fresh `Box<dyn CiProvider>`.
///
/// Returns an error if no provider is active or no project is detected.
pub(super) fn get_active_provider_and_project(
    state: &State<'_, AppState>,
) -> Result<(Box<dyn provider::CiProvider>, String), String> {
    let (kind, base_url, project_ref) = {
        let providers = state.providers.lock().unwrap();
        let active_idx = state.active_provider_index.lock().unwrap();
        let idx = active_idx.ok_or("No active provider")?;
        let conn = providers
            .get(idx)
            .ok_or("Active provider index out of bounds")?;
        let project_ref = conn.project_ref.clone().ok_or("No project detected")?;
        (conn.kind, conn.instance_url.clone(), project_ref)
    };

    let credential = state
        .credential_store
        .get_credential(&base_url)
        .map_err(|e| e.to_string())?
        .ok_or("No credential found for active provider")?;

    let ci_provider = create_ci_provider(kind, &base_url, &credential.token);
    Ok((ci_provider, project_ref))
}

/// Detect which provider (if any) matches the current repo's remote URL
/// and set it as the active provider.
///
/// Iterates all entries in the providers vec, calls
/// [`provider::parse_remote_url`] against each, and on the first match
/// verifies the project via the provider API. Sets `active_provider_index`
/// to the matching entry and stores `project_ref` / `project_name` on it.
/// Clears project info on all non-matching providers.
///
/// If no repo is open or no provider matches, `active_provider_index` is
/// set to `None`.
pub(super) async fn detect_active_provider(state: &State<'_, AppState>) {
    // Get the repo's origin remote URL from the active slot
    let remote_url = {
        let projects = state.projects.lock().unwrap();
        let active = state.active_index.lock().unwrap();
        active
            .and_then(|idx| projects.get(idx))
            .and_then(|slot| slot.repo.as_ref())
            .and_then(extract_origin_url)
    };

    let remote_url = match remote_url {
        Some(url) => url,
        None => {
            // No repo open — clear active index and all project info
            *state.active_provider_index.lock().unwrap() = None;
            let mut providers = state.providers.lock().unwrap();
            for p in providers.iter_mut() {
                p.project_ref = None;
                p.project_name = None;
            }
            return;
        }
    };

    // Snapshot provider metadata (kind, url) so we don't hold the lock across await
    let provider_snapshots: Vec<(usize, provider::ProviderKind, String)> = {
        let providers = state.providers.lock().unwrap();
        providers
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.kind, p.instance_url.clone()))
            .collect()
    };

    let mut matched_index: Option<usize> = None;
    let mut matched_project_ref: Option<String> = None;
    let mut matched_project_name: Option<String> = None;

    for (idx, kind, instance_url) in &provider_snapshots {
        let parsed =
            provider::parse_remote_url(&remote_url, Some(instance_url.as_str()), Some(*kind));

        let project_ref = match parsed {
            Some((_, ref_)) => ref_,
            None => continue,
        };

        // Get token to verify project
        let credential = match state.credential_store.get_credential(instance_url) {
            Ok(Some(c)) => c,
            _ => continue,
        };

        let ci_provider = create_ci_provider(*kind, instance_url, &credential.token);

        // Verify the project exists via the API
        match ci_provider.get_project(&project_ref).await {
            Ok(project) => {
                matched_index = Some(*idx);
                matched_project_ref = Some(project_ref);
                matched_project_name = Some(project.full_path);
                break; // First match wins
            }
            Err(_) => continue,
        }
    }

    // Update providers vec with match results
    {
        let mut providers = state.providers.lock().unwrap();
        for (i, p) in providers.iter_mut().enumerate() {
            if Some(i) == matched_index {
                p.project_ref = matched_project_ref.clone();
                p.project_name = matched_project_name.clone();
            } else {
                p.project_ref = None;
                p.project_name = None;
            }
        }
    }

    *state.active_provider_index.lock().unwrap() = matched_index;
}

/// Persist the current providers vec to `settings.json`.
///
/// Builds a `Vec<SavedProvider>` from the in-memory provider connections
/// and writes it to the config file.
pub(super) fn save_providers_to_config(state: &State<'_, AppState>) {
    let saved: Vec<storage::config::SavedProvider> = {
        let providers = state.providers.lock().unwrap();
        providers
            .iter()
            .map(|p| storage::config::SavedProvider {
                kind: p.kind.as_str().to_string(),
                instance_url: p.instance_url.clone(),
            })
            .collect()
    };

    let mut config = state.config.lock().unwrap();
    config.providers = saved;
    let _ = config.save(&state.config_path);
}

/// Resolve the path to the CLI binary for a given provider.
///
/// Checks the app's bundled resources first, then falls back to PATH lookup.
pub(super) fn resolve_cli_binary(
    _state: &State<'_, AppState>,
    kind: provider::ProviderKind,
) -> Result<std::path::PathBuf, String> {
    let binary_name = match kind {
        provider::ProviderKind::GitHub => {
            if cfg!(target_os = "windows") {
                "gh.exe"
            } else {
                "gh"
            }
        }
        provider::ProviderKind::GitLab => {
            if cfg!(target_os = "windows") {
                "glab.exe"
            } else {
                "glab"
            }
        }
    };

    // Look in the app's resource directory (bundled binaries)
    let resource_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join(binary_name)));

    if let Some(ref path) = resource_path
        && path.exists()
    {
        return Ok(path.clone());
    }

    // Fallback: check if it's on PATH
    which::which(binary_name)
        .map_err(|_| format!("{binary_name} not found. Install it or check your PATH."))
}

/// Build a [`CliProvider`] from the current application state.
///
/// Resolves the active provider's kind, the CLI binary path, and the active
/// project's filesystem path.
pub(super) fn build_cli_provider(
    state: &State<'_, AppState>,
) -> Result<cli_provider::CliProvider, String> {
    let kind = {
        let providers = state.providers.lock().map_err(|e| e.to_string())?;
        let active_idx = state
            .active_provider_index
            .lock()
            .map_err(|e| e.to_string())?;
        let idx = active_idx.ok_or("No active provider")?;
        let conn = providers.get(idx).ok_or("Provider index out of bounds")?;
        conn.kind
    };

    let cwd = get_active_project_path(state)?;
    let binary = resolve_cli_binary(state, kind)?;

    Ok(cli_provider::CliProvider::new(kind, binary, cwd))
}
