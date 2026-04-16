//! Provider authentication and CI run commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Connect to a git hosting provider using a Personal Access Token (PAT).
///
/// Validates the token, stores it in the encrypted credential store,
/// builds a [`ProviderConnection`][crate::state::ProviderConnection] with
/// the authenticated user's profile, and appends it to the providers vec
/// (or replaces an existing entry with the same `instance_url`).
///
/// After connecting, re-runs active provider detection against the current
/// repo's remote URL and persists all providers to `settings.json`.
///
/// # Parameters
/// - `kind`         – Provider type (`"gitlab"` or `"github"`).
/// - `instance_url` – Base URL (e.g. `"https://gitlab.com"` or `"https://api.github.com"`).
/// - `token`        – Personal Access Token.
///
/// # Returns
/// The authenticated user profile as a [`provider::ProviderUser`].
#[tauri::command]
pub async fn connect_provider(
    kind: provider::ProviderKind,
    instance_url: String,
    token: String,
    state: State<'_, AppState>,
) -> Result<provider::ProviderUser, String> {
    // 1. Validate token
    let user = match kind {
        provider::ProviderKind::GitLab => auth::validate_gitlab_pat(&instance_url, &token).await,
        provider::ProviderKind::GitHub => auth::validate_github_pat(&instance_url, &token).await,
    }
    .map_err(|e| e.to_string())?;

    // 2. Store credential
    let credential = auth::Credential {
        token: token.clone(),
        provider: kind,
    };
    state
        .credential_store
        .store_credential(&instance_url, &credential)
        .map_err(|e| e.to_string())?;

    // 3. Build ProviderConnection (metadata only, no CiProvider)
    let conn = crate::state::ProviderConnection {
        kind,
        instance_url: instance_url.clone(),
        user: user.clone(),
        project_ref: None,
        project_name: None,
    };

    // 4. Insert or replace in providers vec
    {
        let mut providers = state.providers.lock().unwrap();
        if let Some(pos) = providers
            .iter()
            .position(|p| p.instance_url == instance_url)
        {
            providers[pos] = conn;
        } else {
            providers.push(conn);
        }
    }

    // 5. Save to config and detect active provider
    save_providers_to_config(&state);
    detect_active_provider(&state).await;

    Ok(user)
}

/// Disconnect a specific provider identified by its instance URL.
///
/// Removes the provider from the in-memory vec, deletes the credential
/// from the encrypted store, saves the updated config, and re-runs
/// active provider detection.
///
/// # Parameters
/// - `instance_url` – Base URL of the provider to disconnect.
#[tauri::command]
pub async fn disconnect_provider(
    instance_url: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Remove from providers vec
    {
        let mut providers = state.providers.lock().unwrap();
        providers.retain(|p| p.instance_url != instance_url);
    }

    // Delete credential
    let _ = state.credential_store.delete_credential(&instance_url);

    // Save config and re-detect
    save_providers_to_config(&state);
    detect_active_provider(&state).await;

    Ok(())
}

/// Attempt to restore all previously saved provider sessions on app startup.
///
/// Reads the `providers` list from `settings.json`, retrieves each token from
/// the credential store, validates it against the provider API, and builds
/// a [`ProviderConnection`][crate::state::ProviderConnection] for each
/// successful validation.
///
/// After reconnecting, runs active provider detection against the current
/// repo's remote URL.
///
/// # Returns
/// A list of successfully reconnected user profiles.
#[tauri::command]
pub async fn try_auto_connect(
    state: State<'_, AppState>,
) -> Result<Vec<provider::ProviderUser>, String> {
    // Read saved providers from config
    let saved_providers = {
        let config = state.config.lock().unwrap();
        config.providers.clone()
    };

    let mut connected_users = Vec::new();
    let mut connections = Vec::new();

    for saved in &saved_providers {
        let kind = match provider::ProviderKind::from_config_str(&saved.kind) {
            Some(k) => k,
            None => continue,
        };

        // Get token from credential store
        let credential = match state.credential_store.get_credential(&saved.instance_url) {
            Ok(Some(c)) => c,
            _ => continue,
        };

        // Validate token
        let user = match kind {
            provider::ProviderKind::GitLab => {
                auth::validate_gitlab_pat(&saved.instance_url, &credential.token).await
            }
            provider::ProviderKind::GitHub => {
                auth::validate_github_pat(&saved.instance_url, &credential.token).await
            }
        };

        let user = match user {
            Ok(u) => u,
            Err(_) => continue,
        };

        connections.push(crate::state::ProviderConnection {
            kind,
            instance_url: saved.instance_url.clone(),
            user: user.clone(),
            project_ref: None,
            project_name: None,
        });
        connected_users.push(user);
    }

    // Store all successful connections
    *state.providers.lock().unwrap() = connections;

    // Detect active provider from repo remote
    detect_active_provider(&state).await;

    Ok(connected_users)
}

/// Return the current multi-provider connection status.
///
/// Builds a [`provider::ProviderStatusResponse`] containing all authenticated
/// providers and which one (if any) is active for the currently open repository.
/// Used by the frontend to render the provider list and active badge.
#[tauri::command]
pub fn get_provider_status(state: State<'_, AppState>) -> provider::ProviderStatusResponse {
    let providers = state.providers.lock().unwrap();
    let active_index = *state.active_provider_index.lock().unwrap();

    let connected: Vec<provider::ConnectedProvider> = providers
        .iter()
        .map(|p| provider::ConnectedProvider {
            kind: p.kind.as_str().to_string(),
            instance_url: p.instance_url.clone(),
            user: p.user.clone(),
            project_name: p.project_name.clone(),
        })
        .collect();

    provider::ProviderStatusResponse {
        providers: connected,
        active_index,
    }
}

/// Re-detect the active provider from the currently open repository's remote URL.
///
/// Iterates all connected providers, matches the remote URL against each,
/// and sets the active provider index on the first match. Clears project info
/// on all non-matching providers.
///
/// Call this after opening a new repo when providers are already connected,
/// so the CI panel automatically scopes to the correct project.
#[tauri::command]
pub async fn detect_project(state: State<'_, AppState>) -> Result<(), String> {
    detect_active_provider(&state).await;
    Ok(())
}
