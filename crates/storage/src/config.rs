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

fn default_locale() -> String {
    "en-US".to_string()
}

fn default_ui_scale() -> u32 {
    100
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

    /// Preferred AI provider kind (e.g. `"claude_code"`, `"codex"`, `"open_code"`).
    /// `None` means "use first detected".
    #[serde(default)]
    pub preferred_ai_provider: Option<String>,

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
            preferred_ai_provider: None,
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
    fn test_theme_auto_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let mut config = AppConfig::default();
        config.theme_auto = false;
        config.theme = "gitlab-light".to_string();
        config.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert!(!loaded.theme_auto);
        assert_eq!(loaded.theme, "gitlab-light");
    }
}
