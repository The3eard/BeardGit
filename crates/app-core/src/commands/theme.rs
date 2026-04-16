//! Theme listing, selection, auto-detection, and startup resolution commands.

use tauri::{AppHandle, State};

use crate::state::AppState;

/// List all available themes (built-in + user-installed).
#[tauri::command]
pub fn list_themes(state: State<'_, AppState>) -> Vec<storage::ThemeMeta> {
    let _ = &state;
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let themes_dir = config_dir.join("themes");
    let _ = storage::theme::ensure_themes_dir(&themes_dir);
    storage::theme::list_all_themes(&themes_dir)
}

/// Resolve a full theme by name (built-in or user file).
#[tauri::command]
pub fn get_theme(name: String, state: State<'_, AppState>) -> storage::Theme {
    let _ = &state;
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let themes_dir = config_dir.join("themes");
    storage::theme::resolve_theme(&name, &themes_dir)
}

/// Set the active theme name and emit a `theme-changed` event with the resolved theme.
#[tauri::command]
pub fn set_theme(name: String, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let config_path = config_dir.join("settings.json");
    let mut config = storage::AppConfig::load(&config_path).unwrap_or_default();
    config.theme = name.clone();
    config.save(&config_path).map_err(|e| e.to_string())?;

    // Also update the in-memory config
    let mut cfg = state.config.lock().unwrap();
    cfg.theme = name.clone();

    let themes_dir = config_dir.join("themes");
    let theme = storage::theme::resolve_theme(&name, &themes_dir);
    use tauri::Emitter as _;
    let _ = app.emit("theme-changed", &theme);
    Ok(())
}

/// Get the current `theme_auto` setting.
#[tauri::command]
pub fn get_theme_auto(_state: State<'_, AppState>) -> bool {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let config_path = config_dir.join("settings.json");
    storage::AppConfig::load(&config_path)
        .map(|c| c.theme_auto)
        .unwrap_or(true)
}

/// Set the `theme_auto` preference and persist to config.
#[tauri::command]
pub fn set_theme_auto(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let config_path = config_dir.join("settings.json");
    let mut config = storage::AppConfig::load(&config_path).unwrap_or_default();
    config.theme_auto = enabled;
    config.save(&config_path).map_err(|e| e.to_string())?;

    // Also update the in-memory config
    let mut cfg = state.config.lock().unwrap();
    cfg.theme_auto = enabled;

    Ok(())
}

/// Resolve the startup theme, respecting the `theme_auto` setting and OS dark/light mode.
#[tauri::command]
pub fn resolve_startup_theme(app: AppHandle, _state: State<'_, AppState>) -> storage::Theme {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let config_path = config_dir.join("settings.json");
    let themes_dir = config_dir.join("themes");
    let _ = storage::theme::ensure_themes_dir(&themes_dir);

    let config = storage::AppConfig::load(&config_path).unwrap_or_default();

    let theme_id = if config.theme_auto {
        use tauri::Manager as _;
        let os_dark = app
            .get_webview_window("main")
            .and_then(|w| w.theme().ok())
            .map(|t| matches!(t, tauri::Theme::Dark))
            .unwrap_or(true);

        resolve_theme_for_mode(&config.theme, os_dark)
    } else {
        config.theme.clone()
    };

    storage::theme::resolve_theme(&theme_id, &themes_dir)
}

/// Given a base theme id and whether the OS is in dark mode, resolve the correct variant.
///
/// Delegates to `storage::theme::resolve_theme_for_mode` which uses the
/// `complementary` field from theme metadata instead of string replacement.
pub fn resolve_theme_for_mode(base: &str, os_dark: bool) -> String {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let themes_dir = config_dir.join("themes");
    storage::theme::resolve_theme_for_mode(base, os_dark, &themes_dir)
}
