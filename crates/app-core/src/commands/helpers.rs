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

/// Build the installed sidecar filename for a given provider.
///
/// Although sidecars are authored on disk as `{base}-{target_triple}[.exe]`,
/// Tauri's build script (`tauri-build::copy_binaries`) strips the triple
/// when copying them into the target directory and the final app bundle.
/// At runtime we therefore look for the plain `{base}[.exe]` filename.
fn sidecar_binary_name(kind: provider::ProviderKind) -> &'static str {
    match kind {
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
    }
}

/// Compute candidate filesystem paths where a sidecar binary might live.
///
/// Tauri places `externalBin` sidecars next to the main executable:
/// - **macOS:** `Foo.app/Contents/MacOS/{name}` (same dir as the exe)
/// - **Linux / Windows:** same directory as the executable
/// - **Dev mode (`cargo tauri dev`):** `target/debug/{name}` alongside the exe
///
/// On macOS we also probe `Contents/Resources/{name}` for resilience against
/// older Tauri versions and a dev-mode `binaries/` subdirectory fallback,
/// but the next-to-exe location is authoritative.
fn sidecar_candidate_paths(
    exe_path: &std::path::Path,
    sidecar_name: &str,
) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    if let Some(exe_dir) = exe_path.parent() {
        // Primary location: next to the main executable. This covers
        // `cargo tauri dev` (target/debug), bundled .app on macOS
        // (Contents/MacOS), Linux, and Windows.
        paths.push(exe_dir.join(sidecar_name));

        // macOS .app fallback: Contents/Resources (older Tauri layouts).
        #[cfg(target_os = "macos")]
        if let Some(contents) = exe_dir.parent() {
            paths.push(contents.join("Resources").join(sidecar_name));
        }

        // Dev-mode `binaries/` subdirectory — defensive fallback if a
        // local workflow places binaries there without running tauri-build.
        paths.push(exe_dir.join("binaries").join(sidecar_name));
    }

    paths
}

/// Resolve the path to the CLI binary for a given provider.
///
/// Resolution order:
/// 1. System `PATH` lookup (plain `gh` / `glab`) — picks up the user's
///    already-installed + authenticated CLI when present.
/// 2. Bundled Tauri sidecar paths (candidate locations from
///    [`sidecar_candidate_paths`]) — used when the user has nothing on
///    PATH so the app still works out of the box.
///
/// The PATH-first ordering is load-bearing. Users who already run
/// `gh auth login` / `glab auth login` against a tool on their PATH
/// expect BeardGit to reuse that session. Preferring the sidecar meant
/// we'd shell out to an unauthenticated bundled binary and silently get
/// empty MR/PR lists (401s parsed as "no results"). The sidecar is the
/// fallback for users who don't install the CLIs themselves.
///
/// Sidecar binaries are authored as `{name}-{target_triple}[.exe]` but
/// Tauri strips the triple when copying them, so at runtime the
/// installed filename is the plain `{name}[.exe]`.
pub(super) fn resolve_cli_binary(
    _state: &State<'_, AppState>,
    kind: provider::ProviderKind,
) -> Result<std::path::PathBuf, String> {
    // 1. Prefer a system-wide install so the user's existing auth is reused.
    //    `which::which` handles `PATHEXT` resolution on Windows, so we pass
    //    the extensionless name on every OS.
    let plain_name = match kind {
        provider::ProviderKind::GitHub => "gh",
        provider::ProviderKind::GitLab => "glab",
    };
    if let Ok(path) = which::which(plain_name) {
        return Ok(path);
    }

    // 2. Fallback: bundled Tauri sidecar.
    let sidecar_name = sidecar_binary_name(kind);
    if let Ok(exe_path) = std::env::current_exe() {
        for candidate in sidecar_candidate_paths(&exe_path, sidecar_name) {
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    Err(format!(
        "{plain_name} not found. Install it (or authenticate it) and restart BeardGit.\n\
         Looked for system '{plain_name}' and bundled sidecar '{sidecar_name}'."
    ))
}

/// Build an [`Arc<dyn ForgeProvider>`] from the current application state.
///
/// Resolves the active provider's kind, the CLI binary path, and the active
/// project's filesystem path, then constructs the concrete `GitHubCli` /
/// `GitLabCli` and erases it behind the trait. Downstream command modules
/// only see the trait object.
pub(super) fn build_forge_provider(
    state: &State<'_, AppState>,
) -> Result<std::sync::Arc<dyn forge_provider::ForgeProvider>, String> {
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

    let provider: std::sync::Arc<dyn forge_provider::ForgeProvider> = match kind {
        provider::ProviderKind::GitHub => {
            std::sync::Arc::new(cli_provider::GitHubCli::new(binary, cwd))
        }
        provider::ProviderKind::GitLab => {
            std::sync::Arc::new(cli_provider::GitLabCli::new(binary, cwd))
        }
    };

    Ok(provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_binary_name_github_is_plain() {
        let name = sidecar_binary_name(provider::ProviderKind::GitHub);
        if cfg!(target_os = "windows") {
            assert_eq!(name, "gh.exe");
        } else {
            assert_eq!(name, "gh");
        }
    }

    #[test]
    fn sidecar_binary_name_gitlab_is_plain() {
        let name = sidecar_binary_name(provider::ProviderKind::GitLab);
        if cfg!(target_os = "windows") {
            assert_eq!(name, "glab.exe");
        } else {
            assert_eq!(name, "glab");
        }
    }

    #[test]
    fn sidecar_paths_first_candidate_is_next_to_exe() {
        let fake_exe = std::path::PathBuf::from("/app/beardgit");
        let paths = sidecar_candidate_paths(&fake_exe, "gh");

        // The first (authoritative) candidate must be next to the exe —
        // this is where Tauri installs sidecars in every release layout.
        assert_eq!(
            paths.first(),
            Some(&std::path::PathBuf::from("/app/gh")),
            "expected next-to-exe to be the first candidate, got: {paths:?}"
        );
    }

    #[test]
    fn sidecar_paths_include_binaries_subdir_fallback() {
        let fake_exe = std::path::PathBuf::from("/app/beardgit");
        let paths = sidecar_candidate_paths(&fake_exe, "gh");

        assert!(
            paths
                .iter()
                .any(|p| p == std::path::Path::new("/app/binaries/gh")),
            "expected binaries/ subdir path, got: {paths:?}"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sidecar_paths_include_macos_resources_fallback() {
        let fake_exe = std::path::PathBuf::from("/App.app/Contents/MacOS/beardgit");
        let paths = sidecar_candidate_paths(&fake_exe, "gh");

        assert!(
            paths
                .iter()
                .any(|p| p == std::path::Path::new("/App.app/Contents/Resources/gh")),
            "expected Contents/Resources fallback, got: {paths:?}"
        );
    }
}
