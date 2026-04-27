//! CLI-based authentication — status detection, token extraction, and
//! programmatic non-interactive auth.
//!
//! Two shapes live here:
//!
//! 1. **Status / extract helpers** — [`check_gh_auth_status`],
//!    [`check_glab_auth_status`], [`is_cli_authenticated`],
//!    [`extract_cli_token`]. Read-only probes over the CLIs that the
//!    settings UI uses to render the per-row connection state.
//! 2. **Programmatic non-interactive auth** — [`pipe_token_to_cli`] and
//!    [`clear_cli_auth`]. Drive `gh auth login --with-token` / `glab auth
//!    login --stdin` (and their `logout` counterparts) from an already-
//!    validated PAT so a single connect action in the app also logs the
//!    matching CLI in under the same identity. These run synchronously
//!    and are intended to be invoked from a `tokio::spawn_blocking`
//!    background task by the Tauri command layer (fire-and-forget).
//!
//! Interactive browser-based login is handled outside this module by the
//! terminal-hosted `gh auth login` / `glab auth login` flow the UI runs
//! in an xterm.js tab — there is no programmatic OAuth device-code path
//! on the Rust side anymore.

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

// ─── Programmatic non-interactive auth ──────────────────────────────────────

/// Normalize a provider instance URL into the hostname the CLI expects for its
/// `--hostname` flag.
///
/// Strips `https://` / `http://` and a trailing slash. For GitHub, also strips
/// an `api.` prefix so `https://api.github.com` → `github.com` (GHES /
/// self-hosted GitHub uses `gh.example.com`, not `api.gh.example.com`, so the
/// strip is safe for those too — the prefix is only present on the public
/// cloud API endpoint we store for the default GitHub provider).
///
/// GitLab passes through unchanged (`gitlab.com`, `gitlab.mycompany.com`, …).
fn normalize_hostname(kind: ProviderKind, url: &str) -> String {
    let host = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url)
        .trim_end_matches('/');
    match kind {
        ProviderKind::GitHub => host.strip_prefix("api.").unwrap_or(host).to_string(),
        ProviderKind::GitLab => host.to_string(),
    }
}

/// Pipe an already-validated PAT into the CLI so `gh` / `glab` are logged in
/// under the same identity as the app's API session.
///
/// Runs synchronously — intended to be dispatched from a background
/// `tokio::spawn_blocking` task by the Tauri command layer so the user is
/// never blocked on subprocess latency. The caller is expected to have
/// validated the token against the provider API first; this helper only
/// handles the CLI hand-off.
///
/// Command shapes:
/// - GitHub: `gh auth login --with-token --hostname <host>` (token on stdin)
/// - GitLab: `glab auth login --stdin --hostname <host> --api-protocol https`
///
/// # Failure modes
///
/// - [`CliError::Io`] — the binary failed to spawn (binary missing, permission
///   error, OS resource exhaustion).
/// - [`CliError::CommandFailed`] — the CLI exited non-zero. The inner message
///   contains the captured stderr so the background logger can surface it.
pub fn pipe_token_to_cli(
    binary_path: &Path,
    kind: ProviderKind,
    instance_url: &str,
    token: &str,
) -> Result<(), CliError> {
    use std::io::Write as _;

    let host = normalize_hostname(kind, instance_url);
    let mut cmd = Command::new(binary_path);
    match kind {
        ProviderKind::GitHub => {
            cmd.args(["auth", "login", "--with-token", "--hostname", &host]);
        }
        ProviderKind::GitLab => {
            cmd.args([
                "auth",
                "login",
                "--stdin",
                "--hostname",
                &host,
                "--api-protocol",
                "https",
            ]);
        }
    }
    configure_no_window(&mut cmd);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| CliError::CommandFailed("no stdin on spawned CLI".into()))?;
        stdin.write_all(token.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(CliError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }
}

/// Log the CLI out of the given host.
///
/// Runs `gh auth logout --hostname <host>` / `glab auth logout --hostname
/// <host>`. Intended for use from a background `tokio::spawn_blocking` task
/// when the user disconnects the matching PAT in the app.
///
/// # Idempotency
///
/// Disconnect in the app is user-visible success the moment the credential is
/// removed from the keyring; the CLI half is a side-effect. To preserve that
/// "disconnect always works" guarantee this helper treats a "not logged in"
/// or "no credentials" stderr on non-zero exit as success ( `Ok(())` ). Any
/// other non-zero exit propagates as [`CliError::CommandFailed`] so callers
/// can log the unexpected failure.
///
/// # Failure modes
///
/// - [`CliError::Io`] — the binary failed to spawn.
/// - [`CliError::CommandFailed`] — the CLI exited non-zero for a reason other
///   than "already logged out".
pub fn clear_cli_auth(
    binary_path: &Path,
    kind: ProviderKind,
    instance_url: &str,
) -> Result<(), CliError> {
    let host = normalize_hostname(kind, instance_url);
    let mut cmd = Command::new(binary_path);
    cmd.args(["auth", "logout", "--hostname", &host]);
    configure_no_window(&mut cmd);
    let output = cmd.output()?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("not logged in") || stderr.contains("no credentials") {
        return Ok(());
    }
    Err(CliError::CommandFailed(stderr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_hostname_strips_api_prefix_for_github() {
        assert_eq!(
            normalize_hostname(ProviderKind::GitHub, "https://api.github.com"),
            "github.com"
        );
        assert_eq!(
            normalize_hostname(
                ProviderKind::GitHub,
                "https://api.github.enterprise.example"
            ),
            "github.enterprise.example"
        );
        // GHES without the `api.` prefix passes through unchanged.
        assert_eq!(
            normalize_hostname(ProviderKind::GitHub, "https://github.enterprise.example/"),
            "github.enterprise.example"
        );
    }

    #[test]
    fn normalize_hostname_passes_gitlab_through() {
        assert_eq!(
            normalize_hostname(ProviderKind::GitLab, "https://gitlab.com"),
            "gitlab.com"
        );
        assert_eq!(
            normalize_hostname(ProviderKind::GitLab, "https://gitlab.mycompany.com/"),
            "gitlab.mycompany.com"
        );
        assert_eq!(
            normalize_hostname(ProviderKind::GitLab, "http://gitlab.local"),
            "gitlab.local"
        );
    }

    #[cfg(unix)]
    mod unix_subprocess {
        //! Unix-only tests for the programmatic auth helpers.
        //!
        //! These fake the CLI binary with a tiny POSIX shell script that
        //! records its stdin to a sibling file and exits with a
        //! configurable code. Windows gets the same real-world coverage
        //! via manual testing; a parallel `.cmd` rig is out of scope for
        //! this slice (see the plan's "Deferred / follow-up" section).
        use super::*;
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        use std::path::PathBuf;
        use tempfile::TempDir;

        /// Build a POSIX shell script that records its stdin to
        /// `<dir>/stdin.txt`, writes `stderr_msg` to stderr, and exits
        /// with `exit_code`. Returns `(script_path, stdin_capture_path)`.
        ///
        /// Implementation note: we open the file with the executable bit
        /// set at creation (`OpenOptions::mode(0o755)`), `sync_all` it,
        /// then drop the handle before returning. The previous
        /// `fs::write` + `set_permissions` shape was vulnerable to the
        /// `ETXTBSY` ("Text file busy") race on Linux CI: the kernel can
        /// still see an outstanding write reference at exec time when
        /// permissions are flipped after the close, which made tests like
        /// `clear_cli_auth_other_non_zero_is_error` flake intermittently
        /// on the GitHub Actions ubuntu runners.
        fn mock_cli(dir: &TempDir, exit_code: i32, stderr_msg: &str) -> (PathBuf, PathBuf) {
            let script_path = dir.path().join("mock-cli");
            let stdin_capture = dir.path().join("stdin.txt");
            // Escape single quotes in the stderr message by closing +
            // reopening the quoted string.
            let escaped_stderr = stderr_msg.replace('\'', "'\\''");
            let script = format!(
                "#!/bin/sh\ncat > '{}'\nprintf '%s' '{}' >&2\nexit {}\n",
                stdin_capture.display(),
                escaped_stderr,
                exit_code,
            );
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o755)
                .open(&script_path)
                .expect("open mock script");
            file.write_all(script.as_bytes())
                .expect("write mock script");
            file.sync_all().expect("sync mock script");
            // Explicit drop so the close happens before we return — the
            // exec in the caller must see a fully-written, non-busy file.
            drop(file);
            (script_path, stdin_capture)
        }

        #[test]
        fn pipe_token_to_cli_sends_token_on_stdin() {
            let dir = tempfile::tempdir().unwrap();
            let (script, stdin_capture) = mock_cli(&dir, 0, "");

            let result = pipe_token_to_cli(
                &script,
                ProviderKind::GitHub,
                "https://api.github.com",
                "ghp_testtoken",
            );

            assert!(result.is_ok(), "expected Ok, got {result:?}");
            let recorded =
                std::fs::read_to_string(&stdin_capture).expect("read stdin capture");
            assert_eq!(recorded.trim(), "ghp_testtoken");
        }

        #[test]
        fn pipe_token_to_cli_maps_non_zero_exit_to_command_failed() {
            let dir = tempfile::tempdir().unwrap();
            let (script, _) = mock_cli(&dir, 1, "bad token");

            let err = pipe_token_to_cli(
                &script,
                ProviderKind::GitHub,
                "https://api.github.com",
                "ghp_bad",
            )
            .expect_err("expected Err on non-zero exit");

            match err {
                CliError::CommandFailed(msg) => {
                    assert!(
                        msg.contains("bad token"),
                        "expected stderr in error message, got {msg:?}"
                    );
                }
                other => panic!("expected CommandFailed, got {other:?}"),
            }
        }

        #[test]
        fn pipe_token_to_cli_missing_binary_returns_error() {
            let result = pipe_token_to_cli(
                Path::new("/definitely/not/a/real/binary/path/gh"),
                ProviderKind::GitHub,
                "https://api.github.com",
                "ghp_any",
            );
            assert!(result.is_err(), "expected Err for missing binary");
        }

        #[test]
        fn clear_cli_auth_success() {
            let dir = tempfile::tempdir().unwrap();
            let (script, _) = mock_cli(&dir, 0, "");

            let result = clear_cli_auth(&script, ProviderKind::GitHub, "https://api.github.com");
            assert!(result.is_ok(), "expected Ok, got {result:?}");
        }

        #[test]
        fn clear_cli_auth_is_idempotent_when_not_logged_in() {
            let dir = tempfile::tempdir().unwrap();
            // Realistic `gh`/`glab` stderr when no creds exist for the host.
            let (script, _) = mock_cli(&dir, 1, "not logged in to github.com");

            let result = clear_cli_auth(&script, ProviderKind::GitHub, "https://api.github.com");
            assert!(
                result.is_ok(),
                "expected idempotent Ok when already logged out, got {result:?}"
            );
        }

        #[test]
        fn clear_cli_auth_is_idempotent_on_no_credentials_stderr() {
            let dir = tempfile::tempdir().unwrap();
            let (script, _) = mock_cli(&dir, 1, "No credentials stored for gitlab.com");

            let result = clear_cli_auth(&script, ProviderKind::GitLab, "https://gitlab.com");
            assert!(
                result.is_ok(),
                "expected idempotent Ok on 'no credentials', got {result:?}"
            );
        }

        #[test]
        fn clear_cli_auth_other_non_zero_is_error() {
            let dir = tempfile::tempdir().unwrap();
            let (script, _) = mock_cli(&dir, 2, "unexpected failure talking to keychain");

            let err = clear_cli_auth(&script, ProviderKind::GitHub, "https://api.github.com")
                .expect_err("expected Err on unrelated non-zero exit");
            match err {
                CliError::CommandFailed(msg) => {
                    assert!(
                        msg.contains("unexpected failure"),
                        "expected stderr content in error, got {msg:?}"
                    );
                }
                other => panic!("expected CommandFailed, got {other:?}"),
            }
        }
    }
}
