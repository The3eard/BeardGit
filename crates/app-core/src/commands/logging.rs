//! Debug info, log file, and log-level commands.
//!
//! ## Integration checklist (src-tauri/src/lib.rs)
//!
//! 1. Add to `generate_handler![]`:
//!    ```text
//!    app_core::commands::get_debug_info,
//!    app_core::commands::get_log_path,
//!    app_core::commands::open_log_directory,
//!    app_core::commands::get_log_level,
//!    app_core::commands::set_log_level,
//!    ```
//!
//! 2. Add logging init at the top of the `.setup()` closure, seeded with
//!    the persisted level from `AppState::config`:
//!    ```text
//!    storage::logging::init_logging(&level).ok();
//!    ```

use tauri::State;

use crate::ipc_error::IpcError;
use crate::state::AppState;

/// Get debug information for error reports.
#[tauri::command]
pub fn get_debug_info() -> storage::logging::DebugInfo {
    storage::logging::collect_debug_info()
}

/// Get the log file directory path.
#[tauri::command]
pub fn get_log_path() -> String {
    storage::logging::log_directory()
        .to_string_lossy()
        .into_owned()
}

/// Open the log directory in the system file manager.
#[tauri::command]
pub fn open_log_directory() -> Result<(), IpcError> {
    let path = storage::logging::log_directory();
    open::that(&path)
        .map_err(|e| IpcError::from(format!("failed to open log directory: {e}")))
}

/// Return the effective file-log verbosity (`"error"` / `"info"` / `"debug"`).
///
/// Reads the in-memory `AppConfig` rather than the live subscriber: the
/// two only diverge when `RUST_LOG` overrode the startup filter, and the
/// selector should reflect the user's own choice.
///
/// Normalized on the way out, because `AppConfig::log_level` is a plain
/// `String` that nothing validates on load. A hand-edited `"DEBUG"` is
/// what `init_logging` actually applied, so returning it verbatim would
/// make the selector show `Info` while the app logged at debug.
#[tauri::command]
pub fn get_log_level(state: State<'_, AppState>) -> Result<String, IpcError> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(storage::logging::normalize_level(&config.log_level)
        .unwrap_or("info")
        .to_string())
}

/// Persist a new file-log verbosity and apply it to the running subscriber.
///
/// Validation happens first, then the live swap, and only then the write
/// to disk — so a rejected level never lands in `settings.json`, and a
/// persisted level is always one that actually took effect.
#[tauri::command]
pub fn set_log_level(level: String, state: State<'_, AppState>) -> Result<(), IpcError> {
    let normalized = storage::logging::normalize_level(&level)
        .ok_or_else(|| IpcError::new("invalid_log_level", format!("unknown log level: {level}")))?;

    storage::logging::set_level(normalized).map_err(|e| IpcError::new("log_level_failed", e))?;

    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.log_level = normalized.to_string();
    config.save(&state.config_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! These commands are thin delegates into `storage::logging`. We test
    //! their observable output without relying on a live Tauri runtime.

    use super::{get_debug_info, get_log_path};

    #[test]
    fn get_log_path_returns_non_empty_platform_path() {
        let path = get_log_path();
        assert!(!path.is_empty(), "log directory path must not be empty");
        // The returned path should end with something logs-ish — the
        // platform resolves it but every branch we care about appends
        // "logs" or the app name. Keep the assertion permissive: just
        // that it's absolute-ish (contains a separator).
        assert!(
            path.contains(std::path::MAIN_SEPARATOR) || path.contains('/'),
            "log path should be a real filesystem path, got {path:?}"
        );
    }

    #[test]
    fn get_debug_info_fills_core_fields() {
        let info = get_debug_info();
        assert!(
            !info.app_version.is_empty(),
            "app_version should come from CARGO_PKG_VERSION"
        );
        assert!(!info.os.is_empty(), "os string should be populated");
        assert!(!info.arch.is_empty(), "arch string should be populated");
        assert!(!info.log_path.is_empty(), "log path should be populated");
        // git_version is an Option<String> — populated when git is on PATH.
        // On developer machines and CI the system git is always present;
        // keep this as a soft check so environments without git still pass.
        if let Some(ref v) = info.git_version {
            assert!(
                v.contains("git") || !v.is_empty(),
                "git_version string should not be empty, got {v:?}"
            );
        }
    }
}
