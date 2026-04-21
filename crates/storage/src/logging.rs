//! Structured file logging with daily rotation.
//!
//! Writes logs to platform-specific directories:
//! - macOS: `~/Library/Logs/BeardGit/`
//! - Linux: `~/.local/share/beardgit/logs/`
//! - Windows: `%APPDATA%/BeardGit/logs/`

use std::path::PathBuf;

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

/// Delete log files older than `max_age_days` from `log_dir`.
///
/// Only removes files whose names contain `"log"`. This matches both the
/// current `beardgit.{date}.log` layout and any legacy `beardgit.log.{date}`
/// files left behind by pre-rename installs. Returns the number of files deleted.
///
/// # Errors
/// Returns an I/O error if the directory cannot be read.
pub fn purge_old_logs(log_dir: &std::path::Path, max_age_days: u64) -> std::io::Result<usize> {
    use std::time::{Duration, SystemTime};

    let cutoff = SystemTime::now() - Duration::from_secs(max_age_days * 86400);
    let mut deleted = 0usize;

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only consider files whose name contains "log"
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.contains("log") => n.to_string(),
            _ => continue,
        };

        // Skip directories
        if !path.is_file() {
            continue;
        }

        // Check modification time
        let metadata = std::fs::metadata(&path)?;
        let modified = metadata.modified()?;

        if modified < cutoff {
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!(file = %name, error = %e, "Failed to remove old log file");
            } else {
                deleted += 1;
            }
        }
    }

    if deleted > 0 {
        tracing::info!(count = deleted, "Purged old log files");
    }

    Ok(deleted)
}

/// Build the daily-rotating file appender used by `init_logging`.
///
/// Filename layout: `beardgit.{YYYY-MM-DD}.log` — the `.log` suffix is last
/// so `*.log` globs and standard log viewers recognize the file.
fn build_file_appender(
    log_dir: &std::path::Path,
) -> tracing_appender::rolling::RollingFileAppender {
    tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("beardgit")
        .filename_suffix("log")
        .build(log_dir)
        .expect("rolling file appender builder should not fail for a valid directory")
}

/// Initialize the global tracing subscriber with file logging.
///
/// Creates a daily-rotating log file in the platform log directory.
/// The non-blocking writer guard is intentionally leaked so it stays alive
/// for the entire process lifetime.
pub fn init_logging() -> Result<(), String> {
    let log_dir = log_directory();
    std::fs::create_dir_all(&log_dir).map_err(|e| format!("failed to create log dir: {e}"))?;

    let file_appender = build_file_appender(&log_dir);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};

    /// Helper: create a log file with a modified time set to `days_ago` days in the past.
    fn create_aged_log(dir: &std::path::Path, name: &str, days_ago: u64) {
        let path = dir.join(name);
        fs::write(&path, "log content").unwrap();
        let age = SystemTime::now() - Duration::from_secs(days_ago * 86400);
        filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(age)).unwrap();
    }

    #[test]
    fn purge_deletes_old_logs() {
        let tmp = tempfile::tempdir().unwrap();
        create_aged_log(tmp.path(), "beardgit.2026-04-01.log", 10);
        create_aged_log(tmp.path(), "beardgit.2026-04-10.log", 3);

        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 1);
        assert!(!tmp.path().join("beardgit.2026-04-01.log").exists());
        assert!(tmp.path().join("beardgit.2026-04-10.log").exists());
    }

    #[test]
    fn purge_ignores_non_log_files() {
        let tmp = tempfile::tempdir().unwrap();
        create_aged_log(tmp.path(), "beardgit.2026-04-01.log", 10);
        create_aged_log(tmp.path(), "settings.json", 10);

        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 1);
        assert!(tmp.path().join("settings.json").exists());
    }

    #[test]
    fn purge_returns_zero_on_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn purge_handles_nonexistent_dir() {
        let result = purge_old_logs(std::path::Path::new("/nonexistent/path"), 7);
        assert!(result.is_err());
    }

    #[test]
    fn purge_keeps_all_when_none_old_enough() {
        let tmp = tempfile::tempdir().unwrap();
        create_aged_log(tmp.path(), "beardgit.2026-04-14.log", 2);
        create_aged_log(tmp.path(), "beardgit.2026-04-15.log", 1);

        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn purge_matches_new_filename_pattern() {
        let tmp = tempfile::tempdir().unwrap();
        // New shape — old enough to purge.
        create_aged_log(tmp.path(), "beardgit.2026-04-01.log", 10);
        // New shape — recent, should survive.
        create_aged_log(tmp.path(), "beardgit.2026-04-20.log", 1);

        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 1);
        assert!(!tmp.path().join("beardgit.2026-04-01.log").exists());
        assert!(tmp.path().join("beardgit.2026-04-20.log").exists());
    }

    #[test]
    fn purge_handles_legacy_filenames_without_crashing() {
        // Legacy `beardgit.log.{date}` files may linger from pre-rename installs.
        // Rotation should treat them like any other log file: age-based purge, no panic.
        let tmp = tempfile::tempdir().unwrap();
        create_aged_log(tmp.path(), "beardgit.log.2026-04-01", 10); // legacy, old
        create_aged_log(tmp.path(), "beardgit.2026-04-20.log", 1); // new, recent

        let deleted = purge_old_logs(tmp.path(), 7).unwrap();
        assert_eq!(deleted, 1, "legacy old file should be purged by age");
        assert!(!tmp.path().join("beardgit.log.2026-04-01").exists());
        assert!(tmp.path().join("beardgit.2026-04-20.log").exists());
    }

    #[test]
    fn init_logging_produces_filename_matching_new_pattern() {
        // The rolling appender writes `beardgit.{YYYY-MM-DD}.log`.
        // We build the appender via the production helper to assert
        // the filename shape without touching the global subscriber.
        let tmp = tempfile::tempdir().unwrap();
        let appender = build_file_appender(tmp.path());

        // Force a write so the file is created.
        use std::io::Write;
        let mut w = appender;
        writeln!(w, "probe").unwrap();
        drop(w);

        let entries: Vec<String> = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok().and_then(|e| e.file_name().into_string().ok()))
            .collect();

        assert_eq!(
            entries.len(),
            1,
            "expected exactly one log file, got {entries:?}"
        );
        let name = &entries[0];
        assert!(
            name.starts_with("beardgit.") && name.ends_with(".log"),
            "filename {name:?} does not match beardgit.{{date}}.log"
        );
        // Reject the legacy shape: prefix `beardgit.log.` means the `.log`
        // slot is in the middle, which is exactly what we are fixing.
        assert!(
            !name.starts_with("beardgit.log."),
            "filename {name:?} still uses the legacy beardgit.log.{{date}} shape"
        );
    }
}
