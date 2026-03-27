//! Theme loading, parsing, and validation.
//!
//! Supports built-in themes (compiled into the binary) and user-provided TOML
//! theme files loaded from `~/.config/beardgit/themes/`.

use std::path::Path;

use serde::{Deserialize, Serialize};

// -- Built-in theme TOML sources --

const GITHUB_DARK_TOML: &str = include_str!("themes/github_dark.toml");
const GITHUB_LIGHT_TOML: &str = include_str!("themes/github_light.toml");
const GITLAB_DARK_TOML: &str = include_str!("themes/gitlab_dark.toml");
const GITLAB_LIGHT_TOML: &str = include_str!("themes/gitlab_light.toml");

/// The default theme used when the requested theme is not found.
pub const DEFAULT_THEME_ID: &str = "github-dark";

/// README content written into the user themes directory.
const THEMES_README: &str = r##"# BeardGit Custom Themes

Place `.toml` files in this directory to add custom themes.
BeardGit will pick them up automatically on next launch.

## Required Format

Every theme file must have three sections: `[meta]`, `[colors]`, and `[graph]`.

```toml
[meta]
id = "my-custom-theme"      # unique identifier (kebab-case)
name = "My Custom Theme"    # display name in the theme picker
mode = "dark"               # "dark" or "light"

[colors]
bg-primary = "#1a1b26"
bg-secondary = "#24283b"
bg-toolbar = "#1f2335"
text-primary = "#c0caf5"
text-secondary = "#565f89"
accent-blue = "#7aa2f7"
accent-green = "#9ece6a"
accent-orange = "#ff9e64"
accent-purple = "#bb9af7"
accent-red = "#f7768e"
border = "#3b4261"
selection = "#283457"

[graph]
lane-colors = [
    "#7aa2f7",
    "#9ece6a",
    "#ff9e64",
    "#bb9af7",
    "#f7768e",
    "#7dcfff",
    "#73daca",
    "#e0af68",
    "#c0caf5",
    "#ff007c",
]
background = "#1a1b26"
foreground = "#c0caf5"
text-primary = "#c0caf5"
text-secondary = "#565f89"
text-sha = "#7aa2f7"
selection = "#28345766"
head-lane-tint = "#7aa2f722"
selection-highlight = "#28345799"
dim-opacity = 0.4
node-radius = 4.0
merge-radius = 3.0
ref-branch = "#9ece6a"
ref-remote = "#7aa2f7"
ref-tag = "#ff9e64"
ref-head = "#bb9af7"
```

## Color Formats

Accepted formats:
- `#RRGGBB`   (e.g. `#58a6ff`)
- `#RRGGBBAA` (e.g. `#58a6ff33` — with alpha)
- `rgba(r, g, b, a)` (e.g. `rgba(88, 166, 255, 0.2)`)

## Overriding Built-in Themes

To override a built-in theme, use the same `id` in your `[meta]` section.
User themes always take priority over built-in themes with the same id.
"##;

// -- Error type --

/// Errors that can occur when loading or parsing themes.
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    /// A required TOML section is missing.
    #[error("missing required field: {0}")]
    MissingField(String),
    /// A color string doesn't match any accepted format.
    #[error("invalid color format for {field}: {value}")]
    InvalidColor {
        /// The TOML field name.
        field: String,
        /// The invalid value.
        value: String,
    },
    /// The `mode` field is not `"dark"` or `"light"`.
    #[error("invalid mode: expected \"dark\" or \"light\"")]
    InvalidMode,
    /// Fewer than 2 lane colors were provided.
    #[error("lane-colors must have at least 2 entries")]
    InsufficientLaneColors,
    /// TOML deserialization failed.
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
    /// Filesystem I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// -- Public types --

/// Minimal theme metadata for listing in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMeta {
    /// Unique identifier (kebab-case, e.g. `"github-dark"`).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// `"dark"` or `"light"`.
    pub mode: String,
}

/// Full theme definition as parsed from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Metadata section.
    pub meta: ThemeMetaSection,
    /// UI color tokens.
    pub colors: ThemeColors,
    /// Graph-specific rendering tokens.
    pub graph: ThemeGraph,
}

/// The `[meta]` section of a theme file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMetaSection {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// `"dark"` or `"light"`.
    pub mode: String,
}

/// The `[colors]` section — general UI color tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    #[serde(alias = "bg-primary")]
    pub bg_primary: String,
    #[serde(alias = "bg-secondary")]
    pub bg_secondary: String,
    #[serde(alias = "bg-toolbar")]
    pub bg_toolbar: String,
    #[serde(alias = "text-primary")]
    pub text_primary: String,
    #[serde(alias = "text-secondary")]
    pub text_secondary: String,
    #[serde(alias = "accent-blue")]
    pub accent_blue: String,
    #[serde(alias = "accent-green")]
    pub accent_green: String,
    #[serde(alias = "accent-orange")]
    pub accent_orange: String,
    #[serde(alias = "accent-purple")]
    pub accent_purple: String,
    #[serde(alias = "accent-red")]
    pub accent_red: String,
    pub border: String,
    pub selection: String,
}

/// The `[graph]` section — canvas/graph rendering tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeGraph {
    #[serde(alias = "lane-colors")]
    pub lane_colors: Vec<String>,
    pub background: String,
    pub foreground: String,
    #[serde(alias = "text-primary")]
    pub text_primary: String,
    #[serde(alias = "text-secondary")]
    pub text_secondary: String,
    #[serde(alias = "text-sha")]
    pub text_sha: String,
    pub selection: String,
    #[serde(alias = "head-lane-tint")]
    pub head_lane_tint: String,
    #[serde(alias = "selection-highlight")]
    pub selection_highlight: String,
    #[serde(alias = "dim-opacity")]
    pub dim_opacity: f64,
    #[serde(alias = "node-radius")]
    pub node_radius: f64,
    #[serde(alias = "merge-radius")]
    pub merge_radius: f64,
    #[serde(alias = "ref-branch")]
    pub ref_branch: String,
    #[serde(alias = "ref-remote")]
    pub ref_remote: String,
    #[serde(alias = "ref-tag")]
    pub ref_tag: String,
    #[serde(alias = "ref-head")]
    pub ref_head: String,
}

// -- Raw deserialization helper --

/// Intermediate struct for TOML deserialization with optional sections.
#[derive(Deserialize)]
struct RawTheme {
    meta: Option<ThemeMetaSection>,
    colors: Option<ThemeColors>,
    graph: Option<ThemeGraph>,
}

// -- Parsing and validation --

/// Parse and validate a TOML theme string into a [`Theme`].
pub fn parse_theme(toml_str: &str) -> Result<Theme, ThemeError> {
    let raw: RawTheme = toml::from_str(toml_str)?;

    let meta = raw
        .meta
        .ok_or_else(|| ThemeError::MissingField("meta".to_string()))?;
    let colors = raw
        .colors
        .ok_or_else(|| ThemeError::MissingField("colors".to_string()))?;
    let graph = raw
        .graph
        .ok_or_else(|| ThemeError::MissingField("graph".to_string()))?;

    // Validate mode
    if meta.mode != "dark" && meta.mode != "light" {
        return Err(ThemeError::InvalidMode);
    }

    // Validate lane colors count
    if graph.lane_colors.len() < 2 {
        return Err(ThemeError::InsufficientLaneColors);
    }

    // Validate all color fields
    validate_color("colors.bg_primary", &colors.bg_primary)?;
    validate_color("colors.bg_secondary", &colors.bg_secondary)?;
    validate_color("colors.bg_toolbar", &colors.bg_toolbar)?;
    validate_color("colors.text_primary", &colors.text_primary)?;
    validate_color("colors.text_secondary", &colors.text_secondary)?;
    validate_color("colors.accent_blue", &colors.accent_blue)?;
    validate_color("colors.accent_green", &colors.accent_green)?;
    validate_color("colors.accent_orange", &colors.accent_orange)?;
    validate_color("colors.accent_purple", &colors.accent_purple)?;
    validate_color("colors.accent_red", &colors.accent_red)?;
    validate_color("colors.border", &colors.border)?;
    validate_color("colors.selection", &colors.selection)?;

    for (i, c) in graph.lane_colors.iter().enumerate() {
        validate_color(&format!("graph.lane_colors[{i}]"), c)?;
    }
    validate_color("graph.background", &graph.background)?;
    validate_color("graph.foreground", &graph.foreground)?;
    validate_color("graph.text_primary", &graph.text_primary)?;
    validate_color("graph.text_secondary", &graph.text_secondary)?;
    validate_color("graph.text_sha", &graph.text_sha)?;
    validate_color("graph.selection", &graph.selection)?;
    validate_color("graph.head_lane_tint", &graph.head_lane_tint)?;
    validate_color("graph.selection_highlight", &graph.selection_highlight)?;
    validate_color("graph.ref_branch", &graph.ref_branch)?;
    validate_color("graph.ref_remote", &graph.ref_remote)?;
    validate_color("graph.ref_tag", &graph.ref_tag)?;
    validate_color("graph.ref_head", &graph.ref_head)?;

    Ok(Theme {
        meta,
        colors,
        graph,
    })
}

/// Validate that a color string is `#RRGGBB`, `#RRGGBBAA`, or `rgba(...)`.
fn validate_color(field: &str, value: &str) -> Result<(), ThemeError> {
    let valid = if let Some(hex_part) = value.strip_prefix('#') {
        (hex_part.len() == 6 || hex_part.len() == 8)
            && hex_part.chars().all(|c| c.is_ascii_hexdigit())
    } else {
        value.starts_with("rgba(") && value.ends_with(')')
    };

    if valid {
        Ok(())
    } else {
        Err(ThemeError::InvalidColor {
            field: field.to_string(),
            value: value.to_string(),
        })
    }
}

impl Theme {
    /// Extract lightweight metadata for UI listing.
    pub fn to_meta(&self) -> ThemeMeta {
        ThemeMeta {
            id: self.meta.id.clone(),
            name: self.meta.name.clone(),
            mode: self.meta.mode.clone(),
        }
    }
}

// -- Loading functions --

/// Parse and return all built-in themes, skipping any that fail to parse.
pub fn load_builtin_themes() -> Vec<Theme> {
    [
        GITHUB_DARK_TOML,
        GITHUB_LIGHT_TOML,
        GITLAB_DARK_TOML,
        GITLAB_LIGHT_TOML,
    ]
    .iter()
    .filter_map(|src| parse_theme(src).ok())
    .collect()
}

/// Load user themes from `.toml` files in the given directory, skipping invalid files.
pub fn load_user_themes(themes_dir: &Path) -> Vec<Theme> {
    let Ok(entries) = std::fs::read_dir(themes_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "toml")
        })
        .filter_map(|entry| {
            let content = std::fs::read_to_string(entry.path()).ok()?;
            parse_theme(&content).ok()
        })
        .collect()
}

/// List metadata for all available themes (built-in + user).
///
/// User themes override built-in themes with the same `id`. Results sorted by name.
pub fn list_all_themes(themes_dir: &Path) -> Vec<ThemeMeta> {
    let builtins = load_builtin_themes();
    let user = load_user_themes(themes_dir);

    let mut map = std::collections::HashMap::new();
    for theme in builtins {
        map.insert(theme.meta.id.clone(), theme.to_meta());
    }
    // User themes override built-in by id
    for theme in user {
        map.insert(theme.meta.id.clone(), theme.to_meta());
    }

    let mut result: Vec<ThemeMeta> = map.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

/// Resolve a theme by id, checking user themes first, then built-in.
///
/// Falls back to the default theme (`github-dark`) if not found.
pub fn resolve_theme(id: &str, themes_dir: &Path) -> Theme {
    // Check user themes first
    for theme in load_user_themes(themes_dir) {
        if theme.meta.id == id {
            return theme;
        }
    }
    // Then built-in
    for theme in load_builtin_themes() {
        if theme.meta.id == id {
            return theme;
        }
    }
    // Fallback
    parse_theme(GITHUB_DARK_TOML).expect("built-in github-dark theme must parse")
}

/// Create the themes directory and a README.md if they don't already exist.
pub fn ensure_themes_dir(themes_dir: &Path) -> Result<(), ThemeError> {
    std::fs::create_dir_all(themes_dir)?;
    let readme = themes_dir.join("README.md");
    if !readme.exists() {
        std::fs::write(&readme, THEMES_README)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid TOML for testing.
    const MINIMAL_THEME: &str = r##"
[meta]
id = "test-theme"
name = "Test Theme"
mode = "dark"

[colors]
bg-primary = "#111111"
bg-secondary = "#222222"
bg-toolbar = "#333333"
text-primary = "#eeeeee"
text-secondary = "#999999"
accent-blue = "#0000ff"
accent-green = "#00ff00"
accent-orange = "#ff8800"
accent-purple = "#8800ff"
accent-red = "#ff0000"
border = "#444444"
selection = "#0000ff33"

[graph]
lane-colors = ["#0000ff", "#00ff00", "#ff0000"]
background = "#111111"
foreground = "#eeeeee"
text-primary = "#eeeeee"
text-secondary = "#999999"
text-sha = "#0000ff"
selection = "#0000ff44"
head-lane-tint = "#0000ff22"
selection-highlight = "#0000ff66"
dim-opacity = 0.4
node-radius = 4.0
merge-radius = 3.0
ref-branch = "#00ff00"
ref-remote = "#0000ff"
ref-tag = "#ff8800"
ref-head = "#8800ff"
"##;

    #[test]
    fn test_parse_valid_theme() {
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        assert_eq!(theme.meta.id, "test-theme");
        assert_eq!(theme.meta.name, "Test Theme");
        assert_eq!(theme.meta.mode, "dark");
        assert_eq!(theme.colors.bg_primary, "#111111");
        assert_eq!(theme.graph.lane_colors.len(), 3);
    }

    #[test]
    fn test_parse_missing_meta() {
        let toml = r##"
[colors]
bg-primary = "#111111"
bg-secondary = "#222222"
bg-toolbar = "#333333"
text-primary = "#eeeeee"
text-secondary = "#999999"
accent-blue = "#0000ff"
accent-green = "#00ff00"
accent-orange = "#ff8800"
accent-purple = "#8800ff"
accent-red = "#ff0000"
border = "#444444"
selection = "#0000ff33"

[graph]
lane-colors = ["#0000ff", "#00ff00"]
background = "#111111"
foreground = "#eeeeee"
text-primary = "#eeeeee"
text-secondary = "#999999"
text-sha = "#0000ff"
selection = "#0000ff44"
head-lane-tint = "#0000ff22"
selection-highlight = "#0000ff66"
dim-opacity = 0.4
node-radius = 4.0
merge-radius = 3.0
ref-branch = "#00ff00"
ref-remote = "#0000ff"
ref-tag = "#ff8800"
ref-head = "#8800ff"
"##;
        let err = parse_theme(toml).unwrap_err();
        assert!(err.to_string().contains("meta"));
    }

    #[test]
    fn test_parse_missing_colors() {
        let toml = r##"
[meta]
id = "x"
name = "X"
mode = "dark"

[graph]
lane-colors = ["#0000ff", "#00ff00"]
background = "#111111"
foreground = "#eeeeee"
text-primary = "#eeeeee"
text-secondary = "#999999"
text-sha = "#0000ff"
selection = "#0000ff44"
head-lane-tint = "#0000ff22"
selection-highlight = "#0000ff66"
dim-opacity = 0.4
node-radius = 4.0
merge-radius = 3.0
ref-branch = "#00ff00"
ref-remote = "#0000ff"
ref-tag = "#ff8800"
ref-head = "#8800ff"
"##;
        let err = parse_theme(toml).unwrap_err();
        assert!(err.to_string().contains("colors"));
    }

    #[test]
    fn test_parse_invalid_mode() {
        let toml = MINIMAL_THEME.replace("mode = \"dark\"", "mode = \"neon\"");
        let err = parse_theme(&toml).unwrap_err();
        assert!(err.to_string().contains("mode"));
    }

    #[test]
    fn test_parse_insufficient_lane_colors() {
        let toml = MINIMAL_THEME.replace(
            r##"lane-colors = ["#0000ff", "#00ff00", "#ff0000"]"##,
            r##"lane-colors = ["#0000ff"]"##,
        );
        let err = parse_theme(&toml).unwrap_err();
        assert!(err.to_string().contains("lane-colors"));
    }

    #[test]
    fn test_parse_invalid_color_format() {
        let toml = MINIMAL_THEME.replace(r##"bg-primary = "#111111""##, r##"bg-primary = "nope""##);
        let err = parse_theme(&toml).unwrap_err();
        assert!(err.to_string().contains("invalid color"));
    }

    #[test]
    fn test_parse_rgba_color_accepted() {
        let toml = MINIMAL_THEME.replace(
            r##"selection = "#0000ff33""##,
            r##"selection = "rgba(0, 0, 255, 0.2)""##,
        );
        // This replaces the first occurrence (colors.selection)
        let theme = parse_theme(&toml).unwrap();
        assert_eq!(theme.colors.selection, "rgba(0, 0, 255, 0.2)");
    }

    #[test]
    fn test_to_meta() {
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        let meta = theme.to_meta();
        assert_eq!(meta.id, "test-theme");
        assert_eq!(meta.name, "Test Theme");
        assert_eq!(meta.mode, "dark");
    }

    #[test]
    fn test_parse_invalid_toml() {
        let err = parse_theme("this is not { valid toml !!!").unwrap_err();
        assert!(matches!(err, ThemeError::Parse(_)));
    }

    #[test]
    fn test_load_builtin_themes() {
        let themes = load_builtin_themes();
        assert_eq!(themes.len(), 4);
    }

    #[test]
    fn test_builtin_themes_have_correct_modes() {
        let themes = load_builtin_themes();
        let dark_count = themes.iter().filter(|t| t.meta.mode == "dark").count();
        let light_count = themes.iter().filter(|t| t.meta.mode == "light").count();
        assert_eq!(dark_count, 2);
        assert_eq!(light_count, 2);
    }

    #[test]
    fn test_resolve_builtin_theme() {
        let dir = tempfile::tempdir().unwrap();
        let theme = resolve_theme("gitlab-dark", dir.path());
        assert_eq!(theme.meta.id, "gitlab-dark");
    }

    #[test]
    fn test_resolve_unknown_falls_back() {
        let dir = tempfile::tempdir().unwrap();
        let theme = resolve_theme("nonexistent-theme", dir.path());
        assert_eq!(theme.meta.id, DEFAULT_THEME_ID);
    }

    #[test]
    fn test_user_theme_overrides_builtin() {
        let dir = tempfile::tempdir().unwrap();
        // Write a user theme that overrides github-dark
        let custom = MINIMAL_THEME
            .replace("id = \"test-theme\"", "id = \"github-dark\"")
            .replace("name = \"Test Theme\"", "name = \"My Custom GitHub Dark\"");
        std::fs::write(dir.path().join("custom.toml"), &custom).unwrap();

        let theme = resolve_theme("github-dark", dir.path());
        assert_eq!(theme.meta.name, "My Custom GitHub Dark");
    }

    #[test]
    fn test_list_all_themes() {
        let dir = tempfile::tempdir().unwrap();
        let themes = list_all_themes(dir.path());
        assert!(themes.len() >= 4);
    }

    #[test]
    fn test_ensure_themes_dir_creates_readme() {
        let dir = tempfile::tempdir().unwrap();
        let themes_dir = dir.path().join("themes");
        ensure_themes_dir(&themes_dir).unwrap();
        assert!(themes_dir.join("README.md").exists());
    }

    #[test]
    fn test_load_user_themes_skips_invalid() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.toml"), "not valid theme").unwrap();
        std::fs::write(dir.path().join("good.toml"), MINIMAL_THEME).unwrap();
        let themes = load_user_themes(dir.path());
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].meta.id, "test-theme");
    }
}
