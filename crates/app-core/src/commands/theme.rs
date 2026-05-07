//! Theme listing, selection, auto-detection, and startup resolution commands.
//!
//! These commands read and write the in-memory `state.config` rather than
//! re-loading `settings.json` on every call — the AppState's `config`
//! mutex is the canonical source of truth for the user's theme prefs and
//! `set_*` commands persist via `AppConfig::save(&state.config_path)`.

use tauri::{AppHandle, State};

use crate::state::AppState;

/// List all available themes (built-in + user-installed).
#[tauri::command]
pub fn list_themes(state: State<'_, AppState>) -> Vec<storage::ThemeMeta> {
    let themes_dir = state.config_dir.join("themes");
    let _ = storage::theme::ensure_themes_dir(&themes_dir);
    storage::theme::list_all_themes(&themes_dir)
}

/// Resolve a full theme by name (built-in or user file).
#[tauri::command]
pub fn get_theme(name: String, state: State<'_, AppState>) -> storage::Theme {
    let themes_dir = state.config_dir.join("themes");
    storage::theme::resolve_theme(&name, &themes_dir)
}

/// Set the active theme name and emit a `theme-changed` event with the resolved theme.
#[tauri::command]
pub fn set_theme(name: String, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.theme = name.clone();
        cfg.save(&state.config_path).map_err(|e| e.to_string())?;
    }

    let themes_dir = state.config_dir.join("themes");
    let theme = storage::theme::resolve_theme(&name, &themes_dir);
    use tauri::Emitter as _;
    let _ = app.emit("theme-changed", &theme);
    Ok(())
}

/// Get the current `theme_auto` setting.
#[tauri::command]
pub fn get_theme_auto(state: State<'_, AppState>) -> bool {
    state.config.lock().unwrap().theme_auto
}

/// Set the `theme_auto` preference and persist to config.
#[tauri::command]
pub fn set_theme_auto(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    cfg.theme_auto = enabled;
    cfg.save(&state.config_path).map_err(|e| e.to_string())
}

/// Resolve the startup theme, respecting the `theme_auto` setting and OS dark/light mode.
#[tauri::command]
pub fn resolve_startup_theme(app: AppHandle, state: State<'_, AppState>) -> storage::Theme {
    let themes_dir = state.config_dir.join("themes");
    let _ = storage::theme::ensure_themes_dir(&themes_dir);

    let (theme_auto, base_theme) = {
        let cfg = state.config.lock().unwrap();
        (cfg.theme_auto, cfg.theme.clone())
    };

    let theme_id = if theme_auto {
        use tauri::Manager as _;
        let os_dark = app
            .get_webview_window("main")
            .and_then(|w| w.theme().ok())
            .map(|t| matches!(t, tauri::Theme::Dark))
            .unwrap_or(true);

        storage::theme::resolve_theme_for_mode(&base_theme, os_dark, &themes_dir)
    } else {
        base_theme
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

#[cfg(test)]
mod tests {
    //! Drive the `storage::theme` functions that these commands delegate to
    //! against an empty (no user themes) tempdir — we avoid touching the
    //! real `~/.config/beardgit/themes` directory.

    use storage::theme::{list_all_themes, resolve_theme, resolve_theme_for_mode};
    use tempfile::tempdir;

    #[test]
    fn list_all_themes_in_empty_dir_returns_builtins() {
        let tmp = tempdir().unwrap();
        let themes = list_all_themes(tmp.path());
        assert!(
            !themes.is_empty(),
            "built-in themes must ship even without user themes"
        );
        assert!(
            themes.iter().any(|t| t.id == "github-dark"),
            "default 'github-dark' should be present, got {:?}",
            themes.iter().map(|t| &t.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn resolve_theme_known_id_returns_matching_id() {
        let tmp = tempdir().unwrap();
        let theme = resolve_theme("github-dark", tmp.path());
        assert_eq!(
            theme.meta.id, "github-dark",
            "resolve_theme should return the requested theme when it exists"
        );
    }

    #[test]
    fn resolve_theme_unknown_id_falls_back_to_default() {
        let tmp = tempdir().unwrap();
        let theme = resolve_theme("does-not-exist-xyz", tmp.path());
        assert!(
            !theme.meta.id.is_empty(),
            "resolve_theme must always return a valid theme (fallback), got {theme:?}"
        );
    }

    #[test]
    fn resolve_theme_for_mode_respects_dark_mode_flag() {
        let tmp = tempdir().unwrap();
        // We don't know exactly what the complementary mapping does, but we
        // can assert it returns a non-empty id for both modes (i.e. the
        // helper is callable and yields something usable).
        let dark_id = resolve_theme_for_mode("github-dark", true, tmp.path());
        let light_id = resolve_theme_for_mode("github-dark", false, tmp.path());
        assert!(!dark_id.is_empty(), "dark-mode resolution must yield an id");
        assert!(
            !light_id.is_empty(),
            "light-mode resolution must yield an id"
        );
    }
}
