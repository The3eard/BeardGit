//! Application configuration persisted as a JSON file.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::StorageError;

fn default_theme() -> String {
    "github-dark".to_string()
}

fn default_theme_auto() -> bool {
    true
}

fn default_auto_check_updates() -> bool {
    true
}

fn default_locale() -> String {
    "en-US".to_string()
}

fn default_ui_scale() -> u32 {
    100
}

fn default_ai_background_concurrency_cap() -> u32 {
    3
}

/// Default editor-preferences value used by `serde(default = …)` so old
/// config files (written before the editor preferences existed) load
/// cleanly with the canonical defaults filled in.
pub fn default_editor_preferences() -> EditorPreferences {
    EditorPreferences::default()
}

/// User-tunable preferences for the in-app mini editor.
///
/// All extension toggles default to the values most users expect from a
/// modern code editor (most ON; rectangular-selection / crosshair OFF
/// because they're niche). `respect_gitignore_in_tree` defaults to `false`
/// per the product brief — the file tree shows `.gitignore`d files unless
/// the user opts into hiding them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorPreferences {
    // --- Toggleable CodeMirror extensions ---
    /// Show the autocomplete popup as the user types.
    pub autocomplete: bool,
    /// Auto-close brackets / quotes when typing the opening character.
    pub close_brackets: bool,
    /// Highlight the matching bracket of the bracket under the cursor.
    pub bracket_matching: bool,
    /// Highlight the line the cursor is currently on.
    pub highlight_active_line: bool,
    /// Highlight every other occurrence of the current selection.
    pub highlight_selection_matches: bool,
    /// Render a fold gutter so users can collapse code regions.
    pub fold_gutter: bool,
    /// Auto-indent on Enter / closing tag.
    pub indent_on_input: bool,
    /// Soft-wrap long lines (no horizontal scroll).
    pub line_wrapping: bool,
    /// Allow rectangular (column) selections with Alt+drag.
    pub rectangular_selection: bool,
    /// Render a crosshair cursor while Alt is held (pairs with rectangular selection).
    pub crosshair_cursor: bool,
    // --- Behavior ---
    /// Number of spaces (or visual width of a tab) per indentation level. Clamped 1..=8.
    pub tab_size: u8,
    /// When true, the editor inserts tab characters; otherwise spaces.
    pub indent_with_tabs: bool,
    /// When true, the file tree hides paths matched by `.gitignore`. Default `false`.
    pub respect_gitignore_in_tree: bool,
    /// File-size threshold (KB) above which the editor warns before opening. Clamped 1..=2048.
    pub large_file_warning_kb: u32,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            autocomplete: true,
            close_brackets: true,
            bracket_matching: true,
            highlight_active_line: true,
            highlight_selection_matches: true,
            fold_gutter: true,
            indent_on_input: true,
            line_wrapping: true,
            rectangular_selection: false,
            crosshair_cursor: false,
            tab_size: 2,
            indent_with_tabs: false,
            respect_gitignore_in_tree: false,
            large_file_warning_kb: 256,
        }
    }
}

/// Canonical order of the Navigation sidebar items. Kept in lockstep with
/// the `DEFAULT_ORDER` constant on the frontend (`src/lib/utils/applyLayout.ts`).
/// When a new nav item ships, append its id here — existing user layouts
/// pick it up via the `applyLayout` tail-merge so nothing is silently dropped.
fn default_sidebar_nav_order() -> Vec<String> {
    vec![
        "graph",
        "changes",
        "editor",
        "branches",
        "tags",
        "stashes",
        "worktrees",
        "reflog",
        "bisect",
        "submodules",
        "ai-config",
        "ai-sessions",
        "requests",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// Persisted provider connection info for auto-reconnect on startup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedProvider {
    /// Provider type: `"gitlab"` or `"github"`.
    pub kind: String,
    /// Base URL of the provider instance.
    pub instance_url: String,
}

/// Persisted graph column configuration (visibility + width).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphColumnConfig {
    /// Column identifier (e.g. "author", "date", "email", "sha").
    pub id: String,
    /// Column pixel width.
    pub width: u32,
    /// Whether the column is visible.
    pub visible: bool,
}

/// Persistent application settings stored in `~/.config/beardgit/settings.json`.
///
/// ## Migration
///
/// Previous versions stored provider info differently:
/// - Plan 5 format: `provider_kind` + `provider_instance_url` (single provider)
/// - Pre-Plan 5: `gitlab_instance_url` (GitLab only)
///
/// [`AppConfig::load`] automatically migrates both old formats into the
/// `providers` vec on read. Legacy fields are never written back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Active theme name (defaults to `"github-dark"`).
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Whether the app should automatically switch between light/dark themes
    /// based on the OS appearance setting.
    #[serde(default = "default_theme_auto")]
    pub theme_auto: bool,
    /// BCP 47 locale tag for the UI language (e.g. `"en-US"`, `"es-ES"`).
    #[serde(default = "default_locale")]
    pub locale: String,
    /// List of recently opened repository paths.
    #[serde(default)]
    pub recent_repos: Vec<String>,
    /// Authenticated providers to auto-reconnect on startup.
    #[serde(default)]
    pub providers: Vec<SavedProvider>,
    /// Command used to open files in an external editor (e.g. `"code"` or `"nvim"`).
    #[serde(default)]
    pub external_editor: Option<String>,
    /// Persisted window width in logical pixels.
    #[serde(default)]
    pub window_width: Option<u32>,
    /// Persisted window height in logical pixels.
    #[serde(default)]
    pub window_height: Option<u32>,

    /// Paths of currently open projects (persisted across restarts).
    #[serde(default)]
    pub open_projects: Vec<String>,

    /// Index into `open_projects` for the last active tab.
    #[serde(default)]
    pub active_project_index: Option<usize>,

    /// UI scale percentage (80–150). Defaults to 100.
    #[serde(default = "default_ui_scale")]
    pub ui_scale: u32,

    /// Persisted graph column layout (visibility and widths).
    #[serde(default)]
    pub graph_columns: Vec<GraphColumnConfig>,

    /// Whether the sidebar is collapsed to icon-only mode.
    #[serde(default)]
    pub sidebar_collapsed: bool,

    /// Persisted order of the Navigation sidebar items (by id). New ids
    /// not present here are appended at the end by the frontend's
    /// `applyLayout` helper.
    #[serde(default = "default_sidebar_nav_order")]
    pub sidebar_nav_order: Vec<String>,

    /// Ids of Navigation sidebar items the user has chosen to hide.
    #[serde(default)]
    pub sidebar_nav_hidden: Vec<String>,

    /// Preferred AI provider kind (e.g. `"claude_code"`, `"codex"`, `"open_code"`).
    /// `None` means "use first detected".
    #[serde(default)]
    pub preferred_ai_provider: Option<String>,

    /// Override for where AI background worktrees get created. When `None`,
    /// defaults to `<repo>/.beardgit/ai-worktrees`. Can be absolute or
    /// repo-relative. The coordinator creates parent directories as needed.
    #[serde(default)]
    pub ai_worktree_root: Option<String>,

    /// Maximum number of concurrent AI background runs. Runs spawned past the
    /// cap are queued and dispatched when a slot frees. Minimum: 1.
    #[serde(default = "default_ai_background_concurrency_cap")]
    pub ai_background_concurrency_cap: u32,

    /// When true, pass the provider's permission-skip flag to headless AI
    /// runs (e.g. Claude Code's `--dangerously-skip-permissions`). Default
    /// `false` — users opt in explicitly because skipping permissions means
    /// the agent can edit any file under the worktree without prompting.
    #[serde(default)]
    pub ai_prompt_auto_accept: bool,

    /// When true, silently probe the updater endpoint on app startup and
    /// surface a toast if a new version is available. Default `true` so
    /// users get updates out of the box; they can disable the probe in
    /// Settings → Updates.
    #[serde(default = "default_auto_check_updates")]
    pub auto_check_updates: bool,

    /// When true, the CodeMirror diff viewer renders spaces and tabs as
    /// visible glyphs (`·` / `→`). Useful for spotting whitespace-only
    /// changes — a removed tab or trailing-space tweak otherwise looks
    /// identical to the original line. Default `false` to avoid noise on
    /// the common case of content edits.
    #[serde(default)]
    pub diff_show_whitespace: bool,

    /// Persisted editor-panel preferences. New users get the
    /// `EditorPreferences::default()` set.
    #[serde(default = "default_editor_preferences")]
    pub editor_preferences: EditorPreferences,

    /// Whether the user has dismissed the macOS Gatekeeper re-authorization
    /// notice. When `true`, the install flow skips the apology dialog on
    /// macOS. Independent from the Windows flag below — users might
    /// dismiss one while still wanting the other.
    #[serde(default)]
    pub auto_update_reauth_notice_dismissed_macos: bool,

    /// Whether the user has dismissed the Windows SmartScreen re-authorization
    /// notice. When `true`, the install flow skips the apology dialog on
    /// Windows.
    #[serde(default)]
    pub auto_update_reauth_notice_dismissed_windows: bool,

    // -- Legacy fields (read during migration, never written) --
    /// Legacy Plan 5 field. Migrated to `providers` vec.
    #[serde(default, skip_serializing)]
    provider_kind: Option<String>,
    /// Legacy Plan 5 field. Migrated to `providers` vec.
    #[serde(default, skip_serializing)]
    provider_instance_url: Option<String>,
    /// Legacy pre-Plan 5 field. Migrated to `providers` vec.
    #[serde(default, skip_serializing)]
    gitlab_instance_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            theme_auto: default_theme_auto(),
            locale: default_locale(),
            recent_repos: Vec::new(),
            providers: Vec::new(),
            external_editor: None,
            window_width: None,
            window_height: None,
            open_projects: Vec::new(),
            active_project_index: None,
            ui_scale: default_ui_scale(),
            graph_columns: Vec::new(),
            sidebar_collapsed: false,
            sidebar_nav_order: default_sidebar_nav_order(),
            sidebar_nav_hidden: Vec::new(),
            preferred_ai_provider: None,
            ai_worktree_root: None,
            ai_background_concurrency_cap: default_ai_background_concurrency_cap(),
            ai_prompt_auto_accept: false,
            auto_check_updates: default_auto_check_updates(),
            diff_show_whitespace: false,
            auto_update_reauth_notice_dismissed_macos: false,
            auto_update_reauth_notice_dismissed_windows: false,
            editor_preferences: EditorPreferences::default(),
            provider_kind: None,
            provider_instance_url: None,
            gitlab_instance_url: None,
        }
    }
}

impl AppConfig {
    /// Load config from a JSON file. Returns the default config if the file doesn't exist.
    ///
    /// Automatically migrates legacy provider fields into the `providers` vec.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let mut config: Self = serde_json::from_str(&content)?;

        // Migrate legacy formats into providers vec (only if vec is empty)
        if config.providers.is_empty() {
            // Plan 5 format: provider_kind + provider_instance_url
            if let (Some(kind), Some(url)) = (
                config.provider_kind.take(),
                config.provider_instance_url.take(),
            ) {
                config.providers.push(SavedProvider {
                    kind,
                    instance_url: url,
                });
            }
            // Pre-Plan 5 format: gitlab_instance_url
            else if let Some(url) = config.gitlab_instance_url.take() {
                config.providers.push(SavedProvider {
                    kind: "gitlab".to_string(),
                    instance_url: url,
                });
            }
        }

        Ok(config)
    }

    /// Save config to a JSON file, creating parent directories as needed.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), StorageError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.theme, "github-dark");
        assert!(config.providers.is_empty());
    }

    #[test]
    fn test_save_and_load_with_providers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = AppConfig {
            providers: vec![
                SavedProvider {
                    kind: "gitlab".to_string(),
                    instance_url: "https://gitlab.com".to_string(),
                },
                SavedProvider {
                    kind: "github".to_string(),
                    instance_url: "https://api.github.com".to_string(),
                },
            ],
            ..AppConfig::default()
        };
        config.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(loaded.providers.len(), 2);
        assert_eq!(loaded.providers[0].kind, "gitlab");
        assert_eq!(loaded.providers[1].kind, "github");
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let config = AppConfig::load(&path).unwrap();
        assert!(config.providers.is_empty());
    }

    #[test]
    fn test_migrate_plan5_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let json = r#"{
            "theme": "gitlab-dark",
            "provider_kind": "github",
            "provider_instance_url": "https://api.github.com"
        }"#;
        std::fs::write(&path, json).unwrap();

        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.providers[0].kind, "github");
        assert_eq!(config.providers[0].instance_url, "https://api.github.com");
    }

    #[test]
    fn test_migrate_pre_plan5_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let json = r#"{
            "theme": "gitlab-dark",
            "gitlab_instance_url": "https://gitlab.example.com"
        }"#;
        std::fs::write(&path, json).unwrap();

        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.providers[0].kind, "gitlab");
        assert_eq!(
            config.providers[0].instance_url,
            "https://gitlab.example.com"
        );
    }

    #[test]
    fn test_no_migration_when_providers_present() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let json = r#"{
            "theme": "gitlab-dark",
            "providers": [{"kind": "github", "instance_url": "https://api.github.com"}],
            "provider_kind": "gitlab",
            "provider_instance_url": "https://gitlab.com"
        }"#;
        std::fs::write(&path, json).unwrap();

        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.providers[0].kind, "github");
    }

    #[test]
    fn test_open_projects_persist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = AppConfig {
            open_projects: vec![
                "/home/user/repo-a".to_string(),
                "/home/user/repo-b".to_string(),
            ],
            active_project_index: Some(1),
            ..AppConfig::default()
        };
        config.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(
            loaded.open_projects,
            vec!["/home/user/repo-a", "/home/user/repo-b"]
        );
        assert_eq!(loaded.active_project_index, Some(1));
    }

    #[test]
    fn test_open_projects_default_empty() {
        let config = AppConfig::default();
        assert!(config.open_projects.is_empty());
        assert_eq!(config.active_project_index, None);
    }

    #[test]
    fn test_legacy_config_without_open_projects() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let json = r#"{"theme": "gitlab-dark"}"#;
        std::fs::write(&path, json).unwrap();

        let config = AppConfig::load(&path).unwrap();
        assert!(config.open_projects.is_empty());
        assert_eq!(config.active_project_index, None);
    }

    #[test]
    fn test_legacy_fields_not_serialized() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = AppConfig {
            providers: vec![SavedProvider {
                kind: "gitlab".to_string(),
                instance_url: "https://gitlab.com".to_string(),
            }],
            ..AppConfig::default()
        };
        config.save(&path).unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(!saved.contains("provider_kind"));
        assert!(!saved.contains("provider_instance_url"));
        assert!(!saved.contains("gitlab_instance_url"));
    }

    #[test]
    fn test_theme_auto_default_true() {
        let config = AppConfig::default();
        assert!(config.theme_auto);
        assert_eq!(config.theme, "github-dark");
    }

    #[test]
    fn test_ai_background_defaults() {
        let config = AppConfig::default();
        assert!(config.ai_worktree_root.is_none());
        assert_eq!(config.ai_background_concurrency_cap, 3);
        assert!(!config.ai_prompt_auto_accept);
    }

    #[test]
    fn test_legacy_config_gets_ai_background_defaults() {
        // Old configs that pre-date Phase 10 must still load, with the
        // AI background knobs falling back to defaults.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let json = r#"{"theme": "github-dark", "theme_auto": true}"#;
        std::fs::write(&path, json).unwrap();

        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.ai_background_concurrency_cap, 3);
        assert!(config.ai_worktree_root.is_none());
        assert!(!config.ai_prompt_auto_accept);
    }

    #[test]
    fn test_ai_background_settings_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = AppConfig {
            ai_worktree_root: Some("custom/ai-worktrees".into()),
            ai_background_concurrency_cap: 5,
            ai_prompt_auto_accept: true,
            ..AppConfig::default()
        };
        config.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(
            loaded.ai_worktree_root.as_deref(),
            Some("custom/ai-worktrees")
        );
        assert_eq!(loaded.ai_background_concurrency_cap, 5);
        assert!(loaded.ai_prompt_auto_accept);
    }

    #[test]
    fn test_auto_check_updates_default_true() {
        let config = AppConfig::default();
        assert!(config.auto_check_updates);
    }

    #[test]
    fn test_auto_check_updates_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = AppConfig {
            auto_check_updates: false,
            ..AppConfig::default()
        };
        config.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert!(!loaded.auto_check_updates);
    }

    #[test]
    fn test_legacy_config_defaults_auto_check_updates_true() {
        // Old configs (pre-auto-update) must load cleanly with the probe
        // enabled by default — users opt out, they don't opt in.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let json = r#"{"theme": "github-dark"}"#;
        std::fs::write(&path, json).unwrap();

        let config = AppConfig::load(&path).unwrap();
        assert!(config.auto_check_updates);
    }

    #[test]
    fn test_reauth_dismissal_defaults_false() {
        let config = AppConfig::default();
        assert!(!config.auto_update_reauth_notice_dismissed_macos);
        assert!(!config.auto_update_reauth_notice_dismissed_windows);
    }

    #[test]
    fn test_reauth_dismissal_persists_per_os() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = AppConfig {
            auto_update_reauth_notice_dismissed_macos: true,
            auto_update_reauth_notice_dismissed_windows: false,
            ..AppConfig::default()
        };
        config.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert!(loaded.auto_update_reauth_notice_dismissed_macos);
        assert!(!loaded.auto_update_reauth_notice_dismissed_windows);
    }

    #[test]
    fn test_legacy_config_defaults_reauth_flags_false() {
        // Existing configs (written before the reauth flags existed) must
        // still load without error — flags fall back to `false`.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let json = r#"{"theme": "github-dark"}"#;
        std::fs::write(&path, json).unwrap();

        let config = AppConfig::load(&path).unwrap();
        assert!(!config.auto_update_reauth_notice_dismissed_macos);
        assert!(!config.auto_update_reauth_notice_dismissed_windows);
    }

    #[test]
    fn test_sidebar_nav_layout_defaults_and_roundtrip() {
        // Fresh defaults should match the canonical 13-item order and have
        // no hidden items.
        let cfg = AppConfig::default();
        assert_eq!(
            cfg.sidebar_nav_order,
            vec![
                "graph",
                "changes",
                "editor",
                "branches",
                "tags",
                "stashes",
                "worktrees",
                "reflog",
                "bisect",
                "submodules",
                "ai-config",
                "ai-sessions",
                "requests",
            ]
        );
        assert!(cfg.sidebar_nav_hidden.is_empty());

        // Mutate and roundtrip through save/load.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let cfg = AppConfig {
            sidebar_nav_order: vec![
                "changes".to_string(),
                "graph".to_string(),
                "branches".to_string(),
            ],
            sidebar_nav_hidden: vec!["bisect".to_string(), "reflog".to_string()],
            ..AppConfig::default()
        };
        cfg.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(
            loaded.sidebar_nav_order,
            vec!["changes", "graph", "branches"]
        );
        assert_eq!(loaded.sidebar_nav_hidden, vec!["bisect", "reflog"]);
    }

    #[test]
    fn test_legacy_config_gets_sidebar_nav_defaults() {
        // Config files written before this feature must still load, with
        // the new fields falling back to the default order and empty hidden.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let json = r#"{"theme": "github-dark"}"#;
        std::fs::write(&path, json).unwrap();

        let cfg = AppConfig::load(&path).unwrap();
        assert_eq!(cfg.sidebar_nav_order.len(), 13);
        assert_eq!(cfg.sidebar_nav_order.first().unwrap(), "graph");
        assert!(cfg.sidebar_nav_hidden.is_empty());
    }

    #[test]
    fn editor_preferences_default_matches_struct_default() {
        // The `default_editor_preferences` helper is what `serde(default = …)`
        // calls when the field is missing from a config file; it must agree
        // with `EditorPreferences::default()` so legacy configs and fresh
        // ones converge on the same shape.
        assert_eq!(default_editor_preferences(), EditorPreferences::default());
    }

    #[test]
    fn editor_preferences_round_trip_through_appconfig_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let prefs = EditorPreferences {
            autocomplete: false,
            close_brackets: false,
            bracket_matching: true,
            highlight_active_line: false,
            highlight_selection_matches: false,
            fold_gutter: false,
            indent_on_input: false,
            line_wrapping: false,
            rectangular_selection: true,
            crosshair_cursor: true,
            tab_size: 4,
            indent_with_tabs: true,
            respect_gitignore_in_tree: true,
            large_file_warning_kb: 1024,
        };
        let cfg = AppConfig {
            editor_preferences: prefs.clone(),
            ..AppConfig::default()
        };
        cfg.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(loaded.editor_preferences, prefs);
    }

    #[test]
    fn appconfig_with_missing_editor_preferences_falls_back_to_default() {
        // An old config file that pre-dates the editor preferences must still
        // load — `serde(default = …)` should fill the field with the helper's
        // value, matching `EditorPreferences::default()`.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let json = r#"{"theme": "github-dark"}"#;
        std::fs::write(&path, json).unwrap();

        let cfg = AppConfig::load(&path).unwrap();
        assert_eq!(cfg.editor_preferences, EditorPreferences::default());
    }

    #[test]
    fn test_theme_auto_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = AppConfig {
            theme_auto: false,
            theme: "gitlab-light".to_string(),
            ..AppConfig::default()
        };
        config.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert!(!loaded.theme_auto);
        assert_eq!(loaded.theme, "gitlab-light");
    }
}
