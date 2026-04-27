//! Debug info and log file commands.
//!
//! ## Integration checklist (src-tauri/src/lib.rs)
//!
//! 1. Add to `generate_handler![]`:
//!    ```text
//!    app_core::commands::get_debug_info,
//!    app_core::commands::get_log_path,
//!    app_core::commands::open_log_directory,
//!    ```
//!
//! 2. Add logging init at the top of the `.setup()` closure:
//!    ```text
//!    storage::logging::init_logging().ok();
//!    ```

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
pub fn open_log_directory() -> Result<(), String> {
    let path = storage::logging::log_directory();
    open::that(&path).map_err(|e| format!("failed to open log directory: {e}"))
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
