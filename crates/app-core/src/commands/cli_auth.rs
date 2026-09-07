//! CLI-based OAuth login and authentication check commands.

use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::ipc_error::IpcError;
use crate::state::AppState;

/// Check authentication status for both `gh` and `glab` CLIs.
///
/// Resolves the bundled sidecar first, then falls back to PATH. Returns a
/// `CliAuthStatus` per tool — if the binary isn't found, the entry has
/// `installed: false` instead of an error.
#[tauri::command]
#[instrument(skip(state), name = "cmd::cli_auth::check_status")]
pub async fn cli_check_auth_status(
    state: State<'_, AppState>,
) -> Result<Vec<cli_provider::auth::CliAuthStatus>, IpcError> {
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
        Ok::<_, String>(vec![gh, glab])
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

/// Get the shell command string to launch an interactive auth flow in a
/// terminal tab for the given CLI tool.
///
/// The frontend opens a terminal tab and types this command into the
/// user's shell. It must therefore name the **same binary** the app itself
/// will drive afterwards — the resolved sidecar path, quoted for the
/// shell — not the bare `gh`/`glab`, which the shell resolves through its
/// own `PATH`: for the user without the CLI installed (the reason the
/// sidecar exists) that was "command not found".
#[tauri::command]
#[instrument(skip(state), name = "cmd::cli_auth::get_auth_command")]
pub fn cli_get_auth_command(tool: String, state: State<'_, AppState>) -> Result<String, IpcError> {
    cli_shell_command(&tool, "login", &state)
}

/// Get the shell command to log out of a CLI tool.
#[tauri::command]
#[instrument(skip(state), name = "cmd::cli_auth::get_logout_command")]
pub fn cli_get_logout_command(
    tool: String,
    state: State<'_, AppState>,
) -> Result<String, IpcError> {
    cli_shell_command(&tool, "logout", &state)
}

fn cli_shell_command(
    tool: &str,
    verb: &str,
    state: &State<'_, AppState>,
) -> Result<String, IpcError> {
    let kind = match tool {
        "gh" => provider::ProviderKind::GitHub,
        "glab" => provider::ProviderKind::GitLab,
        _ => return Err(IpcError::from(format!("Unknown CLI tool: {tool}"))),
    };
    let binary = resolve_cli_binary(state, kind)?;
    Ok(format!(
        "{} auth {verb}",
        shell_quote_path(&binary.to_string_lossy())
    ))
}

/// Quote a filesystem path for the user's interactive shell.
///
/// POSIX shells: single quotes, with embedded `'` spliced as `'\''`.
/// Windows: the terminal is PowerShell/cmd; `& 'path'` runs a quoted
/// path in PowerShell and `'` is doubled inside. cmd.exe users get a
/// PowerShell-shaped line, which is what the default terminal there is.
fn shell_quote_path(path: &str) -> String {
    if cfg!(windows) {
        format!("& '{}'", path.replace('\'', "''"))
    } else {
        format!("'{}'", path.replace('\'', "'\\''"))
    }
}

/// Check if the CLI tool is already authenticated for the given provider.
#[tauri::command]
#[instrument(skip(state), name = "cmd::cli_auth::is_authenticated")]
pub async fn is_cli_authenticated(
    kind: String,
    state: State<'_, AppState>,
) -> Result<bool, IpcError> {
    let provider_kind = provider::ProviderKind::from_config_str(&kind)
        .ok_or_else(|| format!("Unknown provider: {kind}"))?;
    let binary = resolve_cli_binary(&state, provider_kind)?;
    tokio::task::spawn_blocking(move || {
        Ok::<_, String>(cli_provider::auth::is_cli_authenticated(
            &binary,
            provider_kind,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(IpcError::from)
}

#[cfg(test)]
mod tests {
    //! The CLI-detection flows depend on the real `gh`/`glab` binaries and a
    //! Tauri handle — those are integration-tested. What we can unit-test
    //! is the shell quoting of the resolved binary path plus the
    //! not-installed status helper.

    use super::shell_quote_path;

    #[test]
    #[cfg(unix)]
    fn shell_quote_path_survives_spaces_and_quotes() {
        assert_eq!(
            shell_quote_path("/Applications/My Apps/BeardGit.app/Contents/MacOS/gh"),
            "'/Applications/My Apps/BeardGit.app/Contents/MacOS/gh'"
        );
        // An embedded single quote is closed, escaped, and reopened.
        assert_eq!(shell_quote_path("/tmp/it's/gh"), "'/tmp/it'\\''s/gh'");
    }

    #[test]
    #[cfg(windows)]
    fn shell_quote_path_uses_powershell_call_operator() {
        assert_eq!(
            shell_quote_path(r"C:\Program Files\BeardGit\gh.exe"),
            r"& 'C:\Program Files\BeardGit\gh.exe'"
        );
        assert_eq!(shell_quote_path(r"C:\it's\gh.exe"), r"& 'C:\it''s\gh.exe'");
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
