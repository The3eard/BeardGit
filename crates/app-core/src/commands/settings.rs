//! UI settings commands: locale, scale, graph columns, sidebar state.

use tauri::State;

use crate::state::AppState;

/// Return the persisted UI locale tag (e.g. `"en-US"`).
#[tauri::command]
pub fn get_locale(state: State<'_, AppState>) -> String {
    let config = state.config.lock().unwrap();
    config.locale.clone()
}

/// Change the persisted UI locale tag.
#[tauri::command]
pub fn set_locale(locale: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    config.locale = locale;
    config.save(&state.config_path).map_err(|e| e.to_string())
}

/// Return the current UI scale percentage (80-150).
#[tauri::command]
pub fn get_ui_scale(state: State<'_, AppState>) -> u32 {
    let config = state.config.lock().unwrap();
    config.ui_scale
}

/// Set the UI scale percentage and persist. Clamped to 80-150.
#[tauri::command]
pub fn set_ui_scale(scale: u32, state: State<'_, AppState>) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    config.ui_scale = scale.clamp(80, 150);
    config.save(&state.config_path).map_err(|e| e.to_string())
}

/// Return the persisted graph column configuration.
#[tauri::command]
pub fn get_graph_columns(state: State<'_, AppState>) -> Vec<storage::GraphColumnConfig> {
    let config = state.config.lock().unwrap();
    config.graph_columns.clone()
}

/// Persist graph column configuration (visibility + widths).
#[tauri::command]
pub fn set_graph_columns(
    columns: Vec<storage::GraphColumnConfig>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    config.graph_columns = columns;
    config.save(&state.config_path).map_err(|e| e.to_string())
}

/// Get the persisted sidebar collapsed state.
#[tauri::command]
pub fn get_sidebar_collapsed(state: State<'_, AppState>) -> Result<bool, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.sidebar_collapsed)
}

/// Persist sidebar collapsed state.
#[tauri::command]
pub fn set_sidebar_collapsed(collapsed: bool, state: State<'_, AppState>) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.sidebar_collapsed = collapsed;
    config.save(&state.config_path).map_err(|e| e.to_string())
}

/// Return whether the app should silently probe for updates on startup.
///
/// Defaults to `true`. Exposed via the settings IPC so the frontend's
/// `runStartupCheck()` in `autoUpdate.ts` can short-circuit when the
/// user has opted out.
#[tauri::command]
pub fn get_auto_check_updates(state: State<'_, AppState>) -> Result<bool, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.auto_check_updates)
}

/// Persist the `auto_check_updates` preference. The startup toast
/// behaviour flips on the next cold-start.
#[tauri::command]
pub fn set_auto_check_updates(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.auto_check_updates = enabled;
    config.save(&state.config_path).map_err(|e| e.to_string())
}

/// Return whether the per-OS re-authorization notice has been dismissed.
///
/// `os` must be `"macos"` or `"windows"`; other values return `false`
/// (Linux never shows the dialog, so the frontend never probes for it).
#[tauri::command]
pub fn get_reauth_dismissed(os: String, state: State<'_, AppState>) -> Result<bool, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(match os.as_str() {
        "macos" => config.auto_update_reauth_notice_dismissed_macos,
        "windows" => config.auto_update_reauth_notice_dismissed_windows,
        _ => false,
    })
}

/// Persist the re-authorization-notice dismissal for a single OS.
///
/// `os` must be `"macos"` or `"windows"`; other values are ignored so
/// the frontend can't accidentally poison the config with arbitrary keys.
#[tauri::command]
pub fn set_reauth_dismissed(
    os: String,
    dismissed: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    match os.as_str() {
        "macos" => config.auto_update_reauth_notice_dismissed_macos = dismissed,
        "windows" => config.auto_update_reauth_notice_dismissed_windows = dismissed,
        _ => return Ok(()),
    }
    config.save(&state.config_path).map_err(|e| e.to_string())
}

// ─── AI background settings (Phase 10) ───────────────────────────────────

/// Serialisable view of the AI background settings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiBackgroundSettings {
    /// Override for the worktree root (None = use default).
    pub worktree_root: Option<String>,
    /// Concurrent-run cap (1+).
    pub concurrency_cap: u32,
    /// Pass the provider's permission-skip flag.
    pub auto_accept_permissions: bool,
}

/// Read current AI background settings from config.
#[tauri::command]
pub fn ai_background_get_settings(
    state: State<'_, AppState>,
) -> Result<AiBackgroundSettings, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(AiBackgroundSettings {
        worktree_root: config.ai_worktree_root.clone(),
        concurrency_cap: config.ai_background_concurrency_cap,
        auto_accept_permissions: config.ai_prompt_auto_accept,
    })
}

/// Persist AI background settings. `concurrency_cap` is clamped to at
/// least 1.
#[tauri::command]
pub fn ai_background_set_settings(
    settings: AiBackgroundSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.ai_worktree_root = settings
        .worktree_root
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    config.ai_background_concurrency_cap = settings.concurrency_cap.max(1);
    config.ai_prompt_auto_accept = settings.auto_accept_permissions;
    config.save(&state.config_path).map_err(|e| e.to_string())
}

/// Load a project's cached snapshot for instant UI display.
#[tauri::command]
pub fn get_project_snapshot(path: String) -> Result<Option<storage::ProjectSnapshot>, String> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    storage::project_cache::load_snapshot(&config_dir, &path).map_err(|e| e.to_string())
}

/// Save a project's snapshot to the cache.
#[tauri::command]
pub fn save_project_snapshot(snapshot: storage::ProjectSnapshot) -> Result<(), String> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    storage::project_cache::save_snapshot(&config_dir, &snapshot).map_err(|e| e.to_string())
}
