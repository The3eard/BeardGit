//! CLI-based authentication — OAuth login and token extraction.
//!
//! The primary auth flow uses `gh auth login` / `glab auth login` which
//! opens a browser for OAuth. After login, the token is extracted via
//! `gh auth token` / `glab config get token` and stored in the encrypted
//! credential store.

use std::path::Path;
use std::process::Command;

use provider::ProviderKind;

use crate::error::CliError;

/// Check if the CLI is already authenticated.
///
/// Returns `true` if `gh auth status` / `glab auth status` exits 0.
pub fn is_cli_authenticated(binary_path: &Path, kind: ProviderKind) -> bool {
    let args = match kind {
        ProviderKind::GitHub => vec!["auth", "status"],
        ProviderKind::GitLab => vec!["auth", "status"],
    };
    let mut cmd = Command::new(binary_path);
    cmd.args(&args);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    cmd.output().is_ok_and(|o| o.status.success())
}

/// Extract the authentication token from the CLI after successful login.
///
/// - GitHub: `gh auth token`
/// - GitLab: `glab config get token`
///
/// Returns the token string or an error if not authenticated.
pub fn extract_cli_token(
    binary_path: &Path,
    kind: ProviderKind,
    instance_url: Option<&str>,
) -> Result<String, CliError> {
    let mut cmd = Command::new(binary_path);

    match kind {
        ProviderKind::GitHub => {
            cmd.args(["auth", "token"]);
            if let Some(host) = instance_url {
                // Strip scheme for gh: "github.example.com" not "https://..."
                let host = host
                    .strip_prefix("https://")
                    .or_else(|| host.strip_prefix("http://"))
                    .unwrap_or(host);
                cmd.args(["--hostname", host]);
            }
        }
        ProviderKind::GitLab => {
            cmd.args(["config", "get", "token"]);
            if let Some(host) = instance_url {
                cmd.args(["--host", host]);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = cmd.output()?;

    if output.status.success() {
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if token.is_empty() {
            Err(CliError::NotAuthenticated(
                "CLI returned empty token".to_string(),
            ))
        } else {
            Ok(token)
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(CliError::NotAuthenticated(stderr))
    }
}

/// Start the CLI OAuth login flow (opens a browser).
///
/// This is a blocking call that waits for the user to complete OAuth in the browser.
/// - GitHub: `gh auth login --web -p https`
/// - GitLab: `glab auth login`
///
/// Returns `Ok(())` on success, `Err` on failure or if the user cancels.
pub fn start_cli_login(
    binary_path: &Path,
    kind: ProviderKind,
    instance_url: Option<&str>,
) -> Result<(), CliError> {
    let mut cmd = Command::new(binary_path);

    match kind {
        ProviderKind::GitHub => {
            cmd.args(["auth", "login", "--web", "-p", "https"]);
            if let Some(host) = instance_url {
                let host = host
                    .strip_prefix("https://")
                    .or_else(|| host.strip_prefix("http://"))
                    .unwrap_or(host);
                cmd.args(["--hostname", host]);
            }
        }
        ProviderKind::GitLab => {
            cmd.args(["auth", "login"]);
            if let Some(host) = instance_url {
                cmd.args(["--hostname", host]);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = cmd.output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(CliError::CommandFailed(stderr))
    }
}
