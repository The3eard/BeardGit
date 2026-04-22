//! CLI-based OAuth login and authentication check commands.

use tauri::State;
use tracing::instrument;

use super::helpers::*;
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

#[cfg(test)]
mod tests {
    //! The CLI-detection flows depend on the real `gh`/`glab` binaries and a
    //! Tauri handle — those are integration-tested. What we can unit-test
    //! are the two pure command wrappers (`cli_get_auth_command`,
    //! `cli_get_logout_command`) plus the not-installed status helper.

    use super::{cli_get_auth_command, cli_get_logout_command};

    #[test]
    fn cli_get_auth_command_maps_known_tools() {
        assert_eq!(
            cli_get_auth_command("gh".to_string()).unwrap(),
            "gh auth login"
        );
        assert_eq!(
            cli_get_auth_command("glab".to_string()).unwrap(),
            "glab auth login"
        );
    }

    #[test]
    fn cli_get_auth_command_unknown_tool_errors() {
        let err = cli_get_auth_command("hub".to_string()).err();
        assert!(err.is_some(), "unknown tool should be rejected");
    }

    #[test]
    fn cli_get_logout_command_maps_known_tools() {
        assert_eq!(
            cli_get_logout_command("gh".to_string()).unwrap(),
            "gh auth logout"
        );
        assert_eq!(
            cli_get_logout_command("glab".to_string()).unwrap(),
            "glab auth logout"
        );
    }

    #[test]
    fn cli_get_logout_command_unknown_tool_errors() {
        assert!(cli_get_logout_command("".to_string()).is_err());
        assert!(cli_get_logout_command("cli".to_string()).is_err());
    }

    #[test]
    fn not_installed_status_marks_tool_unavailable() {
        // The command flows fall through to `not_installed_status` when a
        // bundled binary can't be resolved. Spot-check the helper so a
        // future refactor that removes the stable shape shows up here.
        let status = cli_provider::auth::not_installed_status("gh");
        assert_eq!(status.tool, "gh");
        assert!(
            !status.installed,
            "not_installed_status should mark tool as not installed, got {status:?}"
        );
    }
}
