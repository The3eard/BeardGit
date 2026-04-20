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

/// Clamp a raw UI-scale value to the 80–150 percentage band.
///
/// Extracted so unit tests can exercise the clamp without a live
/// `State<AppState>`; the command wrapper delegates here verbatim.
pub(crate) fn clamp_ui_scale(scale: u32) -> u32 {
    scale.clamp(80, 150)
}

/// Set the UI scale percentage and persist. Clamped to 80-150.
#[tauri::command]
pub fn set_ui_scale(scale: u32, state: State<'_, AppState>) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    config.ui_scale = clamp_ui_scale(scale);
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

/// Normalize the raw `worktree_root` input: trim whitespace and treat an
/// empty string as `None` so the frontend can clear the override by
/// submitting either `null` or `""`.
pub(crate) fn normalize_worktree_root(raw: Option<String>) -> Option<String> {
    raw.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Floor the AI-background concurrency cap at 1 so an errant 0 from the
/// frontend doesn't stall every background run.
pub(crate) fn clamp_concurrency_cap(cap: u32) -> u32 {
    cap.max(1)
}

/// Persist AI background settings. `concurrency_cap` is clamped to at
/// least 1.
#[tauri::command]
pub fn ai_background_set_settings(
    settings: AiBackgroundSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.ai_worktree_root = normalize_worktree_root(settings.worktree_root);
    config.ai_background_concurrency_cap = clamp_concurrency_cap(settings.concurrency_cap);
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

#[cfg(test)]
mod tests {
    //! Exercises the pure helpers (`clamp_ui_scale`,
    //! `normalize_worktree_root`, `clamp_concurrency_cap`) and verifies the
    //! `AppConfig::save` → `AppConfig::load` roundtrip against a tempdir
    //! config path — the file-backed persistence that every setter
    //! ultimately relies on.

    use super::{clamp_concurrency_cap, clamp_ui_scale, normalize_worktree_root};
    use storage::AppConfig;
    use tempfile::tempdir;

    #[test]
    fn clamp_ui_scale_clamps_below_80_and_above_150() {
        assert_eq!(clamp_ui_scale(50), 80);
        assert_eq!(clamp_ui_scale(200), 150);
        // In-range value passes through unchanged.
        assert_eq!(clamp_ui_scale(125), 125);
    }

    #[test]
    fn normalize_worktree_root_trims_and_nulls_empty() {
        assert_eq!(normalize_worktree_root(None), None);
        assert_eq!(normalize_worktree_root(Some("".to_string())), None);
        assert_eq!(
            normalize_worktree_root(Some("   ".to_string())),
            None,
            "whitespace-only input should collapse to None"
        );
        assert_eq!(
            normalize_worktree_root(Some("  /tmp/wt  ".to_string())),
            Some("/tmp/wt".to_string()),
            "whitespace around a real path is trimmed"
        );
    }

    #[test]
    fn clamp_concurrency_cap_floors_at_one() {
        assert_eq!(clamp_concurrency_cap(0), 1);
        assert_eq!(clamp_concurrency_cap(1), 1);
        assert_eq!(clamp_concurrency_cap(8), 8);
    }

    #[test]
    fn app_config_save_then_load_roundtrips_to_tempdir() {
        // The command layer saves via `config.save(&state.config_path)`; this
        // test uses a tempdir to verify the file-backed roundtrip works
        // without mutating the real user config.
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("config.json");

        let mut cfg = AppConfig::default();
        cfg.locale = "fr-FR".to_string();
        cfg.ui_scale = 125;
        cfg.sidebar_collapsed = true;
        cfg.auto_check_updates = false;
        cfg.save(&path).expect("save");

        let loaded = AppConfig::load(&path).expect("load");
        assert_eq!(loaded.locale, "fr-FR");
        assert_eq!(loaded.ui_scale, 125);
        assert!(loaded.sidebar_collapsed);
        assert!(!loaded.auto_check_updates);
    }

    #[test]
    fn app_config_load_on_missing_file_returns_default() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("does-not-exist.json");
        let loaded = AppConfig::load(&path).expect("load on missing");
        // The default locale is either "en" or system-derived — just assert
        // the file-missing path doesn't error out (it yields defaults).
        assert!(
            !loaded.locale.is_empty(),
            "default config should have a non-empty locale"
        );
    }
}
