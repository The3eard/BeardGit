//! CLI-based OAuth login and authentication check commands.

use tauri::State;
use tracing::instrument;

use super::helpers::*;
use super::provider_auth::connect_provider;
use crate::state::AppState;

/// Check authentication status for both `gh` and `glab` CLIs.
///
/// Resolves bundled binaries first, then falls back to PATH. Returns a
/// `CliAuthStatus` per tool — if the binary isn't found, the entry has
/// `installed: false` instead of an error.
#[tauri::command]
#[instrument(skip(state), name = "cmd::cli_auth::check_status")]
pub async fn cli_check_auth_status(
    state: State<'_, AppState>,
) -> Result<Vec<cli_provider::auth::CliAuthStatus>, String> {
    let gh_path = resolve_cli_binary(&state, provider::ProviderKind::GitHub).ok();
    let glab_path = resolve_cli_binary(&state, provider::ProviderKind::GitLab).ok();

    tokio::task::spawn_blocking(move || {
        let gh = match gh_path {
            Some(path) => cli_provider::auth::check_gh_auth_status(&path),
            None => cli_provider::auth::not_installed_status("gh"),
        };
        let glab = match glab_path {
            Some(path) => cli_provider::auth::check_glab_auth_status(&path),
            None => cli_provider::auth::not_installed_status("glab"),
        };
        Ok(vec![gh, glab])
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get the shell command string to launch an interactive auth flow in a
/// terminal tab for the given CLI tool.
///
/// Returns `"gh auth login"` or `"glab auth login"` — the frontend opens
/// a terminal tab and writes this command.
#[tauri::command]
#[instrument(name = "cmd::cli_auth::get_auth_command")]
pub fn cli_get_auth_command(tool: String) -> Result<String, String> {
    match tool.as_str() {
        "gh" => Ok("gh auth login".to_string()),
        "glab" => Ok("glab auth login".to_string()),
        _ => Err(format!("Unknown CLI tool: {tool}")),
    }
}

/// Get the shell command to log out of a CLI tool.
#[tauri::command]
#[instrument(name = "cmd::cli_auth::get_logout_command")]
pub fn cli_get_logout_command(tool: String) -> Result<String, String> {
    match tool.as_str() {
        "gh" => Ok("gh auth logout".to_string()),
        "glab" => Ok("glab auth logout".to_string()),
        _ => Err(format!("Unknown CLI tool: {tool}")),
    }
}

/// Check if the CLI tool is already authenticated for the given provider.
#[tauri::command]
#[instrument(skip(state), name = "cmd::cli_auth::is_authenticated")]
pub async fn is_cli_authenticated(
    kind: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let provider_kind = provider::ProviderKind::from_config_str(&kind)
        .ok_or_else(|| format!("Unknown provider: {kind}"))?;
    let binary = resolve_cli_binary(&state, provider_kind)?;
    tokio::task::spawn_blocking(move || {
        Ok(cli_provider::auth::is_cli_authenticated(
            &binary,
            provider_kind,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Start the CLI OAuth login flow and extract + store the token.
///
/// This is a blocking call — the browser opens for OAuth, and this
/// command waits until login completes. Emits `oauth-device-code`
/// event with the one-time code so the frontend can display it.
#[tauri::command]
#[instrument(skip(state, app), name = "cmd::cli_auth::login")]
pub async fn cli_login(
    kind: String,
    instance_url: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<provider::ProviderUser, String> {
    let provider_kind = provider::ProviderKind::from_config_str(&kind)
        .ok_or_else(|| format!("Unknown provider: {kind}"))?;
    let binary = resolve_cli_binary(&state, provider_kind)?;
    let url_ref = instance_url.clone();

    // Start login process (captures device code, opens browser)
    let process = {
        let binary = binary.clone();
        let url = url_ref.clone();
        tokio::task::spawn_blocking(move || {
            cli_provider::auth::start_cli_login(&binary, provider_kind, url.as_deref())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?
    };

    // Emit the device code to the frontend BEFORE waiting for OAuth
    use tauri::Emitter as _;
    let _ = app.emit("oauth-device-code", &process.info);

    // Now wait for OAuth completion (blocks until user finishes in browser)
    {
        tokio::task::spawn_blocking(move || process.wait())
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
    }

    // Extract token (also blocking — runs a subprocess)
    let token = tokio::task::spawn_blocking(move || {
        cli_provider::auth::extract_cli_token(&binary, provider_kind, url_ref.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // Determine the effective URL for storing
    let effective_url = instance_url.unwrap_or_else(|| match provider_kind {
        provider::ProviderKind::GitHub => "https://api.github.com".to_string(),
        provider::ProviderKind::GitLab => "https://gitlab.com".to_string(),
    });

    // Validate and store — reuse existing connect_provider logic
    let user = connect_provider(provider_kind, effective_url, token, state).await?;
    Ok(user)
}
