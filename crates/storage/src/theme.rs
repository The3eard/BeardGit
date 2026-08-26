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
const ROSE_PINE_MOON_TOML: &str = include_str!("themes/rose_pine_moon.toml");
const ROSE_PINE_DAWN_TOML: &str = include_str!("themes/rose_pine_dawn.toml");
const EVERFOREST_DARK_TOML: &str = include_str!("themes/everforest_dark.toml");
const EVERFOREST_LIGHT_TOML: &str = include_str!("themes/everforest_light.toml");
const KANAGAWA_TOML: &str = include_str!("themes/kanagawa.toml");
const AYU_DARK_TOML: &str = include_str!("themes/ayu_dark.toml");
const AYU_MIRAGE_TOML: &str = include_str!("themes/ayu_mirage.toml");
const AYU_LIGHT_TOML: &str = include_str!("themes/ayu_light.toml");
const MATERIAL_TOML: &str = include_str!("themes/material.toml");
const ZENBURN_TOML: &str = include_str!("themes/zenburn.toml");
const OXOCARBON_TOML: &str = include_str!("themes/oxocarbon.toml");

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
- **Editor:** syntax highlighting derived from ANSI colors (keyword red,
  string green, function magenta, type blue, number yellow, property cyan);
  diff backgrounds blended from the theme's green/red over its background;
  cursor and selection from blue
- **All other UI elements** are styled via CSS custom properties from derived colors

## Optional Overrides

To tweak specific derived values, add a partial `[accents]`, `[derived]`,
`[graph]` or `[editor]` section. Only the fields you include are overridden —
everything else keeps the derived value.

```toml
[accents]
primary = "cyan"              # an ANSI colour name, or a literal hex
secondary = "#c678dd"

[derived]
text-secondary = "#969ead"    # UI text tokens, when the derived ones are too dim
text-muted = "#78808e"

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

### Accent fields
- `primary`, `secondary`, `tertiary` — the signature accents. Each takes an
  ANSI colour name (`"cyan"`, `"bright_magenta"`, …) or a literal hex.

### Derived fields — fixing low-contrast text

`text-secondary` is derived from `bright-black`, and `text-muted` from that
blended toward the page. In a palette whose `bright-black` sits close to the
background, that lands below the readable threshold — and BeardGit will tell
you so in Settings → General rather than changing your colours for you.

Raising `bright-black` itself would also change your terminal's ANSI palette,
so pin the UI text tokens here instead:

- `text-primary`, `text-secondary`, `text-muted` — the three text rungs
- `border` — panel separators (accepts `#RRGGBBAA`)
- `border-strong` — outlines around inputs, selects and buttons

All three text rungs want at least 4.5:1 (WCAG AA for normal text), and
`border-strong` 3:1. Not against `background` alone: the check measures
each token against every surface it is drawn on, which for text means the
page and the panels, and for the borders the toolbar as well — a colour
solved for the page alone can be a full point dimmer up there. `border`
takes a lower 2:1, being a divider rather than something you have to read.

Every bundled theme is checked against those floors; yours is only
reported.

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
    /// A third, dimmer text step for de-emphasised metadata (timestamps,
    /// paths, counts). Sits between `text_secondary` and the surface so
    /// the type hierarchy has three rungs instead of two. Derived per
    /// mode and held at WCAG AA for normal text — it renders at 10px, so
    /// the large-text allowance it used to claim never applied.
    pub text_muted: String,
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
    /// Panel and card separators. Held to a lower floor than
    /// [`Self::border_strong`] because separation is carried mainly by the
    /// elevation ramp; the line only refines it.
    pub border: String,
    /// Outline for interactive controls — inputs, selects, textareas,
    /// buttons.
    ///
    /// Split out from [`Self::border`] because one token was doing two
    /// jobs with opposite requirements. WCAG 1.4.11 asks 3:1 for anything
    /// that *identifies* a control, and a control whose outline is
    /// invisible has no perceivable boundary — but applying 3:1 to every
    /// panel divider draws hard lines across the whole UI.
    pub border_strong: String,
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
    #[serde(alias = "line-highlight")]
    pub line_highlight: String,
    /// Gutter background color.
    #[serde(alias = "gutter-bg")]
    pub gutter_bg: String,
    /// Gutter foreground (line numbers) color.
    #[serde(alias = "gutter-fg")]
    pub gutter_fg: String,
    /// Added line background in diff view.
    #[serde(alias = "added-bg")]
    pub added_bg: String,
    /// Removed line background in diff view.
    #[serde(alias = "removed-bg")]
    pub removed_bg: String,
    /// Added line text color in diff view.
    #[serde(alias = "added-text")]
    pub added_text: String,
    /// Removed line text color in diff view.
    #[serde(alias = "removed-text")]
    pub removed_text: String,
    /// Syntax: keyword color.
    #[serde(default, alias = "syntax-keyword")]
    pub syntax_keyword: Option<String>,
    /// Syntax: string literal color.
    #[serde(default, alias = "syntax-string")]
    pub syntax_string: Option<String>,
    /// Syntax: comment color.
    #[serde(default, alias = "syntax-comment")]
    pub syntax_comment: Option<String>,
    /// Syntax: function/method name color.
    #[serde(default, alias = "syntax-function")]
    pub syntax_function: Option<String>,
    /// Syntax: type/class name color.
    #[serde(default, alias = "syntax-type")]
    pub syntax_type: Option<String>,
    /// Syntax: number literal color.
    #[serde(default, alias = "syntax-number")]
    pub syntax_number: Option<String>,
    /// Syntax: operator color.
    #[serde(default, alias = "syntax-operator")]
    pub syntax_operator: Option<String>,
    /// Syntax: property/attribute color.
    #[serde(default, alias = "syntax-property")]
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

/// Blend `base` toward `tint`, both `#RRGGBB`. `amount` is 0.0–1.0.
/// Non-hex inputs return `base` unchanged.
fn mix_hex(base: &str, tint: &str, amount: f64) -> String {
    if !base.starts_with('#') || base.len() < 7 || !tint.starts_with('#') || tint.len() < 7 {
        return base.to_string();
    }
    let ch = |s: &str, i: usize| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(128) as f64;
    let blend = |i: usize| (ch(base, i) + (ch(tint, i) - ch(base, i)) * amount) as u8;
    format!("#{:02x}{:02x}{:02x}", blend(1), blend(3), blend(5))
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

// ─── WCAG contrast ───────────────────────────────────────────────────────

/// Parse `#RRGGBB` or `#RRGGBBAA` into linear-light sRGB components.
///
/// Alpha is ignored here rather than composited. Callers that may be
/// handed a translucent value flatten it first with `composite_over` —
/// which the border tokens do, since they are audited against every
/// surface they are drawn on.
fn srgb_channels(hex: &str) -> Option<[f64; 3]> {
    // Expand `#abc` to `#aabbcc` first — the short form is legal CSS and a
    // user theme may well use it, and silently failing to parse it means
    // silently not auditing it.
    let expanded = match hex.strip_prefix('#') {
        Some(short) if short.len() == 3 => {
            let mut out = String::with_capacity(7);
            out.push('#');
            for c in short.chars() {
                out.push(c);
                out.push(c);
            }
            out
        }
        _ => hex.to_string(),
    };
    let hex = expanded.as_str();
    if !hex.starts_with('#') || (hex.len() != 7 && hex.len() != 9) {
        return None;
    }
    let channel = |i: usize| -> Option<f64> {
        let raw = u8::from_str_radix(hex.get(i..i + 2)?, 16).ok()? as f64 / 255.0;
        // sRGB → linear light, per WCAG 2.x relative-luminance definition.
        Some(if raw <= 0.040_45 {
            raw / 12.92
        } else {
            ((raw + 0.055) / 1.055).powf(2.4)
        })
    };
    Some([channel(1)?, channel(3)?, channel(5)?])
}

/// WCAG 2.x relative luminance of an opaque `#RRGGBB` color.
fn relative_luminance(hex: &str) -> Option<f64> {
    let [r, g, b] = srgb_channels(hex)?;
    Some(0.2126 * r + 0.7152 * g + 0.0722 * b)
}

/// WCAG contrast ratio between two colors, from 1.0 (identical) to 21.0
/// (black on white). Returns `None` if either color isn't parseable hex.
///
/// Order-independent: the lighter color is always the numerator.
pub fn contrast_ratio(a: &str, b: &str) -> Option<f64> {
    let (la, lb) = (relative_luminance(a)?, relative_luminance(b)?);
    let (lighter, darker) = if la >= lb { (la, lb) } else { (lb, la) };
    Some((lighter + 0.05) / (darker + 0.05))
}

/// The minimum contrast ratio a token must reach on every surface it is
/// drawn on. See [`audit_surfaces`] for which surfaces those are.
///
/// All three text rungs take the WCAG AA normal-text floor of 4.5:1.
/// `text_muted` used to sit at 3:1 on a large-text exemption it never
/// qualified for — it renders at 10px (`--font-size-2xs`) in the staging
/// area and sidebar, where the allowance needs ≥18.66px bold or ≥24px.
///
/// The two border tokens are split because one token was serving panel
/// separators and control outlines, whose requirements diverge:
///
/// - `border_strong` outlines interactive controls, which WCAG 1.4.11
///   covers, so it takes the full 3:1.
/// - `border` draws separators and takes 2.0:1 — a *safety floor*, not a
///   design target. It marks the point below which the line is simply not
///   there (the previous translucent derivation reached 1.27:1 on
///   gruvbox-dark), and it exists for user themes, which are reported and
///   never modified. It deliberately does not describe what the bundled
///   themes do: `derive_semantic_colors` solves `border` against
///   `bg_toolbar`, the most elevated surface it is drawn on, which lands
///   it at 2.76–3.45:1 on the page — above 1.4.11's 3:1 on every dark
///   theme. That is the accepted trade: a divider that is always visible,
///   including on the toolbar, is worth more than one tuned to be barely
///   perceptible on the page and invisible above it.
///
/// `selection` stays unaudited: it overlays arbitrary content — text, diff
/// rows, graph lanes — so there is no single pair to measure.
pub fn contrast_floor(token: &str) -> Option<f64> {
    match token {
        "text_primary" | "text_secondary" | "text_muted" => Some(4.5),
        // A safety floor for user themes, not a design target — the
        // bundled derivation lands well above it. See the doc comment.
        "border" => Some(2.0),
        // Control outlines DO identify a component, so these take the
        // full 1.4.11 requirement.
        "border_strong" => Some(3.0),
        // `selection` carries alpha over arbitrary content — text, diff
        // rows, graph lanes — so there is no single pair to measure.
        _ => None,
    }
}

/// The surfaces a token has to clear its floor on.
///
/// Auditing everything against `bg_primary` alone reported ratios the eye
/// never sees: an opaque border solved for the page measured 1.42:1 on the
/// toolbar, and `text_muted` solved for the page measured 3.60:1 on a
/// panel. A token is only legible if it is legible everywhere it is drawn,
/// so each one is measured against its own surface set and reported at its
/// worst.
///
/// `text_muted` is the one token that does not take all three. Its four
/// callsites — `StagingArea.svelte` `.commit-hint`, `Sidebar.svelte`
/// `.group-label`, `FileStatusBadge.svelte` `.is-unknown`, and
/// `+page.svelte` `.welcome-dropzone` — sit on `--bg-primary` or
/// `--bg-secondary`; none is inside a toolbar-rooted component. Including
/// `bg_toolbar` anyway is not free: muted would have to clear 4.5:1 there,
/// which pushes it past where `text_secondary` sits today, and holding the
/// three rungs apart from that point washes all 31 palettes toward white
/// (`text_primary` at 7.5:1 on the toolbar is 11.6:1 on the page). So the
/// set follows the callsites.
///
/// **These are claims about CSS that Rust cannot verify**, and they have
/// two failure modes, only one of which is obvious:
///
/// - A token gets used on a surface that is not in its set. Putting
///   `--text-muted` on a `--bg-toolbar` surface would land it at ~3.7:1
///   with nothing failing here. Add `bg_toolbar` to its set if that
///   happens.
/// - A component *invents* a surface out of the token itself. That is not
///   hypothetical: `FileStatusBadge` paints
///   `background: color-mix(var(--st) 18%, transparent)` under a letter
///   coloured by the same `--st`, and its `.is-unknown` kind took
///   `--text-muted` — 4.04:1 on the page and 3.37:1 on a panel, both
///   under the floor this function reports as met. No token dimmer than
///   `text_primary` survives being drawn on a tint of itself
///   (`text_secondary` bottoms out at 3.82), so the rule is that audited
///   text tokens do not get self-tinted fills, guarded by
///   `FileStatusBadge.test.ts`.
fn audit_surfaces<'a>(token: &str, d: &'a DerivedColors) -> Vec<&'a String> {
    match token {
        "text_muted" => vec![&d.bg_primary, &d.bg_secondary],
        _ => vec![&d.bg_primary, &d.bg_secondary, &d.bg_toolbar],
    }
}

/// Flatten `#RRGGBBAA` over an opaque background.
///
/// The audited tokens are opaque as derived, but `[derived]` accepts an
/// 8-digit value, so a user-pinned translucent border has to be measured
/// as it renders rather than as it is written.
fn composite_over(foreground: &str, background: &str) -> String {
    let (Some(fg), Some(bg)) = (foreground.strip_prefix('#'), background.strip_prefix('#')) else {
        return foreground.to_string();
    };
    if fg.len() != 8 {
        return foreground.to_string();
    }
    let byte = |s: &str, i: usize| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0) as f64;
    let alpha = byte(fg, 6) / 255.0;
    let channel = |i: usize| (byte(fg, i) * alpha + byte(bg, i) * (1.0 - alpha)) as u8;
    format!("#{:02x}{:02x}{:02x}", channel(0), channel(2), channel(4))
}

/// One token that falls below its contrast floor on at least one of the
/// surfaces it is drawn on.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContrastWarning {
    /// The `DerivedColors` field name, e.g. `"text_secondary"`.
    pub token: String,
    /// The token's resolved color.
    pub foreground: String,
    /// The worst surface it was measured against — the one this ratio is
    /// from, not necessarily the page.
    pub background: String,
    /// Measured WCAG ratio, rounded to two decimals.
    pub ratio: f64,
    /// The floor this token was required to meet.
    pub required: f64,
}

/// Accessibility report for one theme.
///
/// Empty `warnings` means every audited token clears its floor. This is
/// advisory only: user themes are never modified, they are only reported.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThemeContrastReport {
    /// The theme this describes.
    pub theme_id: String,
    /// Tokens below their floor. Empty when the theme passes.
    pub warnings: Vec<ContrastWarning>,
    /// Tokens whose colour could not be parsed as hex, so no ratio could
    /// be computed.
    ///
    /// Reported rather than dropped: `validate_color` accepts `rgba(…)`,
    /// so a theme written that way would otherwise come back looking clean
    /// no matter how illegible it is — a silent pass is the one outcome an
    /// accessibility check must never produce.
    pub unaudited: Vec<String>,
}

impl ThemeContrastReport {
    /// `true` when every audited token clears its floor **and** every token
    /// could actually be measured. An unparseable colour is not a pass.
    pub fn passes(&self) -> bool {
        self.warnings.is_empty() && self.unaudited.is_empty()
    }
}

/// Audit a theme's text and border tokens against every surface they are
/// drawn on.
///
/// Each token is measured against its own surface set (see
/// [`audit_surfaces`]) and reported at its worst: a token only counts as
/// legible if it is legible everywhere it appears.
pub fn check_theme_contrast(theme: &Theme) -> ThemeContrastReport {
    let d = &theme.derived;

    let mut warnings = Vec::new();
    let mut unaudited = Vec::new();

    for (token, foreground) in [
        ("text_primary", &d.text_primary),
        ("text_secondary", &d.text_secondary),
        ("text_muted", &d.text_muted),
        ("border", &d.border),
        ("border_strong", &d.border_strong),
    ] {
        let Some(required) = contrast_floor(token) else {
            continue;
        };
        let against = audit_surfaces(token, d);
        // Worst surface wins.
        let mut worst: Option<(f64, &String)> = None;
        let mut parse_failed = false;
        for surface in against {
            let flattened = composite_over(foreground, surface);
            match contrast_ratio(&flattened, surface) {
                Some(r) if worst.is_none_or(|(w, _)| r < w) => worst = Some((r, surface)),
                Some(_) => {}
                None => {
                    parse_failed = true;
                    break;
                }
            }
        }
        if parse_failed {
            unaudited.push(token.to_string());
            continue;
        }
        let Some((ratio, background)) = worst else {
            unaudited.push(token.to_string());
            continue;
        };
        if ratio < required {
            warnings.push(ContrastWarning {
                token: token.to_string(),
                foreground: foreground.clone(),
                background: background.clone(),
                ratio: (ratio * 100.0).round() / 100.0,
                required,
            });
        }
    }

    ThemeContrastReport {
        theme_id: theme.meta.id.clone(),
        warnings,
        unaudited,
    }
}

/// Blend `from` toward `toward` until the result clears `target` contrast
/// against `background`.
///
/// Returns `from` unchanged when it already clears the target, so a theme
/// whose own colours are legible keeps them verbatim. Steps in 0.5%
/// increments, which is finer than any perceivable difference and keeps
/// the result close to the palette's own hue line.
///
/// This is the mechanism that makes the accessibility floors hold for
/// *user* themes as well as bundled ones. Clamping here is legitimate
/// where clamping a declared colour would not be: these tokens are
/// derived, never authored — a theme file declares 18 ANSI colours, and
/// anything it does declare explicitly in `[derived]` still wins, because
/// that merge runs after this.
fn blend_to_contrast(from: &str, toward: &str, background: &str, target: f64) -> String {
    if contrast_ratio(from, background).is_some_and(|r| r >= target) {
        return from.to_string();
    }
    for step in 0..=200 {
        let candidate = mix_hex(from, toward, step as f64 / 200.0);
        if contrast_ratio(&candidate, background).is_some_and(|r| r >= target) {
            return candidate;
        }
    }
    // Unreachable for any parseable palette: `toward` is pure black or
    // white, which is 21:1 against anything. Fall back to the input.
    from.to_string()
}

/// Reach `target` while giving up as little of the palette's colour as
/// possible.
///
/// Tries `preferred` first — normally the theme's own foreground, which
/// keeps the result on a line between two of its real colours — and only
/// falls back to `last_resort` (pure white or black) when the preferred
/// anchor cannot get there, which happens when the foreground's own
/// contrast is below the target.
///
/// The two-stage form exists because blending straight to an extreme
/// desaturates: gruvbox-dark's `text_secondary` went from chroma 36 to 6,
/// turning its tan into near-neutral grey. Preferring the foreground keeps
/// the hue for the themes whose foreground is bright enough.
fn blend_preferring(
    from: &str,
    preferred: &str,
    last_resort: &str,
    background: &str,
    target: f64,
) -> String {
    let via_preferred = blend_to_contrast(from, preferred, background, target);
    if contrast_ratio(&via_preferred, background).is_some_and(|r| r >= target) {
        return via_preferred;
    }
    blend_to_contrast(from, last_resort, background, target)
}

/// Blend `from` toward `toward`, keeping the dimmest value that still
/// clears `target` against `measured_against`.
///
/// Used for `text_muted`, which has to sit *below* `text_secondary` while
/// staying legible. Solving upward from the palette would land it on the
/// same value as secondary in themes whose `bright_black` already clears
/// the higher floor.
///
/// `toward` and `measured_against` differ because muted is dimmed toward
/// the page but has to stay legible on the *panel*, which is the worst
/// surface it is drawn on. Dimming and measuring against the same surface
/// would leave it at 3.60:1 where it actually renders.
fn dim_to_contrast(from: &str, toward: &str, measured_against: &str, target: f64) -> String {
    let mut dimmest = from.to_string();
    for step in 0..=200 {
        let candidate = mix_hex(from, toward, step as f64 / 200.0);
        match contrast_ratio(&candidate, measured_against) {
            Some(r) if r >= target => dimmest = candidate,
            _ => break,
        }
    }
    dimmest
}

/// CIE L\*, the perceptual lightness of a color. Returns `None` for
/// anything that isn't parseable hex.
///
/// Contrast ratios are the wrong unit for spacing the text ramp: they are
/// a legibility measure, not a perceptual scale, and two rungs 1.2× apart
/// in ratio can be indistinguishable. L\* is roughly uniform, with a
/// just-noticeable difference around 1–2.
fn lstar(hex: &str) -> Option<f64> {
    let l = relative_luminance(hex)?;
    Some(if l > 0.008856 {
        116.0 * l.cbrt() - 16.0
    } else {
        903.3 * l
    })
}

/// Push `from` toward `toward` until it is at least `min_delta` L\* *past*
/// `below`, in the direction of `toward`, so the two read as distinct
/// rungs of one ramp.
///
/// The contrast floors alone no longer guarantee this. Once each rung is
/// raised to clear WCAG AA on the worst surface it is drawn on, the
/// low-contrast palettes bunch up: one-dark's secondary landed 1.8 L\*
/// from its primary, which is at the edge of being noticeable at all.
///
/// Signed, not `abs()`. A rung that is already `min_delta` away on the
/// *wrong* side is an inverted ramp, and an absolute-distance test would
/// hand it back as if it were fine — the one input where this function is
/// the last thing standing between a user theme and a nonsense hierarchy.
fn separate_above(from: &str, below: &str, toward: &str, min_delta: f64) -> String {
    let (Some(anchor), Some(target)) = (lstar(below), lstar(toward)) else {
        return from.to_string();
    };
    // `toward` is pure white or black, so its own lightness gives the
    // direction the ramp climbs in this mode.
    let sign = if target >= anchor { 1.0 } else { -1.0 };
    for step in 0..=200 {
        let candidate = mix_hex(from, toward, step as f64 / 200.0);
        if lstar(&candidate).is_some_and(|l| (l - anchor) * sign >= min_delta) {
            return candidate;
        }
    }
    // Unreachable for a parseable palette: `toward` is an extreme, so the
    // last step is 100 L* from anything. Fall back to the input.
    from.to_string()
}

/// Where the text ramp *starts*, before the two guarantees below are
/// applied: contrast against the page, which is what gives each rung its
/// place in the hierarchy.
///
/// `primary` is only raised when a theme's own foreground falls below its
/// floor — 24 of the 31 bundled themes keep theirs verbatim. The 7 that
/// change (one-dark, catppuccin-latte, solarized dark/light,
/// rose-pine-dawn, everforest-light, ayu-light) are low-contrast-by-design
/// palettes, and this trades some of that identity for legibility. In
/// solarized-dark's case it also fixes an inverted ramp: its foreground
/// was 4.75:1 while its `bright_black` was 2.79:1, so secondary was
/// *darker* than primary.
///
/// `FLOOR_TEXT_SECONDARY` is dominated by [`FLOOR_TEXT_AA`] plus
/// [`MIN_TEXT_STEP_LSTAR`] in all 31 bundled themes — every one of them
/// ends up above 6.0:1 on the page anyway. It is kept because it anchors
/// the rung to the page independently of the elevation ramp: change how
/// far `bg_toolbar` sits from the page and this is what still holds.
const FLOOR_TEXT_PRIMARY: f64 = 7.5;
const FLOOR_TEXT_SECONDARY: f64 = 6.0;
/// WCAG AA for normal text, with a hair of margin over the 4.5:1 the audit
/// enforces, applied on the worst surface each rung is drawn on.
///
/// This is the floor that actually binds. Solved against the page alone,
/// `text_secondary` measured 3.88:1 on the toolbar and `text_muted`
/// 3.60:1 on a panel — both below AA on surfaces where they really render.
const FLOOR_TEXT_AA: f64 = 4.6;
/// Minimum perceptual distance between adjacent text rungs, in L\*.
///
/// Set at the bottom of the range the ramp used to reach on its own
/// (secondary→muted was 6.9–10.4 before the AA pass existed) — so the
/// hierarchy is now *guaranteed* where it used to be emergent, but it is
/// also, honestly, a little tighter than before: raising muted to AA
/// pulls it up toward secondary, and 28 of the 31 bundled themes now land
/// at a secondary→muted gap of 6.0–7.4 with nine sitting exactly on this
/// floor. That is the trade taken deliberately — legibility on every
/// surface over a wider ramp — and 6.0 L\* is still 3–6× the
/// just-noticeable difference, so the three rungs remain plainly distinct.
const MIN_TEXT_STEP_LSTAR: f64 = 6.0;
/// A safety floor for user themes, below which a divider is simply not
/// there — see [`contrast_floor`]. The derivation targets the toolbar and
/// lands well above it.
const FLOOR_BORDER: f64 = 2.2;
/// Control outlines take 1.4.11's 3:1, with margin.
const FLOOR_BORDER_STRONG: f64 = 3.2;

/// Derive semantic UI colors from the 18 base colors.
fn derive_semantic_colors(colors: &ThemeColors, is_dark: bool) -> DerivedColors {
    // Elevation ramp. The previous steps (dark +5/+8 %, light −3/−5 %) sat
    // within ~12 luminance points of the page, so panels melted into one
    // field. Widen the ramp so page / panel / toolbar read as distinct
    // surfaces in both modes; borders and shadows then only refine the
    // separation luminance already provides.
    let bg_secondary = if is_dark {
        lighten_hex(&colors.background, 0.08)
    } else {
        darken_hex(&colors.background, 0.05)
    };

    let bg_toolbar = if is_dark {
        lighten_hex(&colors.background, 0.14)
    } else {
        darken_hex(&colors.background, 0.10)
    };

    // Text ramp, solved in three passes rather than taken verbatim:
    // place each rung against the page, raise it to clear WCAG AA on the
    // worst surface it is actually drawn on, then re-separate the rungs
    // that the second pass pulled together.
    //
    // `text_secondary` used to be `bright_black` as-is, and `text_muted`
    // that blended 22 % toward the page. An 18-slot ANSI palette has no
    // mid-neutral — it jumps from `foreground` straight to `bright_black` —
    // so on a dozen bundled palettes that landed far below readable
    // (nord 1.69:1, gruvbox-dark 1.67:1). Solving toward the edge of the
    // range instead keeps each rung on the palette's own hue line while
    // guaranteeing the floor.
    //
    // The blend target is pure white or black, not the palette's
    // `bright_white` / `black` slots: those are arbitrary ANSI colours
    // (Catppuccin Latte's `black` is #5c5f77, *lighter* than its
    // foreground), so blending toward them can move away from the edge.
    let extreme = if is_dark { "#ffffff" } else { "#000000" };
    let bg = &colors.background;

    // ── 1. Hierarchy: solve each rung against the page ───────────────
    let text_primary = blend_to_contrast(&colors.foreground, extreme, bg, FLOOR_TEXT_PRIMARY);
    // `primary` starts from the foreground, so the extreme is its only
    // anchor. `secondary` prefers the foreground and reaches for the
    // extreme only when that cannot get it to the floor.
    let text_secondary = blend_preferring(
        &colors.bright_black,
        &colors.foreground,
        extreme,
        bg,
        FLOOR_TEXT_SECONDARY,
    );

    // ── 2. Legibility: clear AA on the worst surface each rung is
    //       drawn on, not just on the page ─────────────────────────────
    //
    // Solved for the page alone, `secondary` rendered at 3.88:1 on the
    // toolbar in the dark themes — and the toolbar, status bar, tab bar
    // and context menus are full of it. `muted` never reaches the
    // toolbar (see `audit_surfaces`), so the panel is its worst surface.
    let text_secondary = blend_preferring(
        &text_secondary,
        &colors.foreground,
        extreme,
        &bg_toolbar,
        FLOOR_TEXT_AA,
    );
    // Step *down* from secondary, so muted reads dimmer rather than
    // landing on the same value in themes whose `bright_black` already
    // clears the higher floor. Dimmed toward the page, measured on the
    // panel.
    let text_muted = dim_to_contrast(&text_secondary, bg, &bg_secondary, FLOOR_TEXT_AA);

    // ── 3. Hierarchy again: the AA pass compresses the ramp ───────────
    //
    // Raising the lower rungs to clear AA pulls them toward the ones
    // above. Unchecked, one-dark's secondary landed 1.8 L* under its
    // primary. Bottom-up, so raising a rung never eats the gap below it.
    let text_secondary = separate_above(&text_secondary, &text_muted, extreme, MIN_TEXT_STEP_LSTAR);
    let text_primary = blend_to_contrast(&text_primary, extreme, &bg_toolbar, FLOOR_TEXT_AA);
    let text_primary = separate_above(&text_primary, &text_secondary, extreme, MIN_TEXT_STEP_LSTAR);

    // Solved against `bg_toolbar`, the most elevated surface these lines
    // are drawn on, rather than against the page. Solved against
    // `bg_primary` alone, an opaque border measured 2.2:1 there and as
    // little as 1.42:1 on the toolbar — below the floor on a surface where
    // plenty of these dividers actually live, and on 11 themes worse than
    // the translucent version it replaced.
    // `blend_preferring`, not `blend_to_contrast`: mixing the page toward
    // the foreground caps at the foreground's own contrast, and on
    // solarized-dark (fg 4.75:1) that cannot reach the control floor
    // against the toolbar — `blend_to_contrast` would silently hand back
    // the background itself, an invisible border reported as derived.
    let border = blend_preferring(bg, &colors.foreground, extreme, &bg_toolbar, FLOOR_BORDER);
    let border_strong = blend_preferring(
        bg,
        &colors.foreground,
        extreme,
        &bg_toolbar,
        FLOOR_BORDER_STRONG,
    );

    DerivedColors {
        bg_primary: colors.background.clone(),
        bg_secondary,
        bg_toolbar,
        text_primary,
        text_secondary,
        text_muted,
        accent_blue: colors.blue.clone(),
        accent_green: colors.green.clone(),
        accent_orange: colors.yellow.clone(),
        accent_purple: colors.magenta.clone(),
        accent_red: colors.red.clone(),
        accent_primary: colors.blue.clone(),
        accent_secondary: colors.magenta.clone(),
        accent_tertiary: colors.green.clone(),
        // Solved outward from the page rather than `bright_black` at 50 %
        // alpha, which composited to as little as 1.27:1 (gruvbox-dark) —
        // an invisible line. Opaque, so the audited ratio is the ratio the
        // eye sees rather than one that depends on what it overlays.
        border,
        border_strong,
        selection: with_alpha(&colors.blue, "40"),
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
fn derive_editor(colors: &ThemeColors, derived: &DerivedColors, is_dark: bool) -> ThemeEditor {
    // Diff backgrounds follow the THEME's green/red over its own
    // background — fixed GitHub constants here made every non-GitHub
    // theme's diffs clash with their surfaces (most visible on warm
    // palettes). Brand themes that need exact values pin them via a
    // partial `[editor]` override.
    let blend = if is_dark { 0.18 } else { 0.16 };
    let added_bg = mix_hex(&derived.bg_primary, &derived.accent_green, blend);
    let removed_bg = mix_hex(&derived.bg_primary, &derived.accent_red, blend);

    // Derive syntax colors spreading the theme's own ANSI palette:
    // keyword/operator → red, string → green, function → purple,
    // type → blue, number → yellow/orange, property → cyan,
    // comment → text-secondary. The old mapping leaned everything on
    // blue (strings were lightened blue, numbers blue, property green),
    // which gave non-blue themes a dissonant cool cast in code views.
    let kw = &derived.accent_red;
    let str_c = derived.accent_green.clone();
    let func = &derived.accent_purple;
    let typ = &derived.accent_blue;
    let num = &derived.accent_orange;
    let prop = &colors.cyan;

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
        syntax_number: Some(num.clone()),
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
    #[serde(alias = "line-highlight")]
    line_highlight: Option<String>,
    #[serde(alias = "gutter-bg")]
    gutter_bg: Option<String>,
    #[serde(alias = "gutter-fg")]
    gutter_fg: Option<String>,
    #[serde(alias = "added-bg")]
    added_bg: Option<String>,
    #[serde(alias = "removed-bg")]
    removed_bg: Option<String>,
    #[serde(alias = "added-text")]
    added_text: Option<String>,
    #[serde(alias = "removed-text")]
    removed_text: Option<String>,
    #[serde(alias = "syntax-keyword")]
    syntax_keyword: Option<String>,
    #[serde(alias = "syntax-string")]
    syntax_string: Option<String>,
    #[serde(alias = "syntax-comment")]
    syntax_comment: Option<String>,
    #[serde(alias = "syntax-function")]
    syntax_function: Option<String>,
    #[serde(alias = "syntax-type")]
    syntax_type: Option<String>,
    #[serde(alias = "syntax-number")]
    syntax_number: Option<String>,
    #[serde(alias = "syntax-operator")]
    syntax_operator: Option<String>,
    #[serde(alias = "syntax-property")]
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
    derived: Option<RawDerivedOverride>,
    accents: Option<ThemeAccents>,
}

/// The `[derived]` section — per-theme overrides for the semantic UI
/// tokens that `derive_semantic_colors` computes from the base palette.
///
/// **Note the policy this reflects, which changed.** The contrast floors
/// are now enforced in `derive_semantic_colors` itself, so a user theme
/// declaring only `[colors]` *does* get its derived text and border tokens
/// adjusted. That is deliberate: those tokens were never authored — a
/// theme file declares 18 ANSI colours — and the audit can otherwise only
/// warn about a palette it is not allowed to help.
///
/// What is still never touched is a value the author wrote down. This
/// section merges after the derivation, so an explicit pin always wins,
/// including one that fails its floor: failing pins are reported, never
/// corrected.
///
/// Raising `colors.bright_black` would not have worked as a fix — it also
/// feeds the terminal's ANSI palette, so a legible Nord would stop being
/// Nord in the terminal.
#[derive(Debug, Clone, Default, Deserialize)]
struct RawDerivedOverride {
    #[serde(default, alias = "text-primary")]
    text_primary: Option<String>,
    #[serde(default, alias = "text-secondary")]
    text_secondary: Option<String>,
    #[serde(default, alias = "text-muted")]
    text_muted: Option<String>,
    #[serde(default)]
    border: Option<String>,
    /// Audited like `border`, so it has to be pinnable too — otherwise a
    /// user whose control outlines are reported as failing has no
    /// supported way to fix them.
    #[serde(default, alias = "border-strong")]
    border_strong: Option<String>,
}

/// Apply partial overrides from a `[derived]` section onto the computed
/// `DerivedColors`.
///
/// A pin always wins. The one piece of derivation that survives is
/// `text_muted` when the theme pins `text_secondary` without pinning
/// muted: muted is defined as a step down from secondary, and leaving it
/// computed from the value the pin just discarded breaks that. Harmless
/// with the four bundled pins, which are all brighter than the derived
/// value — but a theme pinning a *dimmer* secondary would invert the ramp
/// with nothing to catch it, since the audit only checks floors.
fn merge_derived_overrides(base: &mut DerivedColors, overrides: RawDerivedOverride) {
    if let Some(v) = overrides.text_primary {
        base.text_primary = v;
    }
    let pinned_secondary = overrides.text_secondary.is_some();
    if let Some(v) = overrides.text_secondary {
        base.text_secondary = v;
    }
    if let Some(v) = overrides.text_muted {
        base.text_muted = v;
    } else if pinned_secondary {
        base.text_muted = dim_to_contrast(
            &base.text_secondary,
            &base.bg_primary,
            &base.bg_secondary,
            FLOOR_TEXT_AA,
        );
    }
    if let Some(v) = overrides.border {
        base.border = v;
    }
    if let Some(v) = overrides.border_strong {
        base.border_strong = v;
    }
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
    // Last, so an explicit `[derived]` value wins over both the palette
    // derivation and the accent overrides.
    if let Some(overrides) = raw.derived {
        for (field, value) in [
            ("derived.text_primary", overrides.text_primary.as_deref()),
            (
                "derived.text_secondary",
                overrides.text_secondary.as_deref(),
            ),
            ("derived.text_muted", overrides.text_muted.as_deref()),
            ("derived.border", overrides.border.as_deref()),
            ("derived.border_strong", overrides.border_strong.as_deref()),
        ] {
            if let Some(v) = value {
                validate_color(field, v)?;
            }
        }
        merge_derived_overrides(&mut derived, overrides);
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
        ROSE_PINE_MOON_TOML,
        ROSE_PINE_DAWN_TOML,
        EVERFOREST_DARK_TOML,
        EVERFOREST_LIGHT_TOML,
        KANAGAWA_TOML,
        AYU_DARK_TOML,
        AYU_MIRAGE_TOML,
        AYU_LIGHT_TOML,
        MATERIAL_TOML,
        ZENBURN_TOML,
        OXOCARBON_TOML,
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
        assert_eq!(themes.len(), 31);
    }

    // ── WCAG contrast ─────────────────────────────────────────────────────

    #[test]
    fn test_contrast_ratio_known_extremes() {
        // The two endpoints of the WCAG scale, to catch a wrong luminance
        // coefficient or a missing sRGB linearisation.
        let black_on_white = contrast_ratio("#000000", "#ffffff").unwrap();
        assert!(
            (black_on_white - 21.0).abs() < 0.01,
            "expected 21:1, got {black_on_white}"
        );
        let same = contrast_ratio("#7f7f7f", "#7f7f7f").unwrap();
        assert!((same - 1.0).abs() < 0.001, "expected 1:1, got {same}");
    }

    #[test]
    fn test_contrast_ratio_is_order_independent() {
        let a = contrast_ratio("#1e1e1e", "#d4d4d4").unwrap();
        let b = contrast_ratio("#d4d4d4", "#1e1e1e").unwrap();
        assert!((a - b).abs() < f64::EPSILON);
    }

    #[test]
    fn test_contrast_ratio_matches_a_published_value() {
        // #767676 on white is the canonical "exactly AA for normal text"
        // example — 4.54:1. A naive (non-linearised) implementation gets
        // this visibly wrong, so it pins the gamma step specifically.
        let ratio = contrast_ratio("#767676", "#ffffff").unwrap();
        assert!(
            (4.5..4.6).contains(&ratio),
            "expected ~4.54:1 for #767676 on white, got {ratio}"
        );
    }

    #[test]
    fn test_contrast_ratio_rejects_non_hex() {
        // `rgba(…)` is accepted by `validate_color` but carries alpha over
        // an unknown backdrop, so there is no single ratio to report. It is
        // surfaced as `unaudited` rather than silently skipped.
        assert!(contrast_ratio("rgba(0,0,0,0.5)", "#ffffff").is_none());
        assert!(contrast_ratio("", "#000000").is_none());
        assert!(contrast_ratio("#12345", "#000000").is_none());
    }

    #[test]
    fn test_contrast_ratio_expands_three_digit_hex() {
        // `#fff` is legal CSS and a user theme may use it. Failing to parse
        // it would mean silently not auditing the token.
        let short = contrast_ratio("#fff", "#000").unwrap();
        let long = contrast_ratio("#ffffff", "#000000").unwrap();
        assert!((short - long).abs() < 1e-9, "{short} vs {long}");
        assert!((short - 21.0).abs() < 0.01);
    }

    #[test]
    fn test_unparseable_token_is_reported_not_dropped() {
        // A silent pass is the one outcome an accessibility check must never
        // produce: before this, an `rgba()` token made the theme look clean.
        let mut theme = builtin("beardgit-dark");
        theme.derived.text_secondary = "rgba(255, 255, 255, 0.2)".to_string();

        let report = check_theme_contrast(&theme);

        assert!(!report.passes());
        assert_eq!(report.unaudited, vec!["text_secondary".to_string()]);
        assert!(report.warnings.is_empty(), "no ratio means no warning");
    }

    #[test]
    fn test_contrast_ratio_accepts_eight_digit_hex() {
        // `#RRGGBBAA` parses (alpha ignored) so callers passing a derived
        // token with alpha get a number rather than `None`.
        assert!(contrast_ratio("#000000ff", "#ffffff").is_some());
    }

    #[test]
    fn test_contrast_floor_only_covers_audited_tokens() {
        // All three text rungs take the WCAG AA normal-text floor. `muted`
        // used to sit at 3:1 on a large-text exemption it never qualified
        // for — it renders at 10px.
        assert_eq!(contrast_floor("text_primary"), Some(4.5));
        assert_eq!(contrast_floor("text_secondary"), Some(4.5));
        assert_eq!(contrast_floor("text_muted"), Some(4.5));
        // Control outlines identify a component, so they take WCAG
        // 1.4.11's 3:1; plain separators only refine the elevation ramp.
        assert_eq!(contrast_floor("border_strong"), Some(3.0));
        assert_eq!(contrast_floor("border"), Some(2.0));
        // `selection` overlays arbitrary content, so there is no single
        // pair to measure.
        assert_eq!(contrast_floor("selection"), None);
    }

    #[test]
    fn test_composite_over_flattens_alpha() {
        // 50 % black over white is mid-grey; a fully opaque value and a
        // non-hex value pass through untouched.
        assert_eq!(composite_over("#00000080", "#ffffff"), "#7f7f7f");
        assert_eq!(composite_over("#123456", "#ffffff"), "#123456");
        assert_eq!(
            composite_over("rgba(0,0,0,0.5)", "#ffffff"),
            "rgba(0,0,0,0.5)"
        );
    }

    #[test]
    fn test_blend_to_contrast_leaves_a_passing_colour_alone() {
        // A theme whose own colour is already legible keeps it verbatim —
        // the derivation is a floor, not a normaliser.
        assert_eq!(
            blend_to_contrast("#ffffff", "#ffffff", "#000000", 7.5),
            "#ffffff"
        );
    }

    #[test]
    fn test_blend_to_contrast_raises_a_failing_colour() {
        let raised = blend_to_contrast("#111111", "#ffffff", "#000000", 7.5);
        assert_ne!(raised, "#111111");
        assert!(contrast_ratio(&raised, "#000000").unwrap() >= 7.5);
    }

    #[test]
    fn test_dim_to_contrast_steps_below_its_input() {
        // `muted` has to read dimmer than `secondary` while staying above
        // its own floor, so this must move *and* stay legible.
        let secondary = "#98928a";
        let page = "#151312";
        let muted = dim_to_contrast(secondary, page, page, 4.6);
        let rs = contrast_ratio(secondary, page).unwrap();
        let rm = contrast_ratio(&muted, page).unwrap();
        assert!(rm >= 4.6, "muted fell below its floor: {rm}");
        assert!(rm < rs, "muted must be dimmer than secondary: {rm} vs {rs}");
    }

    #[test]
    fn test_dim_to_contrast_holds_the_floor_on_the_measured_surface() {
        // The regression this signature exists for: dimming toward the
        // page while measuring on the page leaves the colour below its
        // floor on the panel, which is where it actually renders.
        let secondary = "#98928a";
        let page = "#151312";
        let panel = "#2a2724";

        let page_only = dim_to_contrast(secondary, page, page, 4.6);
        assert!(
            contrast_ratio(&page_only, panel).unwrap() < 4.6,
            "the old behaviour has to be reproducible, or this guard is vacuous"
        );

        let panel_aware = dim_to_contrast(secondary, page, panel, 4.6);
        assert!(contrast_ratio(&panel_aware, panel).unwrap() >= 4.6);
        // Still dimmed, just not as far.
        assert_ne!(panel_aware, secondary);
    }

    #[test]
    fn test_separate_above_opens_a_gap_and_leaves_a_wide_one_alone() {
        let below = "#606060";
        let touching = separate_above("#666666", below, "#ffffff", 6.0);
        let gap = lstar(&touching).unwrap() - lstar(below).unwrap();
        assert!(gap >= 6.0, "gap not opened: {gap}");

        let already_apart = "#e0e0e0";
        assert_eq!(
            separate_above(already_apart, below, "#ffffff", 6.0),
            already_apart,
            "a rung already far enough away must be kept verbatim"
        );
    }

    #[test]
    fn test_separate_above_corrects_an_inverted_rung() {
        // Far enough away, but on the wrong side: darker than the rung it
        // is supposed to sit above. An absolute-distance check would call
        // this separated and hand back the inversion.
        let below = "#909090";
        let inverted = "#404040";
        assert!(
            (lstar(inverted).unwrap() - lstar(below).unwrap()).abs() >= 6.0,
            "the setup must already clear the gap in absolute terms, or this proves nothing"
        );

        let fixed = separate_above(inverted, below, "#ffffff", 6.0);
        assert!(
            lstar(&fixed).unwrap() - lstar(below).unwrap() >= 6.0,
            "must end up above, not merely far: {fixed}"
        );

        // Same, mirrored, for light mode: the ramp climbs toward black.
        let fixed_light = separate_above("#d0d0d0", below, "#000000", 6.0);
        assert!(
            lstar(below).unwrap() - lstar(&fixed_light).unwrap() >= 6.0,
            "light mode must separate downward: {fixed_light}"
        );
    }

    /// **Every bundled theme must be legible.** This is the audit the
    /// `derive_semantic_colors` comment already claimed existed ("verified
    /// per theme by the contrast check") before one did.
    ///
    /// A failure here means the derivation regressed, since it solves for
    /// these floors — so the fix is normally in `derive_semantic_colors`,
    /// not in a TOML. A `[derived]` pin is the right answer only when a
    /// theme's own palette has a slot that beats the derived value on
    /// *every* surface the token is drawn on, which three of them do. It
    /// can also be the cause: a pin wins over the derivation, so a pin
    /// chosen against the page alone can hold a token below AA on the
    /// toolbar — which is how gitlab-dark's lost its pin.
    #[test]
    fn test_all_builtin_themes_meet_contrast_floors() {
        let failures: Vec<String> = load_builtin_themes()
            .iter()
            .map(check_theme_contrast)
            .filter(|report| !report.passes())
            .flat_map(|report| {
                // Both vectors, not just `warnings`. `passes()` also requires
                // `unaudited` to be empty, so filtering on `!passes()` and
                // then only listing warnings would let a theme with an
                // unparseable colour produce zero failure strings — the
                // assertion below would hold and the guard would pass
                // vacuously on exactly the silent-pass case it exists for.
                let mut lines: Vec<String> = report
                    .warnings
                    .iter()
                    .map(|w| {
                        format!(
                            "{}: {} {} on {} = {:.2}:1 (needs {:.1}:1)",
                            report.theme_id,
                            w.token,
                            w.foreground,
                            w.background,
                            w.ratio,
                            w.required
                        )
                    })
                    .collect();
                lines.extend(report.unaudited.iter().map(|token| {
                    format!(
                        "{}: {token} could not be parsed as hex, so it was never measured",
                        report.theme_id
                    )
                }));
                lines
            })
            .collect();

        assert!(
            failures.is_empty(),
            "{} bundled theme token(s) below the contrast floor:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }

    #[test]
    fn test_check_theme_contrast_flags_a_deliberately_bad_theme() {
        // Guards the audit itself: a `check_theme_contrast` that always
        // returned an empty report would make the test above vacuous.
        let mut theme = builtin("beardgit-dark");
        theme.derived.text_secondary = theme.derived.bg_primary.clone();

        let report = check_theme_contrast(&theme);

        assert!(!report.passes());
        let warning = report
            .warnings
            .iter()
            .find(|w| w.token == "text_secondary")
            .expect("text_secondary must be flagged");
        assert!(
            (warning.ratio - 1.0).abs() < 0.01,
            "identical colors are 1:1, got {}",
            warning.ratio
        );
        assert_eq!(warning.required, 4.5);
    }

    /// The whole point of the surface axis: a token can clear its floor on
    /// the page and still be illegible where it is drawn.
    #[test]
    fn test_a_token_that_only_fails_off_the_page_is_flagged() {
        let mut theme = builtin("beardgit-dark");
        // Solve secondary to exactly AA against the page, which leaves it
        // short on the two elevated surfaces.
        theme.derived.text_secondary = dim_to_contrast(
            &theme.derived.text_primary,
            &theme.derived.bg_primary,
            &theme.derived.bg_primary,
            4.5,
        );

        assert!(
            contrast_ratio(&theme.derived.text_secondary, &theme.derived.bg_primary).unwrap()
                >= 4.5,
            "the setup must pass on the page, or this proves nothing"
        );

        let warning = check_theme_contrast(&theme)
            .warnings
            .into_iter()
            .find(|w| w.token == "text_secondary")
            .expect("a token below AA on the toolbar must be flagged");
        assert_eq!(
            warning.background, theme.derived.bg_toolbar,
            "the report must name the surface that actually failed"
        );
    }

    /// `text_muted` is audited on the page and the panel but not the
    /// toolbar. That set is a claim about CSS, so pin it: a change to
    /// `audit_surfaces` should have to be deliberate.
    #[test]
    fn test_audit_surface_sets_match_where_tokens_are_drawn() {
        let d = &builtin("beardgit-dark").derived;
        assert_eq!(
            audit_surfaces("text_muted", d),
            vec![&d.bg_primary, &d.bg_secondary],
            "no --text-muted callsite sits on a toolbar-rooted component"
        );
        for token in ["text_primary", "text_secondary", "border", "border_strong"] {
            assert_eq!(
                audit_surfaces(token, d),
                vec![&d.bg_primary, &d.bg_secondary, &d.bg_toolbar],
                "{token} is drawn on every surface"
            );
        }
    }

    /// The three rungs have to read as a hierarchy, not as three shades of
    /// the same grey. Contrast floors alone stopped guaranteeing this once
    /// the AA pass started raising the lower rungs.
    #[test]
    fn test_text_ramp_stays_perceptually_separated() {
        let failures: Vec<String> = load_builtin_themes()
            .iter()
            .filter_map(|t| {
                let d = &t.derived;
                let (p, s, m) = (
                    lstar(&d.text_primary)?,
                    lstar(&d.text_secondary)?,
                    lstar(&d.text_muted)?,
                );
                // Ordering as well as spacing: an inverted ramp would clear
                // an abs() gap check while reading as nonsense.
                let ordered = if t.meta.mode == "dark" {
                    p > s && s > m
                } else {
                    p < s && s < m
                };
                let ps = (p - s).abs();
                let sm = (s - m).abs();
                (!ordered || ps < MIN_TEXT_STEP_LSTAR || sm < MIN_TEXT_STEP_LSTAR).then(|| {
                    format!(
                        "{}: L* {p:.1}/{s:.1}/{m:.1} — gaps {ps:.1}, {sm:.1} (need {MIN_TEXT_STEP_LSTAR})",
                        t.meta.id
                    )
                })
            })
            .collect();

        assert!(
            failures.is_empty(),
            "{} theme(s) with a collapsed or inverted text ramp:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }

    /// A pinned `text_secondary` has to drag `text_muted` with it, or the
    /// documented "one step down from secondary" invariant is computed
    /// against a value the theme discarded.
    #[test]
    fn test_pinned_secondary_redirives_muted() {
        let base = builtin("beardgit-dark");
        // Dimmer than the derived value and off its hue line — the
        // direction the three bundled pins never exercise, and the one
        // that inverts the ramp when muted is left computed from the
        // pre-pin value.
        let dim_pin = "#8b93a8";
        assert!(
            lstar(dim_pin).unwrap() < lstar(&base.derived.text_secondary).unwrap(),
            "the pin has to actually be dimmer for this to test anything"
        );

        let mut derived = base.derived.clone();
        merge_derived_overrides(
            &mut derived,
            RawDerivedOverride {
                text_secondary: Some(dim_pin.into()),
                ..Default::default()
            },
        );

        assert_eq!(derived.text_secondary, dim_pin, "the pin must win");
        let s = lstar(&derived.text_secondary).unwrap();
        let m = lstar(&derived.text_muted).unwrap();
        // `<=`, not `<`: a pin already sitting on the AA floor for the
        // panel leaves nowhere dimmer to go, and the two rungs collapse
        // onto one value. That is the correct outcome — the alternative is
        // a muted rung below AA — and it is still a fix, because before
        // this muted was computed from the pre-pin value and came out
        // *brighter* than the secondary the theme actually asked for.
        assert!(
            m <= s,
            "muted must never end up brighter than the pinned secondary: {m} vs {s}"
        );
        assert_ne!(
            derived.text_muted, base.derived.text_muted,
            "muted has to follow the pin, not the value the pin replaced"
        );
        assert!(
            contrast_ratio(&derived.text_muted, &derived.bg_secondary).unwrap() >= 4.5,
            "the re-derived muted still has to clear AA on the panel"
        );
    }

    /// The same fix where the pin leaves room: muted lands strictly below.
    #[test]
    fn test_pinned_secondary_still_leaves_muted_a_step_down() {
        let base = builtin("beardgit-dark");
        // Brighter than derived, like the three bundled pins — muted has
        // room to step down and must actually use it.
        let bright_pin = "#e8e6e3";
        let mut derived = base.derived.clone();
        merge_derived_overrides(
            &mut derived,
            RawDerivedOverride {
                text_secondary: Some(bright_pin.into()),
                ..Default::default()
            },
        );

        let s = lstar(bright_pin).unwrap();
        let m = lstar(&derived.text_muted).unwrap();
        assert!(m < s, "muted must read dimmer than secondary: {m} vs {s}");
        assert!(contrast_ratio(&derived.text_muted, &derived.bg_secondary).unwrap() >= 4.5);
    }

    /// An explicit `text_muted` pin is never touched, even alongside a
    /// `text_secondary` pin.
    #[test]
    fn test_pinned_muted_wins_over_the_rederivation() {
        let mut derived = builtin("beardgit-dark").derived.clone();
        merge_derived_overrides(
            &mut derived,
            RawDerivedOverride {
                text_secondary: Some("#c0c0c0".into()),
                text_muted: Some("#909090".into()),
                ..Default::default()
            },
        );
        assert_eq!(derived.text_muted, "#909090");
    }

    // ── Serde contract with the TypeScript mirror ─────────────────────────
    //
    // `src/lib/types/index.ts` declares these shapes by hand and the
    // frontend reads the fields by name. `#[serde(rename = "…")]` changes
    // *serialization* as well as deserialization, so a kebab-case rename
    // here silently makes every TS read `undefined` — no compile error, no
    // runtime error, just tokens that never apply. `#[serde(alias)]` is the
    // correct attribute for accepting kebab-case TOML input.

    /// Fetch one built-in theme by id.
    fn builtin(id: &str) -> Theme {
        load_builtin_themes()
            .into_iter()
            .find(|t| t.meta.id == id)
            .unwrap_or_else(|| panic!("built-in theme `{id}` not found"))
    }

    /// Serialize a theme and return the sorted keys of one section.
    fn serialized_keys(theme: &Theme, section: &str) -> Vec<String> {
        let value = serde_json::to_value(theme).expect("theme must serialize");
        let mut keys: Vec<String> = value
            .get(section)
            .unwrap_or_else(|| panic!("missing `{section}` section"))
            .as_object()
            .unwrap_or_else(|| panic!("`{section}` must be an object"))
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    }

    fn sorted(items: &[&str]) -> Vec<String> {
        let mut v: Vec<String> = items.iter().map(|s| (*s).to_string()).collect();
        v.sort();
        v
    }

    #[test]
    fn test_serialized_editor_keys_are_snake_case() {
        assert_eq!(
            serialized_keys(&builtin("beardgit-dark"), "editor"),
            sorted(&[
                "background",
                "foreground",
                "cursor",
                "selection",
                "line_highlight",
                "gutter_bg",
                "gutter_fg",
                "added_bg",
                "removed_bg",
                "added_text",
                "removed_text",
                "syntax_keyword",
                "syntax_string",
                "syntax_comment",
                "syntax_function",
                "syntax_type",
                "syntax_number",
                "syntax_operator",
                "syntax_property",
            ]),
            "the `editor` keys the frontend receives must match \
             `ThemeEditorData` in src/lib/types/index.ts exactly"
        );
    }

    #[test]
    fn test_serialized_graph_keys_are_snake_case() {
        assert_eq!(
            serialized_keys(&builtin("beardgit-dark"), "graph"),
            sorted(&[
                "lane_colors",
                "background",
                "foreground",
                "text_primary",
                "text_secondary",
                "text_sha",
                "selection",
                "head_lane_tint",
                "selection_highlight",
                "dim_opacity",
                "node_radius",
                "merge_radius",
                "ref_branch",
                "ref_remote",
                "ref_tag",
                "ref_head",
            ])
        );
    }

    #[test]
    fn test_no_serialized_key_is_kebab_case() {
        // Belt and braces across every section of every built-in theme: a
        // `rename` added to any future field fails here even if nobody
        // remembers to extend the two key lists above.
        for theme in load_builtin_themes() {
            let value = serde_json::to_value(&theme).expect("theme must serialize");
            for (section, contents) in value.as_object().expect("theme is an object") {
                let Some(fields) = contents.as_object() else {
                    continue;
                };
                for key in fields.keys() {
                    assert!(
                        !key.contains('-'),
                        "theme `{}` serializes `{section}.{key}` in kebab-case; \
                         use #[serde(alias = \"…\")] instead of rename so the \
                         TypeScript mirror keeps matching",
                        theme.meta.id
                    );
                }
            }
        }
    }

    #[test]
    fn test_editor_section_still_deserializes_kebab_case_toml() {
        // The bundled TOML files use kebab-case, and so do user themes in
        // `~/.config/beardgit/themes`. Switching rename→alias must not
        // break reading them.
        let toml = r##"
[meta]
id = "probe"
name = "Probe"
mode = "dark"

[colors]
background = "#000000"
foreground = "#ffffff"
black = "#000000"
red = "#ff0000"
green = "#00ff00"
yellow = "#ffff00"
blue = "#0000ff"
magenta = "#ff00ff"
cyan = "#00ffff"
white = "#ffffff"
bright_black = "#444444"
bright_red = "#ff4444"
bright_green = "#44ff44"
bright_yellow = "#ffff44"
bright_blue = "#4444ff"
bright_magenta = "#ff44ff"
bright_cyan = "#44ffff"
bright_white = "#ffffff"

[editor]
background = "#111111"
foreground = "#eeeeee"
cursor = "#ffffff"
selection = "#333333"
line-highlight = "#222222"
gutter-bg = "#101010"
gutter-fg = "#888888"
added-bg = "#0a2a0a"
removed-bg = "#2a0a0a"
added-text = "#44ff44"
removed-text = "#ff4444"
syntax-keyword = "#ff00ff"
"##;
        let theme = parse_theme(toml).expect("kebab-case [editor] must parse");
        let editor = theme.editor.expect("editor section present");
        assert_eq!(editor.added_bg, "#0a2a0a");
        assert_eq!(editor.removed_bg, "#2a0a0a");
        assert_eq!(editor.line_highlight, "#222222");
        assert_eq!(editor.gutter_fg, "#888888");
        assert_eq!(editor.syntax_keyword.as_deref(), Some("#ff00ff"));
    }

    // ── The fixture the frontend test suite consumes ──────────────────────

    /// Every generated theme fixture the frontend consumes: output path and
    /// the theme ids it holds.
    ///
    /// One generator, several outputs. Before this existed, each consumer
    /// hand-dumped its own copy — `theme.test.ts` wrote snake_case by hand
    /// (so it tested the TS type against itself) while
    /// `tests/visual/fixtures/theme-data.ts` held a kebab-case dump taken
    /// from the buggy serialization. Two hand-maintained copies of a shape
    /// is how the mismatch survived; generating them all from here is what
    /// makes it impossible to reintroduce in one place only.
    const FIXTURES: &[(&str, &[&str])] = &[
        // Unit tests for `applyTheme`'s token mapping.
        //
        // `beardgit-light` is the light-mode default, where the bug showed
        // up as dark diff backgrounds. It distinguishes a working read
        // from a broken one on one field only (`syntax_property`, which
        // `derive_editor` maps to `colors.cyan` while the frontend falls
        // back to `accent_blue`). `github-light` is the only light theme
        // shipping a pinned `[editor]` block and differs on four
        // (`syntax_string`, `_type`, `_number`, `_property`), which is
        // what gives the syntax assertions real signal.
        //
        // `beardgit-dark` is here so `src/test/fixtures/theme.ts` can build
        // the visual suite's theme pair from generated data. That file used
        // to hand-mirror the derivation, which meant every screenshot
        // baseline was rendered against a copy that drifted the moment the
        // derivation changed — and so could never catch a theme regression.
        (
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../src/lib/stores/__fixtures__/themes.json"
            ),
            &["beardgit-dark", "beardgit-light", "github-light"],
        ),
        // Playwright marketing screenshots (`marketing.spec.ts` feeds these
        // to `resolve_startup_theme` / `get_theme`).
        (
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/visual/fixtures/theme-data.json"
            ),
            &[
                "beardgit-dark",
                "beardgit-light",
                "fjord-dark",
                "fjord-light",
                "nebula-dark",
                "nebula-light",
            ],
        ),
    ];

    fn fixture_value(ids: &[&str]) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for id in ids {
            map.insert(
                (*id).to_string(),
                serde_json::to_value(builtin(id)).expect("theme must serialize"),
            );
        }
        serde_json::Value::Object(map)
    }

    /// The fixtures the frontend consumes must be byte-for-byte what this
    /// crate actually serializes.
    ///
    /// This is the cross-language half of the contract, and the reason the
    /// original bug was invisible: with hand-written fixtures, both test
    /// suites agreed with the TypeScript types by construction and neither
    /// ever saw what serde emitted.
    #[test]
    fn test_theme_fixtures_match_live_serialization() {
        for (path, ids) in FIXTURES {
            let committed: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(path)
                    .unwrap_or_else(|e| panic!("fixture {path} must exist: {e}")),
            )
            .unwrap_or_else(|e| panic!("fixture {path} must be valid JSON: {e}"));

            let live = fixture_value(ids);

            // Report the key-set difference before the value comparison:
            // `assert_eq!` on two multi-kilobyte `Value`s prints two
            // unreadable single-line dumps, and a kebab/snake mismatch is
            // exactly the case where you want to see the key names.
            if committed != live {
                let keys = |v: &serde_json::Value| -> Vec<String> {
                    v.as_object()
                        .map(|themes| {
                            let mut out: Vec<String> = themes
                                .iter()
                                .flat_map(|(id, theme)| {
                                    theme.as_object().into_iter().flat_map(move |sections| {
                                        sections.iter().flat_map(move |(section, fields)| {
                                            fields.as_object().into_iter().flat_map(
                                                move |f| -> Vec<String> {
                                                    f.keys()
                                                        .map(|k| format!("{id}.{section}.{k}"))
                                                        .collect()
                                                },
                                            )
                                        })
                                    })
                                })
                                .collect();
                            out.sort();
                            out
                        })
                        .unwrap_or_default()
                };
                let committed_keys = keys(&committed);
                let live_keys = keys(&live);
                let only_committed: Vec<_> = committed_keys
                    .iter()
                    .filter(|k| !live_keys.contains(k))
                    .collect();
                let only_live: Vec<_> = live_keys
                    .iter()
                    .filter(|k| !committed_keys.contains(k))
                    .collect();

                panic!(
                    "{path} is stale.\n\
                     keys only in the committed fixture: {only_committed:?}\n\
                     keys only in live serialization:    {only_live:?}\n\
                     (empty on both sides means the keys match and only \
                     values differ)\n\
                     Regenerate with: \
                     cargo test -p storage regenerate_theme_fixtures -- --ignored"
                );
            }
        }
    }

    /// Rewrite the committed fixtures. Ignored by default; run explicitly
    /// after an intentional shape change.
    #[test]
    #[ignore = "writes fixture files; run explicitly after a shape change"]
    fn regenerate_theme_fixtures() {
        for (path, ids) in FIXTURES {
            let json = serde_json::to_string_pretty(&fixture_value(ids)).unwrap();
            std::fs::write(path, format!("{json}\n")).expect("fixture must be writable");
            println!("rewrote {path}");
        }
    }

    /// Every `complementary` must point at a real bundled theme, point
    /// back, and cross modes. Structural only — it does not exercise
    /// `resolve_theme_for_mode` itself, which has its own tests.
    ///
    /// `resolve_theme_for_mode` follows this link when the user has
    /// follow-system-theme on. A one-way link means switching to dark
    /// finds the pair but switching back does not, so the app appears to
    /// get stuck on one variant.
    #[test]
    fn test_complementary_links_are_symmetric_and_reference_real_themes() {
        let themes = load_builtin_themes();
        let by_id: std::collections::HashMap<&str, &Theme> =
            themes.iter().map(|t| (t.meta.id.as_str(), t)).collect();

        let mut problems = Vec::new();
        for theme in &themes {
            let Some(comp_id) = theme.meta.complementary.as_deref() else {
                continue;
            };
            let Some(other) = by_id.get(comp_id) else {
                problems.push(format!(
                    "{} → `{comp_id}`, which is not a bundled theme",
                    theme.meta.id
                ));
                continue;
            };
            if other.meta.complementary.as_deref() != Some(theme.meta.id.as_str()) {
                problems.push(format!(
                    "{} → {comp_id}, but {comp_id} → {:?} (must point back)",
                    theme.meta.id, other.meta.complementary
                ));
            }
            if other.meta.mode == theme.meta.mode {
                problems.push(format!(
                    "{} and {comp_id} are both `{}` — a complement must be the other mode",
                    theme.meta.id, theme.meta.mode
                ));
            }
        }

        assert!(
            problems.is_empty(),
            "{} broken complementary link(s):\n  {}",
            problems.len(),
            problems.join("\n  ")
        );
    }

    #[test]
    fn test_builtin_themes_have_correct_modes() {
        let themes = load_builtin_themes();
        let dark_count = themes.iter().filter(|t| t.meta.mode == "dark").count();
        let light_count = themes.iter().filter(|t| t.meta.mode == "light").count();
        assert_eq!(dark_count, 21);
        assert_eq!(light_count, 10);
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
        // Lighter blend factor than dark mode over the same palette.
        assert_eq!(ed.added_bg, "#0e370e");
        assert_eq!(ed.removed_bg, "#370e0e");
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
        // Blended from the theme's own bg (#111111) toward green/red at 18%.
        let theme = parse_theme(MINIMAL_THEME).unwrap();
        let ed = theme.editor.unwrap();
        assert_eq!(ed.added_bg, "#0d3b0d");
        assert_eq!(ed.removed_bg, "#3b0d0d");
    }

    #[test]
    fn test_derive_editor_light_diff_colors() {
        // Light mode blends at 16% over the same base palette.
        let toml = MINIMAL_THEME.replace("mode = \"dark\"", "mode = \"light\"");
        let theme = parse_theme(&toml).unwrap();
        let ed = theme.editor.unwrap();
        assert_eq!(ed.added_bg, "#0e370e");
        assert_eq!(ed.removed_bg, "#370e0e");
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
        // string = derived.accent_green
        assert_eq!(ed.syntax_string, Some(theme.derived.accent_green.clone()));
        // number = derived.accent_orange (theme yellow)
        assert_eq!(ed.syntax_number, Some(theme.derived.accent_orange.clone()));
        // property = base cyan
        assert_eq!(ed.syntax_property, Some(theme.colors.cyan.clone()));
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
        // Dark-mode added-bg still derived (bg blended toward green)
        assert_eq!(ed.added_bg, "#0d3b0d");
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
