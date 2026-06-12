//! Theme loading, parsing, and validation.
//!
//! Supports built-in themes (compiled into the binary) and user-provided TOML
//! theme files loaded from `~/.config/beardgit/themes/`.
//!
//! Only `[meta]` and `[colors]` are required. The `[graph]` and `[editor]`
//! sections are derived automatically from the base palette when omitted.
//! Users can still override any derived value by including the section.

use std::path::Path;

use serde::{Deserialize, Serialize};

// -- Built-in theme TOML sources --

const BEARDGIT_DARK_TOML: &str = include_str!("themes/beardgit_dark.toml");
const BEARDGIT_LIGHT_TOML: &str = include_str!("themes/beardgit_light.toml");
const FJORD_DARK_TOML: &str = include_str!("themes/fjord_dark.toml");
const FJORD_LIGHT_TOML: &str = include_str!("themes/fjord_light.toml");
const NEBULA_DARK_TOML: &str = include_str!("themes/nebula_dark.toml");
const NEBULA_LIGHT_TOML: &str = include_str!("themes/nebula_light.toml");
const GITHUB_DARK_TOML: &str = include_str!("themes/github_dark.toml");
const GITHUB_LIGHT_TOML: &str = include_str!("themes/github_light.toml");
const GITLAB_DARK_TOML: &str = include_str!("themes/gitlab_dark.toml");
const GITLAB_LIGHT_TOML: &str = include_str!("themes/gitlab_light.toml");
const DRACULA_TOML: &str = include_str!("themes/dracula.toml");
const ONE_DARK_TOML: &str = include_str!("themes/one_dark.toml");
const CATPPUCCIN_MOCHA_TOML: &str = include_str!("themes/catppuccin_mocha.toml");
const CATPPUCCIN_LATTE_TOML: &str = include_str!("themes/catppuccin_latte.toml");
const NORD_TOML: &str = include_str!("themes/nord.toml");
const TOKYO_NIGHT_TOML: &str = include_str!("themes/tokyo_night.toml");
const SOLARIZED_DARK_TOML: &str = include_str!("themes/solarized_dark.toml");
const SOLARIZED_LIGHT_TOML: &str = include_str!("themes/solarized_light.toml");
const GRUVBOX_DARK_TOML: &str = include_str!("themes/gruvbox_dark.toml");
const MONOKAI_PRO_TOML: &str = include_str!("themes/monokai_pro.toml");

/// The default theme used when the requested theme is not found.
pub const DEFAULT_THEME_ID: &str = "beardgit-dark";
/// Default dark theme for fallback when no complementary pair exists.
pub const DEFAULT_DARK_THEME_ID: &str = "beardgit-dark";
/// Default light theme for fallback when no complementary pair exists.
pub const DEFAULT_LIGHT_THEME_ID: &str = "beardgit-light";

/// README content written into the user themes directory.
const THEMES_README: &str = r##"# BeardGit Custom Themes

Place `.toml` files in this directory to add custom themes.
BeardGit will pick them up automatically on next launch.

## Creating a Theme

Only `[meta]` and `[colors]` are required — everything else is derived:

```toml
[meta]
id = "my-custom-theme"      # unique identifier (kebab-case)
name = "My Custom Theme"    # display name in the theme picker
mode = "dark"               # "dark" or "light"
complementary = "my-light"  # optional: paired theme for OS auto-switch

[colors]
background = "#1a1b26"       # main background
foreground = "#c0caf5"       # main text
black = "#32344a"
red = "#f7768e"
green = "#9ece6a"
yellow = "#e0af68"
blue = "#7aa2f7"
magenta = "#bb9af7"
cyan = "#449dab"
white = "#787c99"
bright-black = "#444b6a"
bright-red = "#ff7a93"
bright-green = "#b9f27c"
bright-yellow = "#ff9e64"
bright-blue = "#7da6ff"
bright-magenta = "#bb9af7"
bright-cyan = "#0db9d7"
bright-white = "#acb0d0"
```

That's it — 18 base colors (background + foreground + 16 ANSI). All semantic
UI colors are derived automatically:

- **Graph:** lane colors = 5 accents + lighter variants; refs = green (branch),
  blue (remote), yellow (tag), magenta (HEAD); selection/tints from blue
- **Editor:** syntax highlighting derived from ANSI colors; diff backgrounds
  computed for dark/light mode; cursor and selection from blue
- **All other UI elements** are styled via CSS custom properties from derived colors

## Optional Overrides

To tweak specific derived values, add a partial `[graph]` or `[editor]` section.
Only the fields you include are overridden — everything else keeps the derived value.

```toml
[graph]
lane-colors = ["#7aa2f7", "#9ece6a", "#ff9e64"]  # custom lane palette
node-radius = 5.0                                   # bigger commit dots
dim-opacity = 0.3                                    # more transparent dimmed lanes

[editor]
added-bg = "#1b3829"          # custom diff added background
removed-bg = "#3c1e22"        # custom diff removed background
syntax-keyword = "#ff7b72"    # override keyword color
syntax-string = "#a5d6ff"     # override string color
```

### Graph fields
- `lane-colors` — array of hex colors for commit graph lanes (min 2)
- `background`, `foreground` — graph canvas colors
- `text-primary`, `text-secondary`, `text-sha` — graph text colors
- `selection`, `head-lane-tint`, `selection-highlight` — selection tints
- `dim-opacity` — opacity for dimmed lanes (0.0–1.0)
- `node-radius`, `merge-radius` — commit dot sizes
- `ref-branch`, `ref-remote`, `ref-tag`, `ref-head` — ref badge colors

### Editor fields
- `background`, `foreground` — editor background/text
- `cursor`, `selection`, `line-highlight` — cursor and selection
- `gutter-bg`, `gutter-fg` — line number gutter
- `added-bg`, `removed-bg`, `added-text`, `removed-text` — diff colors
- `syntax-keyword`, `syntax-string`, `syntax-comment`, `syntax-function`,
  `syntax-type`, `syntax-number`, `syntax-operator`, `syntax-property` — syntax tokens

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
    /// ID of the paired theme for OS dark/light auto-switching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complementary: Option<String>,
}

/// Full theme definition as parsed from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Metadata section.
    pub meta: ThemeMetaSection,
    /// Base 18-color palette from TOML.
    pub colors: ThemeColors,
    /// Semantic UI colors derived from base palette.
    pub derived: DerivedColors,
    /// Graph-specific rendering tokens.
    pub graph: ThemeGraph,
    /// CodeMirror 6 editor color tokens.
    pub editor: Option<ThemeEditor>,
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
    /// ID of the paired theme for OS dark/light auto-switching.
    #[serde(default)]
    pub complementary: Option<String>,
}

/// The `[colors]` section — 18 base colors (background + foreground + 16 ANSI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub background: String,
    pub foreground: String,
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    #[serde(alias = "bright-black")]
    pub bright_black: String,
    #[serde(alias = "bright-red")]
    pub bright_red: String,
    #[serde(alias = "bright-green")]
    pub bright_green: String,
    #[serde(alias = "bright-yellow")]
    pub bright_yellow: String,
    #[serde(alias = "bright-blue")]
    pub bright_blue: String,
    #[serde(alias = "bright-magenta")]
    pub bright_magenta: String,
    #[serde(alias = "bright-cyan")]
    pub bright_cyan: String,
    #[serde(alias = "bright-white")]
    pub bright_white: String,
}

/// Semantic UI colors derived from the 18 base colors at theme load time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedColors {
    pub bg_primary: String,
    pub bg_secondary: String,
    pub bg_toolbar: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub accent_blue: String,
    pub accent_green: String,
    pub accent_orange: String,
    pub accent_purple: String,
    pub accent_red: String,
    /// Per-theme signature accent for primary actions (selected button,
    /// active tab, focus ring, spinner). Each TOML chooses which of its
    /// ANSI colors plays this role via `[accents]`; themes without an
    /// `[accents]` section fall back to `blue`, matching the legacy
    /// behaviour where every theme used `--accent-blue` for primary.
    pub accent_primary: String,
    pub accent_secondary: String,
    pub accent_tertiary: String,
    pub border: String,
    pub selection: String,
}

/// Optional `[accents]` section: maps the three semantic accent slots
/// (`primary`, `secondary`, `tertiary`) to one of the theme's ANSI
/// color names. Lets each theme assert its visual identity — Dracula
/// pushes `magenta` as primary, Gruvbox pushes `yellow`, Nord pushes
/// `cyan`, etc.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeAccents {
    /// ANSI color name for the primary accent. Recognised values: any
    /// of the 16 standard ANSI names (`black`, `red`, …, `bright_red`,
    /// …) or a literal `#RRGGBB` hex string. Defaults to `"blue"`.
    pub primary: Option<String>,
    /// ANSI color name (or hex) for the secondary accent.
    /// Defaults to `"magenta"`.
    pub secondary: Option<String>,
    /// ANSI color name (or hex) for the tertiary accent.
    /// Defaults to `"green"`.
    pub tertiary: Option<String>,
}

/// Editor color tokens for CodeMirror 6 integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeEditor {
    /// Editor background color.
    pub background: String,
    /// Default text foreground color.
    pub foreground: String,
    /// Cursor/caret color.
    pub cursor: String,
    /// Text selection background color.
    pub selection: String,
    /// Active line highlight background.
    #[serde(rename = "line-highlight")]
    pub line_highlight: String,
    /// Gutter background color.
    #[serde(rename = "gutter-bg")]
    pub gutter_bg: String,
    /// Gutter foreground (line numbers) color.
    #[serde(rename = "gutter-fg")]
    pub gutter_fg: String,
    /// Added line background in diff view.
    #[serde(rename = "added-bg")]
    pub added_bg: String,
    /// Removed line background in diff view.
    #[serde(rename = "removed-bg")]
    pub removed_bg: String,
    /// Added line text color in diff view.
    #[serde(rename = "added-text")]
    pub added_text: String,
    /// Removed line text color in diff view.
    #[serde(rename = "removed-text")]
    pub removed_text: String,
    /// Syntax: keyword color.
    #[serde(default, rename = "syntax-keyword")]
    pub syntax_keyword: Option<String>,
    /// Syntax: string literal color.
    #[serde(default, rename = "syntax-string")]
    pub syntax_string: Option<String>,
    /// Syntax: comment color.
    #[serde(default, rename = "syntax-comment")]
    pub syntax_comment: Option<String>,
    /// Syntax: function/method name color.
    #[serde(default, rename = "syntax-function")]
    pub syntax_function: Option<String>,
    /// Syntax: type/class name color.
    #[serde(default, rename = "syntax-type")]
    pub syntax_type: Option<String>,
    /// Syntax: number literal color.
    #[serde(default, rename = "syntax-number")]
    pub syntax_number: Option<String>,
    /// Syntax: operator color.
    #[serde(default, rename = "syntax-operator")]
    pub syntax_operator: Option<String>,
    /// Syntax: property/attribute color.
    #[serde(default, rename = "syntax-property")]
    pub syntax_property: Option<String>,
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

// -- Derivation from base palette --

/// Append a 2-digit hex alpha to a `#RRGGBB` color. If the color already has
/// alpha or isn't hex, return it unchanged.
fn with_alpha(hex: &str, alpha: &str) -> String {
    if hex.starts_with('#') && hex.len() == 7 {
        format!("{hex}{alpha}")
    } else {
        hex.to_string()
    }
}

/// Strip alpha from a `#RRGGBBAA` color, returning `#RRGGBB`.
fn strip_alpha(hex: &str) -> String {
    if hex.starts_with('#') && hex.len() == 9 {
        hex[..7].to_string()
    } else {
        hex.to_string()
    }
}

/// Lighten a `#RRGGBB` color by blending toward white. `amount` is 0.0–1.0.
fn lighten_hex(hex: &str, amount: f64) -> String {
    if !hex.starts_with('#') || hex.len() < 7 {
        return hex.to_string();
    }
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(128);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(128);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(128);
    let lr = r as f64 + (255.0 - r as f64) * amount;
    let lg = g as f64 + (255.0 - g as f64) * amount;
    let lb = b as f64 + (255.0 - b as f64) * amount;
    format!("#{:02x}{:02x}{:02x}", lr as u8, lg as u8, lb as u8)
}

/// Darken a `#RRGGBB` color by blending toward black. `amount` is 0.0–1.0.
fn darken_hex(hex: &str, amount: f64) -> String {
    if !hex.starts_with('#') || hex.len() < 7 {
        return hex.to_string();
    }
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(128);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(128);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(128);
    let dr = r as f64 * (1.0 - amount);
    let dg = g as f64 * (1.0 - amount);
    let db = b as f64 * (1.0 - amount);
    format!("#{:02x}{:02x}{:02x}", dr as u8, dg as u8, db as u8)
}

/// Derive semantic UI colors from the 18 base colors.
fn derive_semantic_colors(colors: &ThemeColors, is_dark: bool) -> DerivedColors {
    let bg_secondary = if is_dark {
        lighten_hex(&colors.background, 0.05)
    } else {
        darken_hex(&colors.background, 0.03)
    };

    let bg_toolbar = if is_dark {
        lighten_hex(&colors.background, 0.08)
    } else {
        darken_hex(&colors.background, 0.05)
    };

    DerivedColors {
        bg_primary: colors.background.clone(),
        bg_secondary,
        bg_toolbar,
        text_primary: colors.foreground.clone(),
        text_secondary: colors.bright_black.clone(),
        accent_blue: colors.blue.clone(),
        accent_green: colors.green.clone(),
        accent_orange: colors.yellow.clone(),
        accent_purple: colors.magenta.clone(),
        accent_red: colors.red.clone(),
        accent_primary: colors.blue.clone(),
        accent_secondary: colors.magenta.clone(),
        accent_tertiary: colors.green.clone(),
        border: with_alpha(&colors.bright_black, "80"),
        selection: with_alpha(&colors.blue, "33"),
    }
}

/// Apply the per-theme `[accents]` overrides on top of the legacy
/// blue/magenta/green defaults already in `derived`.
fn apply_accent_overrides(
    derived: &mut DerivedColors,
    colors: &ThemeColors,
    accents: &ThemeAccents,
) {
    derived.accent_primary =
        resolve_accent(colors, accents.primary.as_deref(), &derived.accent_primary);
    derived.accent_secondary = resolve_accent(
        colors,
        accents.secondary.as_deref(),
        &derived.accent_secondary,
    );
    derived.accent_tertiary = resolve_accent(
        colors,
        accents.tertiary.as_deref(),
        &derived.accent_tertiary,
    );
}

/// Shift the hue of a `#RRGGBB` color by `degrees` (-180..180).
/// Uses a simple RGB→HSL→RGB conversion.
fn shift_hue_hex(hex: &str, degrees: i32) -> String {
    if !hex.starts_with('#') || hex.len() < 7 {
        return hex.to_string();
    }
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(128) as f64 / 255.0;
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(128) as f64 / 255.0;
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(128) as f64 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < 1e-6 {
        return hex.to_string(); // achromatic
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < 1e-6 {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if (max - g).abs() < 1e-6 {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };

    let new_h = (h + degrees as f64 / 360.0).rem_euclid(1.0);

    // HSL → RGB
    let hue_to_rgb = |p: f64, q: f64, mut t: f64| -> f64 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 1.0 / 2.0 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    };

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let nr = (hue_to_rgb(p, q, new_h + 1.0 / 3.0) * 255.0) as u8;
    let ng = (hue_to_rgb(p, q, new_h) * 255.0) as u8;
    let nb = (hue_to_rgb(p, q, new_h - 1.0 / 3.0) * 255.0) as u8;

    format!("#{:02x}{:02x}{:02x}", nr, ng, nb)
}

/// Derive the full `[graph]` section from base colors and derived semantics.
fn derive_graph(_colors: &ThemeColors, derived: &DerivedColors) -> ThemeGraph {
    // Build 10 lane colors: 5 accents + 5 shifted variants (lighter for dark, darker for light)
    let lane_colors = vec![
        derived.accent_blue.clone(),
        derived.accent_green.clone(),
        derived.accent_orange.clone(),
        derived.accent_purple.clone(),
        derived.accent_red.clone(),
        lighten_hex(&derived.accent_blue, 0.25),
        lighten_hex(&derived.accent_green, 0.25),
        shift_hue_hex(&derived.accent_orange, -20), // orange → golden
        lighten_hex(&derived.accent_purple, 0.25),
        lighten_hex(&derived.accent_red, 0.2),
    ];

    // Use the base selection color (strip alpha) for graph selection tints
    let sel_base = strip_alpha(&derived.selection);

    ThemeGraph {
        lane_colors,
        background: derived.bg_primary.clone(),
        foreground: derived.text_primary.clone(),
        text_primary: derived.text_primary.clone(),
        text_secondary: derived.text_secondary.clone(),
        text_sha: derived.accent_blue.clone(),
        selection: with_alpha(&sel_base, "44"),
        // Follow the theme's signature accent, not blue — a copper or
        // violet theme would otherwise paint a blue wash behind the
        // HEAD lane that reads as a rendering glitch.
        head_lane_tint: with_alpha(&derived.accent_primary, "22"),
        selection_highlight: with_alpha(&sel_base, "66"),
        dim_opacity: 0.4,
        node_radius: 4.0,
        merge_radius: 3.0,
        ref_branch: derived.accent_green.clone(),
        ref_remote: derived.accent_blue.clone(),
        ref_tag: derived.accent_orange.clone(),
        ref_head: derived.accent_purple.clone(),
    }
}

/// Derive the full `[editor]` section from base colors, derived semantics, and mode.
fn derive_editor(_colors: &ThemeColors, derived: &DerivedColors, is_dark: bool) -> ThemeEditor {
    let (added_bg, removed_bg) = if is_dark {
        ("#1b3829".to_string(), "#3c1e22".to_string())
    } else {
        ("#d4f8db".to_string(), "#fdd8d8".to_string())
    };

    // Derive syntax colors from the theme's own accent palette.
    // keyword/operator → red, string → blue (lightened for dark), function → purple,
    // type/number → blue, property → green, comment → text-secondary.
    let kw = &derived.accent_red;
    let str_c = if is_dark {
        lighten_hex(&derived.accent_blue, 0.35)
    } else {
        shift_hue_hex(&derived.accent_blue, 15)
    };
    let func = &derived.accent_purple;
    let typ = &derived.accent_blue;
    let prop = &derived.accent_green;

    let sel_base = strip_alpha(&derived.selection);

    ThemeEditor {
        background: derived.bg_primary.clone(),
        foreground: derived.text_primary.clone(),
        cursor: derived.accent_blue.clone(),
        selection: with_alpha(&sel_base, "44"),
        line_highlight: with_alpha(&derived.bg_secondary, "66"),
        gutter_bg: derived.bg_primary.clone(),
        gutter_fg: derived.text_secondary.clone(),
        added_bg,
        removed_bg,
        added_text: derived.accent_green.clone(),
        removed_text: derived.accent_red.clone(),
        syntax_keyword: Some(kw.clone()),
        syntax_string: Some(str_c),
        syntax_comment: None, // None → frontend uses text-secondary
        syntax_function: Some(func.clone()),
        syntax_type: Some(typ.clone()),
        syntax_number: Some(typ.clone()), // numbers same color as types
        syntax_operator: Some(kw.clone()), // operators same color as keywords
        syntax_property: Some(prop.clone()),
    }
}

// -- Raw deserialization helper (all sections optional except meta+colors) --

/// Partial editor for TOML override merging. Every field is optional.
#[derive(Debug, Deserialize)]
struct RawEditorOverride {
    background: Option<String>,
    foreground: Option<String>,
    cursor: Option<String>,
    selection: Option<String>,
    #[serde(rename = "line-highlight")]
    line_highlight: Option<String>,
    #[serde(rename = "gutter-bg")]
    gutter_bg: Option<String>,
    #[serde(rename = "gutter-fg")]
    gutter_fg: Option<String>,
    #[serde(rename = "added-bg")]
    added_bg: Option<String>,
    #[serde(rename = "removed-bg")]
    removed_bg: Option<String>,
    #[serde(rename = "added-text")]
    added_text: Option<String>,
    #[serde(rename = "removed-text")]
    removed_text: Option<String>,
    #[serde(rename = "syntax-keyword")]
    syntax_keyword: Option<String>,
    #[serde(rename = "syntax-string")]
    syntax_string: Option<String>,
    #[serde(rename = "syntax-comment")]
    syntax_comment: Option<String>,
    #[serde(rename = "syntax-function")]
    syntax_function: Option<String>,
    #[serde(rename = "syntax-type")]
    syntax_type: Option<String>,
    #[serde(rename = "syntax-number")]
    syntax_number: Option<String>,
    #[serde(rename = "syntax-operator")]
    syntax_operator: Option<String>,
    #[serde(rename = "syntax-property")]
    syntax_property: Option<String>,
}

/// Partial graph for TOML override merging. Every field is optional.
#[derive(Debug, Deserialize)]
struct RawGraphOverride {
    #[serde(alias = "lane-colors")]
    lane_colors: Option<Vec<String>>,
    background: Option<String>,
    foreground: Option<String>,
    #[serde(alias = "text-primary")]
    text_primary: Option<String>,
    #[serde(alias = "text-secondary")]
    text_secondary: Option<String>,
    #[serde(alias = "text-sha")]
    text_sha: Option<String>,
    selection: Option<String>,
    #[serde(alias = "head-lane-tint")]
    head_lane_tint: Option<String>,
    #[serde(alias = "selection-highlight")]
    selection_highlight: Option<String>,
    #[serde(alias = "dim-opacity")]
    dim_opacity: Option<f64>,
    #[serde(alias = "node-radius")]
    node_radius: Option<f64>,
    #[serde(alias = "merge-radius")]
    merge_radius: Option<f64>,
    #[serde(alias = "ref-branch")]
    ref_branch: Option<String>,
    #[serde(alias = "ref-remote")]
    ref_remote: Option<String>,
    #[serde(alias = "ref-tag")]
    ref_tag: Option<String>,
    #[serde(alias = "ref-head")]
    ref_head: Option<String>,
}

/// Intermediate struct for TOML deserialization with optional sections.
#[derive(Deserialize)]
struct RawTheme {
    meta: Option<ThemeMetaSection>,
    colors: Option<ThemeColors>,
    graph: Option<RawGraphOverride>,
    editor: Option<RawEditorOverride>,
    accents: Option<ThemeAccents>,
}

/// Resolve an `accent` slot to a concrete `#RRGGBB` value. Accepts any
/// of the 16 ANSI color names (`"red"`, `"bright_blue"`, …) or a
/// literal hex string. Falls back to `default_color` if the slot is
/// `None` or names an unknown identifier — never panics on bad input.
fn resolve_accent(colors: &ThemeColors, slot: Option<&str>, default: &str) -> String {
    let Some(name) = slot else {
        return default.to_string();
    };
    if name.starts_with('#') {
        return name.to_string();
    }
    match name {
        "black" => colors.black.clone(),
        "red" => colors.red.clone(),
        "green" => colors.green.clone(),
        "yellow" => colors.yellow.clone(),
        "blue" => colors.blue.clone(),
        "magenta" => colors.magenta.clone(),
        "cyan" => colors.cyan.clone(),
        "white" => colors.white.clone(),
        "bright_black" => colors.bright_black.clone(),
        "bright_red" => colors.bright_red.clone(),
        "bright_green" => colors.bright_green.clone(),
        "bright_yellow" => colors.bright_yellow.clone(),
        "bright_blue" => colors.bright_blue.clone(),
        "bright_magenta" => colors.bright_magenta.clone(),
        "bright_cyan" => colors.bright_cyan.clone(),
        "bright_white" => colors.bright_white.clone(),
        _ => default.to_string(),
    }
}

/// Apply partial overrides from a `RawGraphOverride` onto a derived `ThemeGraph`.
fn merge_graph_overrides(base: &mut ThemeGraph, overrides: RawGraphOverride) {
    if let Some(v) = overrides.lane_colors {
        base.lane_colors = v;
    }
    if let Some(v) = overrides.background {
        base.background = v;
    }
    if let Some(v) = overrides.foreground {
        base.foreground = v;
    }
    if let Some(v) = overrides.text_primary {
        base.text_primary = v;
    }
    if let Some(v) = overrides.text_secondary {
        base.text_secondary = v;
    }
    if let Some(v) = overrides.text_sha {
        base.text_sha = v;
    }
    if let Some(v) = overrides.selection {
        base.selection = v;
    }
    if let Some(v) = overrides.head_lane_tint {
        base.head_lane_tint = v;
    }
    if let Some(v) = overrides.selection_highlight {
        base.selection_highlight = v;
    }
    if let Some(v) = overrides.dim_opacity {
        base.dim_opacity = v;
    }
    if let Some(v) = overrides.node_radius {
        base.node_radius = v;
    }
    if let Some(v) = overrides.merge_radius {
        base.merge_radius = v;
    }
    if let Some(v) = overrides.ref_branch {
        base.ref_branch = v;
    }
    if let Some(v) = overrides.ref_remote {
        base.ref_remote = v;
    }
    if let Some(v) = overrides.ref_tag {
        base.ref_tag = v;
    }
    if let Some(v) = overrides.ref_head {
        base.ref_head = v;
    }
}

/// Apply partial overrides from a `RawEditorOverride` onto a derived `ThemeEditor`.
fn merge_editor_overrides(base: &mut ThemeEditor, overrides: RawEditorOverride) {
    if let Some(v) = overrides.background {
        base.background = v;
    }
    if let Some(v) = overrides.foreground {
        base.foreground = v;
    }
    if let Some(v) = overrides.cursor {
        base.cursor = v;
    }
    if let Some(v) = overrides.selection {
        base.selection = v;
    }
    if let Some(v) = overrides.line_highlight {
        base.line_highlight = v;
    }
    if let Some(v) = overrides.gutter_bg {
        base.gutter_bg = v;
    }
    if let Some(v) = overrides.gutter_fg {
        base.gutter_fg = v;
    }
    if let Some(v) = overrides.added_bg {
        base.added_bg = v;
    }
    if let Some(v) = overrides.removed_bg {
        base.removed_bg = v;
    }
    if let Some(v) = overrides.added_text {
        base.added_text = v;
    }
    if let Some(v) = overrides.removed_text {
        base.removed_text = v;
    }
    if overrides.syntax_keyword.is_some() {
        base.syntax_keyword = overrides.syntax_keyword;
    }
    if overrides.syntax_string.is_some() {
        base.syntax_string = overrides.syntax_string;
    }
    if overrides.syntax_comment.is_some() {
        base.syntax_comment = overrides.syntax_comment;
    }
    if overrides.syntax_function.is_some() {
        base.syntax_function = overrides.syntax_function;
    }
    if overrides.syntax_type.is_some() {
        base.syntax_type = overrides.syntax_type;
    }
    if overrides.syntax_number.is_some() {
        base.syntax_number = overrides.syntax_number;
    }
    if overrides.syntax_operator.is_some() {
        base.syntax_operator = overrides.syntax_operator;
    }
    if overrides.syntax_property.is_some() {
        base.syntax_property = overrides.syntax_property;
    }
}

// -- Parsing and validation --

/// Parse and validate a TOML theme string into a [`Theme`].
///
/// Only `[meta]` and `[colors]` are required. `[graph]` and `[editor]` are
/// derived from the base palette when omitted. Partial overrides are merged
/// on top of the derived defaults.
pub fn parse_theme(toml_str: &str) -> Result<Theme, ThemeError> {
    let raw: RawTheme = toml::from_str(toml_str)?;

    let meta = raw
        .meta
        .ok_or_else(|| ThemeError::MissingField("meta".to_string()))?;
    let colors = raw
        .colors
        .ok_or_else(|| ThemeError::MissingField("colors".to_string()))?;

    // Validate mode
    if meta.mode != "dark" && meta.mode != "light" {
        return Err(ThemeError::InvalidMode);
    }

    let is_dark = meta.mode == "dark";

    // Validate all 18 base color fields
    validate_color("colors.background", &colors.background)?;
    validate_color("colors.foreground", &colors.foreground)?;
    validate_color("colors.black", &colors.black)?;
    validate_color("colors.red", &colors.red)?;
    validate_color("colors.green", &colors.green)?;
    validate_color("colors.yellow", &colors.yellow)?;
    validate_color("colors.blue", &colors.blue)?;
    validate_color("colors.magenta", &colors.magenta)?;
    validate_color("colors.cyan", &colors.cyan)?;
    validate_color("colors.white", &colors.white)?;
    validate_color("colors.bright_black", &colors.bright_black)?;
    validate_color("colors.bright_red", &colors.bright_red)?;
    validate_color("colors.bright_green", &colors.bright_green)?;
    validate_color("colors.bright_yellow", &colors.bright_yellow)?;
    validate_color("colors.bright_blue", &colors.bright_blue)?;
    validate_color("colors.bright_magenta", &colors.bright_magenta)?;
    validate_color("colors.bright_cyan", &colors.bright_cyan)?;
    validate_color("colors.bright_white", &colors.bright_white)?;

    // Derive semantic colors from base palette, then layer the
    // per-theme [accents] overrides (if any) on top.
    let mut derived = derive_semantic_colors(&colors, is_dark);
    if let Some(accents) = raw.accents.as_ref() {
        apply_accent_overrides(&mut derived, &colors, accents);
    }

    // Derive graph from base palette + derived, then merge overrides
    let mut graph = derive_graph(&colors, &derived);
    if let Some(overrides) = raw.graph {
        merge_graph_overrides(&mut graph, overrides);
    }

    // Validate graph lane colors
    if graph.lane_colors.len() < 2 {
        return Err(ThemeError::InsufficientLaneColors);
    }
    // The user-supplied `[graph] lane-colors` array is merged without per-entry
    // validation otherwise, so an invalid hex would reach the frontend. Reject
    // it here (load_user_themes filters out themes that fail to parse).
    for (i, c) in graph.lane_colors.iter().enumerate() {
        validate_color(&format!("graph.lane_colors[{i}]"), c)?;
    }

    // Derive editor from base palette + derived, then merge overrides
    let mut editor = derive_editor(&colors, &derived, is_dark);
    if let Some(overrides) = raw.editor {
        merge_editor_overrides(&mut editor, overrides);
    }

    Ok(Theme {
        meta,
        colors,
        derived,
        graph,
        editor: Some(editor),
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
            complementary: self.meta.complementary.clone(),
        }
    }
}

// -- Loading functions --

/// Parse and return all built-in themes, skipping any that fail to parse.
pub fn load_builtin_themes() -> Vec<Theme> {
    [
        BEARDGIT_DARK_TOML,
        BEARDGIT_LIGHT_TOML,
        FJORD_DARK_TOML,
        FJORD_LIGHT_TOML,
        NEBULA_DARK_TOML,
        NEBULA_LIGHT_TOML,
        GITHUB_DARK_TOML,
        GITHUB_LIGHT_TOML,
        GITLAB_DARK_TOML,
        GITLAB_LIGHT_TOML,
        DRACULA_TOML,
        ONE_DARK_TOML,
        CATPPUCCIN_MOCHA_TOML,
        CATPPUCCIN_LATTE_TOML,
        NORD_TOML,
        TOKYO_NIGHT_TOML,
        SOLARIZED_DARK_TOML,
        SOLARIZED_LIGHT_TOML,
        GRUVBOX_DARK_TOML,
        MONOKAI_PRO_TOML,
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
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "toml"))
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
    parse_theme(BEARDGIT_DARK_TOML).expect("built-in beardgit-dark theme must parse")
}

/// Resolve the correct theme when the OS switches between dark and light mode.
///
/// Logic:
/// 1. If the current theme's mode already matches `target_mode`, return its id.
/// 2. If the current theme has a `complementary`, and that theme exists with
///    a matching mode, return the complementary id.
/// 3. Otherwise fall back to the default theme for `target_mode`.
pub fn resolve_theme_for_mode(current_id: &str, os_dark: bool, themes_dir: &Path) -> String {
    let target_mode = if os_dark { "dark" } else { "light" };

    // Load the current theme to check its mode and complementary field.
    let current = resolve_theme(current_id, themes_dir);

    // Already the right mode — keep it.
    if current.meta.mode == target_mode {
        return current_id.to_string();
    }

    // Try the complementary theme.
    if let Some(ref comp_id) = current.meta.complementary {
        let comp = resolve_theme(comp_id, themes_dir);
        // Only use it if its mode actually matches and it's not the fallback.
        if comp.meta.mode == target_mode && comp.meta.id == *comp_id {
            return comp_id.clone();
        }
    }

    // Fallback to defaults.
    if os_dark {
        DEFAULT_DARK_THEME_ID.to_string()
    } else {
        DEFAULT_LIGHT_THEME_ID.to_string()
    }
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

    /// A minimal valid TOML — only `[meta]` + `[colors]`, no graph/editor.
    const MINIMAL_THEME: &str = r##"
[meta]
id = "test-theme"
name = "Test Theme"
mode = "dark"

[colors]
background = "#111111"
foreground = "#eeeeee"
black = "#333333"
red = "#ff0000"
green = "#00ff00"
yellow = "#ff8800"
blue = "#0000ff"
magenta = "#8800ff"
cyan = "#00ffff"
white = "#cccccc"
bright-black = "#999999"
bright-red = "#ff4444"
bright-green = "#44ff44"
bright-yellow = "#ffaa44"
bright-blue = "#4444ff"
bright-magenta = "#aa44ff"
bright-cyan = "#44ffff"
bright-white = "#ffffff"
"##;

    #[test]
    fn test_parse_minimal_theme_derives_graph_and_editor() {
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        assert_eq!(theme.meta.id, "test-theme");
        assert_eq!(theme.meta.mode, "dark");
        // Base colors
        assert_eq!(theme.colors.background, "#111111");
        assert_eq!(theme.colors.foreground, "#eeeeee");
        assert_eq!(theme.colors.blue, "#0000ff");
        // Derived semantic colors
        assert_eq!(theme.derived.bg_primary, "#111111");
        assert_eq!(theme.derived.text_primary, "#eeeeee");
        assert_eq!(theme.derived.accent_blue, "#0000ff");
        assert_eq!(theme.derived.accent_green, "#00ff00");
        assert_eq!(theme.derived.accent_red, "#ff0000");
        assert_eq!(theme.derived.accent_orange, "#ff8800"); // yellow → orange
        assert_eq!(theme.derived.accent_purple, "#8800ff"); // magenta → purple
        // Graph derived from derived colors
        assert_eq!(theme.graph.background, "#111111");
        assert_eq!(theme.graph.ref_branch, "#00ff00");
        assert_eq!(theme.graph.lane_colors.len(), 10);
        // Editor derived
        assert!(theme.editor.is_some());
        let ed = theme.editor.unwrap();
        assert_eq!(ed.background, "#111111");
        assert_eq!(ed.cursor, "#0000ff");
        assert_eq!(ed.added_text, "#00ff00");
        assert_eq!(ed.removed_text, "#ff0000");
    }

    #[test]
    fn test_accents_default_to_blue_magenta_green() {
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        assert_eq!(theme.derived.accent_primary, theme.colors.blue);
        assert_eq!(theme.derived.accent_secondary, theme.colors.magenta);
        assert_eq!(theme.derived.accent_tertiary, theme.colors.green);
    }

    #[test]
    fn test_accents_override_picks_named_ansi_color() {
        let toml = format!(
            r##"{}
[accents]
primary = "magenta"
secondary = "cyan"
tertiary = "yellow"
"##,
            MINIMAL_THEME
        );
        let theme = parse_theme(&toml).unwrap();
        assert_eq!(theme.derived.accent_primary, theme.colors.magenta);
        assert_eq!(theme.derived.accent_secondary, theme.colors.cyan);
        assert_eq!(theme.derived.accent_tertiary, theme.colors.yellow);
    }

    #[test]
    fn test_accents_override_accepts_hex_literal() {
        let toml = format!(
            r##"{}
[accents]
primary = "#ff00ff"
"##,
            MINIMAL_THEME
        );
        let theme = parse_theme(&toml).unwrap();
        assert_eq!(theme.derived.accent_primary, "#ff00ff");
    }

    #[test]
    fn test_accents_unknown_name_falls_back_to_default() {
        let toml = format!(
            r##"{}
[accents]
primary = "neon-pink"
"##,
            MINIMAL_THEME
        );
        let theme = parse_theme(&toml).unwrap();
        assert_eq!(theme.derived.accent_primary, theme.colors.blue);
    }

    #[test]
    fn test_graph_override_merges_on_top_of_derived() {
        let toml = format!(
            r##"{}
[graph]
node-radius = 6.0
ref-branch = "#aabbcc"
"##,
            MINIMAL_THEME
        );
        let theme = parse_theme(&toml).unwrap();
        assert_eq!(theme.graph.node_radius, 6.0);
        assert_eq!(theme.graph.ref_branch, "#aabbcc");
        // Non-overridden fields still derived
        assert_eq!(theme.graph.background, "#111111");
        assert_eq!(theme.graph.ref_remote, "#0000ff");
    }

    #[test]
    fn test_editor_override_merges_on_top_of_derived() {
        let toml = format!(
            r##"{}
[editor]
added-bg = "#114411"
syntax-keyword = "#ff0000"
"##,
            MINIMAL_THEME
        );
        let theme = parse_theme(&toml).unwrap();
        let ed = theme.editor.unwrap();
        assert_eq!(ed.added_bg, "#114411");
        assert_eq!(ed.syntax_keyword, Some("#ff0000".to_string()));
        // Non-overridden fields still derived
        assert_eq!(ed.background, "#111111");
        assert_eq!(ed.cursor, "#0000ff");
    }

    #[test]
    fn test_parse_missing_meta() {
        let toml = r##"
[colors]
background = "#111111"
foreground = "#eeeeee"
black = "#333333"
red = "#ff0000"
green = "#00ff00"
yellow = "#ff8800"
blue = "#0000ff"
magenta = "#8800ff"
cyan = "#00ffff"
white = "#cccccc"
bright-black = "#999999"
bright-red = "#ff4444"
bright-green = "#44ff44"
bright-yellow = "#ffaa44"
bright-blue = "#4444ff"
bright-magenta = "#aa44ff"
bright-cyan = "#44ffff"
bright-white = "#ffffff"
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
"##;
        let err = parse_theme(toml).unwrap_err();
        // Can be "missing required field: colors" or TOML parse error
        let msg = err.to_string();
        assert!(msg.contains("colors") || msg.contains("missing"));
    }

    #[test]
    fn test_parse_invalid_mode() {
        let toml = MINIMAL_THEME.replace("mode = \"dark\"", "mode = \"neon\"");
        let err = parse_theme(&toml).unwrap_err();
        assert!(err.to_string().contains("mode"));
    }

    #[test]
    fn test_parse_lane_colors_override_validated() {
        let toml = format!(
            r##"{}
[graph]
lane-colors = ["#0000ff"]
"##,
            MINIMAL_THEME
        );
        let err = parse_theme(&toml).unwrap_err();
        assert!(err.to_string().contains("lane-colors"));
    }

    #[test]
    fn test_parse_invalid_color_format() {
        let toml = MINIMAL_THEME.replace(r##"background = "#111111""##, r##"background = "nope""##);
        let err = parse_theme(&toml).unwrap_err();
        assert!(err.to_string().contains("invalid color"));
    }

    #[test]
    fn test_parse_rgba_color_accepted() {
        let toml = MINIMAL_THEME.replace(
            r##"blue = "#0000ff""##,
            r##"blue = "rgba(0, 0, 255, 1.0)""##,
        );
        let theme = parse_theme(&toml).unwrap();
        assert_eq!(theme.colors.blue, "rgba(0, 0, 255, 1.0)");
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
        assert_eq!(themes.len(), 20);
    }

    #[test]
    fn test_builtin_themes_have_correct_modes() {
        let themes = load_builtin_themes();
        let dark_count = themes.iter().filter(|t| t.meta.mode == "dark").count();
        let light_count = themes.iter().filter(|t| t.meta.mode == "light").count();
        assert_eq!(dark_count, 13);
        assert_eq!(light_count, 7);
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

    #[test]
    fn test_with_alpha() {
        assert_eq!(with_alpha("#58a6ff", "22"), "#58a6ff22");
        assert_eq!(with_alpha("#58a6ff33", "22"), "#58a6ff33"); // already has alpha
        assert_eq!(with_alpha("rgba(0,0,0,1)", "22"), "rgba(0,0,0,1)"); // not hex
    }

    #[test]
    fn test_lighten_hex() {
        let lighter = lighten_hex("#000000", 0.5);
        assert_eq!(lighter, "#7f7f7f");
        let white = lighten_hex("#000000", 1.0);
        assert_eq!(white, "#ffffff");
    }

    #[test]
    fn test_light_theme_derives_different_diff_colors() {
        let toml = MINIMAL_THEME.replace("mode = \"dark\"", "mode = \"light\"");
        let theme = parse_theme(&toml).unwrap();
        let ed = theme.editor.unwrap();
        assert_eq!(ed.added_bg, "#d4f8db");
        assert_eq!(ed.removed_bg, "#fdd8d8");
    }

    #[test]
    fn test_editor_always_some_after_parse() {
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        assert!(theme.editor.is_some());
    }

    // -- with_alpha edge cases --

    #[test]
    fn test_with_alpha_empty_string() {
        assert_eq!(with_alpha("", "ff"), "");
    }

    #[test]
    fn test_with_alpha_short_hex() {
        // #RGB (4 chars) is not #RRGGBB — pass through unchanged
        assert_eq!(with_alpha("#abc", "ff"), "#abc");
    }

    #[test]
    fn test_with_alpha_rgba_input_unchanged() {
        assert_eq!(
            with_alpha("rgba(88, 166, 255, 0.2)", "33"),
            "rgba(88, 166, 255, 0.2)"
        );
    }

    #[test]
    fn test_with_alpha_rrggbbaa_unchanged() {
        // #RRGGBBAA already has alpha — return as-is
        assert_eq!(with_alpha("#58a6ff99", "44"), "#58a6ff99");
    }

    // -- strip_alpha edge cases --

    #[test]
    fn test_strip_alpha_no_alpha() {
        // #RRGGBB already has no alpha — return unchanged
        assert_eq!(strip_alpha("#58a6ff"), "#58a6ff");
    }

    #[test]
    fn test_strip_alpha_removes_aa() {
        assert_eq!(strip_alpha("#58a6ff33"), "#58a6ff");
    }

    #[test]
    fn test_strip_alpha_non_hex() {
        assert_eq!(strip_alpha("rgba(0, 0, 0, 0.5)"), "rgba(0, 0, 0, 0.5)");
    }

    #[test]
    fn test_strip_alpha_empty() {
        assert_eq!(strip_alpha(""), "");
    }

    // -- lighten_hex edge cases --

    #[test]
    fn test_lighten_hex_white_stays_white() {
        assert_eq!(lighten_hex("#ffffff", 0.5), "#ffffff");
    }

    #[test]
    fn test_lighten_hex_middle_gray() {
        // #808080 lightened by 0.5: each channel = 128 + (255-128)*0.5 = 191 = 0xbf
        let result = lighten_hex("#808080", 0.5);
        assert_eq!(result, "#bfbfbf");
    }

    #[test]
    fn test_lighten_hex_amount_zero_unchanged() {
        assert_eq!(lighten_hex("#123456", 0.0), "#123456");
    }

    #[test]
    fn test_lighten_hex_non_hex_passthrough() {
        assert_eq!(lighten_hex("rgba(0,0,0,1)", 0.5), "rgba(0,0,0,1)");
    }

    #[test]
    fn test_lighten_hex_short_hex_passthrough() {
        // Less than 7 chars — pass through
        assert_eq!(lighten_hex("#fff", 0.5), "#fff");
    }

    // -- shift_hue_hex --

    #[test]
    fn test_shift_hue_hex_zero_degrees_is_identity() {
        assert_eq!(shift_hue_hex("#ff0000", 0), "#ff0000");
    }

    #[test]
    fn test_shift_hue_hex_360_degrees_is_identity() {
        // 360° rotation = full cycle = same color
        assert_eq!(shift_hue_hex("#ff0000", 360), "#ff0000");
    }

    #[test]
    fn test_shift_hue_hex_positive_shift() {
        // Red (#ff0000) shifted +120° → Green (#00ff00)
        let result = shift_hue_hex("#ff0000", 120);
        assert_eq!(result, "#00ff00");
    }

    #[test]
    fn test_shift_hue_hex_negative_shift() {
        // Green (#00ff00) shifted -120° → Red (#ff0000)
        let result = shift_hue_hex("#00ff00", -120);
        assert_eq!(result, "#ff0000");
    }

    #[test]
    fn test_shift_hue_hex_achromatic_returns_same() {
        // Gray is achromatic — no hue to shift, return unchanged
        assert_eq!(shift_hue_hex("#808080", 90), "#808080");
        assert_eq!(shift_hue_hex("#000000", 45), "#000000");
        assert_eq!(shift_hue_hex("#ffffff", 180), "#ffffff");
    }

    #[test]
    fn test_shift_hue_hex_non_hex_passthrough() {
        assert_eq!(shift_hue_hex("rgba(255,0,0,1)", 90), "rgba(255,0,0,1)");
    }

    #[test]
    fn test_shift_hue_hex_short_passthrough() {
        assert_eq!(shift_hue_hex("#f00", 90), "#f00");
    }

    // -- derive_graph direct tests (via parse_theme) --

    #[test]
    fn test_derive_graph_lane_colors_count() {
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        assert_eq!(theme.graph.lane_colors.len(), 10);
    }

    #[test]
    fn test_derive_graph_specific_mappings() {
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        // background = derived.bg_primary
        assert_eq!(theme.graph.background, theme.derived.bg_primary);
        // ref_branch = derived.accent_green
        assert_eq!(theme.graph.ref_branch, theme.derived.accent_green);
        // ref_remote = derived.accent_blue
        assert_eq!(theme.graph.ref_remote, theme.derived.accent_blue);
        // ref_tag = derived.accent_orange
        assert_eq!(theme.graph.ref_tag, theme.derived.accent_orange);
        // ref_head = derived.accent_purple
        assert_eq!(theme.graph.ref_head, theme.derived.accent_purple);
        // text_sha = derived.accent_blue
        assert_eq!(theme.graph.text_sha, theme.derived.accent_blue);
        // foreground = derived.text_primary
        assert_eq!(theme.graph.foreground, theme.derived.text_primary);
    }

    #[test]
    fn test_derive_graph_first_five_lane_colors_are_accents() {
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        let lc = &theme.graph.lane_colors;
        assert_eq!(lc[0], theme.derived.accent_blue);
        assert_eq!(lc[1], theme.derived.accent_green);
        assert_eq!(lc[2], theme.derived.accent_orange);
        assert_eq!(lc[3], theme.derived.accent_purple);
        assert_eq!(lc[4], theme.derived.accent_red);
    }

    // -- derive_editor direct tests (via parse_theme) --

    #[test]
    fn test_derive_editor_dark_diff_colors() {
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        let ed = theme.editor.unwrap();
        assert_eq!(ed.added_bg, "#1b3829");
        assert_eq!(ed.removed_bg, "#3c1e22");
    }

    #[test]
    fn test_derive_editor_light_diff_colors() {
        let toml = MINIMAL_THEME.replace("mode = \"dark\"", "mode = \"light\"");
        let theme = parse_theme(&toml).unwrap();
        let ed = theme.editor.unwrap();
        assert_eq!(ed.added_bg, "#d4f8db");
        assert_eq!(ed.removed_bg, "#fdd8d8");
    }

    #[test]
    fn test_derive_editor_syntax_colors_from_accent() {
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        let ed = theme.editor.unwrap();
        // keyword = derived.accent_red
        assert_eq!(ed.syntax_keyword, Some(theme.derived.accent_red.clone()));
        // operator = derived.accent_red (same as keyword)
        assert_eq!(ed.syntax_operator, Some(theme.derived.accent_red.clone()));
        // function = derived.accent_purple
        assert_eq!(
            ed.syntax_function,
            Some(theme.derived.accent_purple.clone())
        );
        // type = derived.accent_blue
        assert_eq!(ed.syntax_type, Some(theme.derived.accent_blue.clone()));
        // number = derived.accent_blue (same as type)
        assert_eq!(ed.syntax_number, Some(theme.derived.accent_blue.clone()));
        // property = derived.accent_green
        assert_eq!(ed.syntax_property, Some(theme.derived.accent_green.clone()));
        // comment = None (frontend uses text-secondary)
        assert_eq!(ed.syntax_comment, None);
    }

    #[test]
    fn test_derive_editor_cursor_and_selection() {
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        let ed = theme.editor.unwrap();
        // cursor = derived.accent_blue
        assert_eq!(ed.cursor, theme.derived.accent_blue);
        // gutter_bg = derived.bg_primary
        assert_eq!(ed.gutter_bg, theme.derived.bg_primary);
        // gutter_fg = derived.text_secondary
        assert_eq!(ed.gutter_fg, theme.derived.text_secondary);
    }

    // -- merge_graph_overrides direct tests (via parse_theme with partial graph) --

    #[test]
    fn test_merge_graph_override_one_field_others_unchanged() {
        let toml = format!(
            r##"{}
[graph]
dim-opacity = 0.8
"##,
            MINIMAL_THEME
        );
        let theme = parse_theme(&toml).unwrap();
        assert_eq!(theme.graph.dim_opacity, 0.8);
        // Derived defaults preserved
        assert_eq!(theme.graph.node_radius, 4.0);
        assert_eq!(theme.graph.merge_radius, 3.0);
        assert_eq!(theme.graph.ref_branch, theme.derived.accent_green);
    }

    #[test]
    fn test_merge_graph_override_lane_colors_with_valid_count() {
        let toml = format!(
            r##"{}
[graph]
lane-colors = ["#aabbcc", "#ddeeff"]
"##,
            MINIMAL_THEME
        );
        let theme = parse_theme(&toml).unwrap();
        assert_eq!(theme.graph.lane_colors, vec!["#aabbcc", "#ddeeff"]);
    }

    // -- merge_editor_overrides direct tests (via parse_theme with partial editor) --

    #[test]
    fn test_merge_editor_override_one_field_others_unchanged() {
        let toml = format!(
            r##"{}
[editor]
removed-bg = "#ff000033"
"##,
            MINIMAL_THEME
        );
        let theme = parse_theme(&toml).unwrap();
        let ed = theme.editor.unwrap();
        assert_eq!(ed.removed_bg, "#ff000033");
        // Dark-mode added-bg still derived
        assert_eq!(ed.added_bg, "#1b3829");
        // Other fields still derived
        assert_eq!(ed.cursor, theme.derived.accent_blue);
    }

    #[test]
    fn test_merge_editor_override_syntax_color() {
        let toml = format!(
            r##"{}
[editor]
syntax-comment = "#888888"
"##,
            MINIMAL_THEME
        );
        let theme = parse_theme(&toml).unwrap();
        let ed = theme.editor.unwrap();
        assert_eq!(ed.syntax_comment, Some("#888888".to_string()));
        // Other syntax fields still derived
        assert_eq!(ed.syntax_keyword, Some(theme.derived.accent_red.clone()));
    }

    #[test]
    fn test_resolve_theme_for_mode_already_correct() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_theme_for_mode("github-dark", true, dir.path());
        assert_eq!(result, "github-dark");
    }

    #[test]
    fn test_resolve_theme_for_mode_uses_complementary() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_theme_for_mode("github-dark", false, dir.path());
        assert_eq!(result, "github-light");
    }

    #[test]
    fn test_resolve_theme_for_mode_complementary_reverse() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_theme_for_mode("github-light", true, dir.path());
        assert_eq!(result, "github-dark");
    }

    #[test]
    fn test_resolve_theme_for_mode_no_complementary_falls_back() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_theme_for_mode("dracula", false, dir.path());
        assert_eq!(result, DEFAULT_LIGHT_THEME_ID);
    }

    #[test]
    fn test_resolve_theme_for_mode_unpaired_dark_stays() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_theme_for_mode("dracula", true, dir.path());
        assert_eq!(result, "dracula");
    }
}
