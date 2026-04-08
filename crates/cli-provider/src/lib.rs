//! CLI-based provider operations wrapping `gh` and `glab` binaries.
//!
//! Both CLI tools use `--json` flags for structured JSON output, parsed
//! into the shared MR/PR types. The binary paths are resolved from the
//! app's bundled resources directory.

pub mod auth;
pub mod detail;
pub mod error;
pub mod list;
pub mod review;
pub mod types;
pub mod write;

use std::path::{Path, PathBuf};
use std::process::Command;

use provider::ProviderKind;

pub use error::CliError;
pub use types::*;

/// CLI provider that dispatches to `gh` or `glab` based on the provider kind.
pub struct CliProvider {
    /// Which provider this instance represents.
    pub kind: ProviderKind,
    /// Absolute path to the CLI binary (`gh` or `glab`).
    pub binary_path: PathBuf,
    /// Working directory for CLI commands (the repo root).
    pub repo_path: PathBuf,
}

impl CliProvider {
    /// Create a new CLI provider.
    ///
    /// `binary_path` must point to the bundled `gh` or `glab` executable.
    /// `repo_path` is used as the working directory so the CLI auto-detects
    /// the remote and project.
    pub fn new(
        kind: ProviderKind,
        binary_path: impl Into<PathBuf>,
        repo_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            kind,
            binary_path: binary_path.into(),
            repo_path: repo_path.into(),
        }
    }

    /// Run the CLI binary with the given arguments and return stdout.
    pub(crate) fn run(&self, args: &[&str]) -> Result<String, CliError> {
        if !self.binary_path.exists() && !Self::is_path_binary(&self.binary_path) {
            return Err(CliError::BinaryNotFound(
                self.binary_path.display().to_string(),
            ));
        }

        let mut cmd = Command::new(&self.binary_path);
        cmd.args(args).current_dir(&self.repo_path);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let output = cmd.output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(CliError::CommandFailed(stderr))
        }
    }

    /// Run the CLI and parse JSON output.
    pub(crate) fn run_json<T: serde::de::DeserializeOwned>(
        &self,
        args: &[&str],
    ) -> Result<T, CliError> {
        let stdout = self.run(args)?;
        serde_json::from_str(&stdout).map_err(|e| CliError::JsonError(e.to_string()))
    }

    /// Run a CLI command with stdin input.
    pub(crate) fn run_with_stdin(&self, args: &[&str], stdin_data: &str) -> Result<String, CliError> {
        use std::io::Write;
        use std::process::Stdio;

        if !self.binary_path.exists() && !Self::is_path_binary(&self.binary_path) {
            return Err(CliError::BinaryNotFound(
                self.binary_path.display().to_string(),
            ));
        }

        let mut cmd = Command::new(&self.binary_path);
        cmd.args(args)
            .current_dir(&self.repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }

        let mut child = cmd.spawn()?;

        if let Some(ref mut stdin) = child.stdin {
            stdin.write_all(stdin_data.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(CliError::CommandFailed(stderr))
        }
    }

    /// Check if a path refers to a binary name resolvable on PATH (no directory component).
    fn is_path_binary(path: &Path) -> bool {
        path.parent().is_none_or(|p| p.as_os_str().is_empty())
    }

    /// List open MR/PRs for the current repository.
    ///
    /// Filters by state. Returns newest-first. Delegates to the
    /// provider-specific implementation in `list.rs`.
    pub fn list_mr_prs(
        &self,
        state_filter: Option<MrPrState>,
        limit: u32,
    ) -> Result<Vec<MrPr>, CliError> {
        self.list_mr_prs_impl(state_filter, limit)
    }

    /// Fetch detailed information about a single MR/PR.
    ///
    /// Delegates to the provider-specific implementation in `detail.rs`.
    pub fn get_mr_pr_detail(&self, number: u64) -> Result<MrPrDetail, CliError> {
        self.get_mr_pr_detail_impl(number)
    }

    /// Get the list of changed files in a MR/PR diff.
    ///
    /// Delegates to the provider-specific implementation in `detail.rs`.
    pub fn get_mr_pr_diff(&self, number: u64) -> Result<Vec<MrPrDiffFile>, CliError> {
        self.get_mr_pr_diff_impl(number)
    }
}
