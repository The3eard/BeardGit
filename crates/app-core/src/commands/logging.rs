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
