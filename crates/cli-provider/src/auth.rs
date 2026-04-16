//! CLI-based authentication — OAuth login and token extraction.
//!
//! The primary auth flow uses `gh auth login` / `glab auth login` which
//! opens a browser for OAuth. After login, the token is extracted via
//! `gh auth token` / `glab config get token` and stored in the encrypted
//! credential store.

use std::path::Path;
use std::process::{Command, Stdio};

use provider::ProviderKind;

use crate::configure_no_window;
use crate::error::CliError;

// ─── Auth Status Detection ──────────────────────────────────────────────────

/// Authentication status for a CLI tool (`gh` or `glab`).
///
/// Used by the settings UI to show whether each CLI is installed and
/// authenticated, along with the logged-in username when available.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CliAuthStatus {
    /// CLI tool name (`"gh"` or `"glab"`).
    pub tool: String,
    /// Whether the CLI binary was found (bundled or on PATH).
    pub installed: bool,
    /// Whether the user is authenticated.
    pub authenticated: bool,
    /// Username if authenticated (parsed from status output).
    pub username: Option<String>,
    /// Error message if the check failed.
    pub error: Option<String>,
}

/// Check `gh auth status` and return structured auth info.
///
/// The binary path should be resolved via `resolve_cli_binary` so bundled
/// binaries are preferred over PATH.
pub fn check_gh_auth_status(binary_path: &Path) -> CliAuthStatus {
    let mut cmd = Command::new(binary_path);
    cmd.args(["auth", "status"]);
    configure_no_window(&mut cmd);

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}{stderr}");
            let authenticated = output.status.success();
            // gh outputs: "Logged in to github.com account USERNAME ..."
            let username = combined
                .lines()
                .find(|l| l.contains("Logged in to"))
                .and_then(|l| l.split("account ").nth(1))
                .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
                .filter(|s| !s.is_empty());
            CliAuthStatus {
                tool: "gh".into(),
                installed: true,
                authenticated,
                username,
                error: None,
            }
        }
        Err(e) => CliAuthStatus {
            tool: "gh".into(),
            installed: true,
            authenticated: false,
            username: None,
            error: Some(e.to_string()),
        },
    }
}

/// Check `glab auth status` and return structured auth info.
///
/// The binary path should be resolved via `resolve_cli_binary` so bundled
/// binaries are preferred over PATH.
pub fn check_glab_auth_status(binary_path: &Path) -> CliAuthStatus {
    let mut cmd = Command::new(binary_path);
    cmd.args(["auth", "status"]);
    configure_no_window(&mut cmd);

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}{stderr}");
            // glab may exit 0 or print "Logged in" even on non-zero
            let authenticated = output.status.success() || combined.contains("Logged in");
            // glab outputs: "Logged in to gitlab.com as USERNAME ..."
            let username = combined
                .lines()
                .find(|l| l.contains("Logged in"))
                .and_then(|l| l.split("as ").nth(1))
                .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
                .filter(|s| !s.is_empty());
            CliAuthStatus {
                tool: "glab".into(),
                installed: true,
                authenticated,
                username,
                error: None,
            }
        }
        Err(e) => CliAuthStatus {
            tool: "glab".into(),
            installed: true,
            authenticated: false,
            username: None,
            error: Some(e.to_string()),
        },
    }
}

/// Build a `CliAuthStatus` for a tool whose binary was not found.
pub fn not_installed_status(tool: &str) -> CliAuthStatus {
    CliAuthStatus {
        tool: tool.into(),
        installed: false,
        authenticated: false,
        username: None,
        error: None,
    }
}

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
    configure_no_window(&mut cmd);

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

    configure_no_window(&mut cmd);

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

/// Info returned from the OAuth login flow for display in the UI.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OAuthLoginInfo {
    /// The one-time device code (e.g. "ABCD-1234").
    pub code: Option<String>,
    /// The URL the user should visit to enter the code.
    pub url: Option<String>,
}

/// A running OAuth login process.
///
/// Holds the child process handle so the caller can emit UI events
/// (with the device code) before waiting for OAuth completion.
pub struct OAuthLoginProcess {
    child: std::process::Child,
    /// Device code and URL parsed from stderr.
    pub info: OAuthLoginInfo,
}

impl OAuthLoginProcess {
    /// Wait for the OAuth process to complete.
    ///
    /// Blocks until the user finishes (or cancels) in the browser.
    pub fn wait(self) -> Result<(), CliError> {
        let output = self.child.wait_with_output()?;
        if output.status.success() {
            Ok(())
        } else {
            // If we had a code, the user might have just cancelled — not a hard error
            if self.info.code.is_some() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Err(CliError::CommandFailed(stderr))
            }
        }
    }
}

/// Start the CLI OAuth login flow (non-blocking first phase).
///
/// Spawns `gh auth login --web` with `GH_PROMPT_DISABLED=1`, reads the
/// device code and URL from stderr, then returns immediately with the
/// running process handle. The caller should:
///
/// 1. Emit the device code to the UI
/// 2. Call `process.wait()` to block until OAuth completes
pub fn start_cli_login(
    binary_path: &Path,
    kind: ProviderKind,
    instance_url: Option<&str>,
) -> Result<OAuthLoginProcess, CliError> {
    let mut cmd = Command::new(binary_path);

    match kind {
        ProviderKind::GitHub => {
            cmd.args(["auth", "login", "--web", "-p", "https"]);
            // Skip the interactive "Press Enter" prompt — the CLI prints
            // the code and URL directly, then opens the browser.
            cmd.env("GH_PROMPT_DISABLED", "1");
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

    configure_no_window(&mut cmd);
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    // Read stderr to capture the one-time code and URL.
    // gh writes these lines immediately, then blocks waiting for OAuth.
    let mut code: Option<String> = None;
    let mut url: Option<String> = None;

    if let Some(stderr) = child.stderr.take() {
        let reader = std::io::BufReader::new(stderr);
        for line in std::io::BufRead::lines(reader) {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            // gh outputs: "! First copy your one-time code: XXXX-XXXX"
            if line.contains("one-time code:")
                && let Some(c) = line.rsplit("code:").next()
            {
                code = Some(c.trim().to_string());
            }

            // gh outputs: "Open this URL to continue in your web browser: https://..."
            if line.contains("https://") {
                for word in line.split_whitespace() {
                    if word.starts_with("https://") {
                        url = Some(word.trim_end_matches("...").to_string());
                        break;
                    }
                }
            }

            // Once we have both code and URL, stop reading stderr
            // so gh can proceed with the OAuth flow
            if code.is_some() && url.is_some() {
                break;
            }
        }
    }

    Ok(OAuthLoginProcess {
        child,
        info: OAuthLoginInfo { code, url },
    })
}
