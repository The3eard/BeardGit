//! Structured file logging with daily rotation.
//!
//! Writes logs to platform-specific directories:
//! - macOS: `~/Library/Logs/BeardGit/`
//! - Linux: `~/.local/share/beardgit/logs/`
//! - Windows: `%APPDATA%/BeardGit/logs/`

use std::path::PathBuf;

use tracing_appender::rolling;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Debug information for error reports and the "About" screen.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DebugInfo {
    /// Application version from Cargo metadata.
    pub app_version: String,
    /// Operating system and architecture (e.g. `"macos aarch64"`).
    pub os: String,
    /// CPU architecture (e.g. `"aarch64"`).
    pub arch: String,
    /// Output of `git --version`, if git is on PATH.
    pub git_version: Option<String>,
    /// Filesystem path to the log directory.
    pub log_path: String,
}

/// Get the platform-specific log directory.
pub fn log_directory() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_default()
            .join("Library/Logs/BeardGit")
    }
    #[cfg(target_os = "linux")]
    {
        dirs::data_local_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
            .join("beardgit/logs")
    }
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir().unwrap_or_default().join("BeardGit/logs")
    }
}

/// Initialize the global tracing subscriber with file logging.
///
/// Creates a daily-rotating log file in the platform log directory.
/// The non-blocking writer guard is intentionally leaked so it stays alive
/// for the entire process lifetime.
pub fn init_logging() -> Result<(), String> {
    let log_dir = log_directory();
    std::fs::create_dir_all(&log_dir).map_err(|e| format!("failed to create log dir: {e}"))?;

    let file_appender = rolling::daily(&log_dir, "beardgit.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Keep the guard alive for the lifetime of the app.
    // We leak it intentionally — it is a singleton that lives until process exit.
    std::mem::forget(guard);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,git_engine=debug,app_core=debug"));

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        "BeardGit logging initialized"
    );

    Ok(())
}

/// Collect debug information about the running application.
///
/// Queries the system git binary for its version string and gathers
/// platform metadata for error reports.
pub fn collect_debug_info() -> DebugInfo {
    let git_version = std::process::Command::new("git")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    DebugInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        arch: std::env::consts::ARCH.to_string(),
        git_version,
        log_path: log_directory().to_string_lossy().into_owned(),
    }
}
