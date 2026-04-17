//! CLI-based [`ForgeProvider`][forge_provider::ForgeProvider] implementations
//! wrapping the `gh` and `glab` binaries.
//!
//! Both CLI tools expose structured JSON via `--json` flags; this crate parses
//! that JSON into the shared forge types (`MrPr`, `MrPrDetail`, `Comment`, …)
//! defined by the `forge-provider` crate. Binary paths are resolved from the
//! app's bundled resources directory.

pub mod auth;
pub mod error;
pub mod github;
pub mod gitlab;
pub mod parsers;
pub mod releases;
pub(crate) mod runner;

use std::process::Command;

pub use error::CliError;
pub use github::GitHubCli;
pub use gitlab::GitLabCli;

/// Configure a [`Command`] to suppress the console window on Windows.
///
/// On Windows, this sets `CREATE_NO_WINDOW` (0x08000000) to prevent a visible
/// console window from flashing when spawning CLI subprocesses.
/// On other platforms this is a no-op.
#[cfg(target_os = "windows")]
pub(crate) fn configure_no_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000);
}

/// No-op on non-Windows platforms.
#[cfg(not(target_os = "windows"))]
pub(crate) fn configure_no_window(_cmd: &mut Command) {}
