/**
 * Theme fixtures.
 *
 * `ThemeData` is what `resolve_startup_theme` / `get_theme` return —
 * the Rust side derives `derived`, `graph`, and `editor` sections from
 * the base 16 colours; we hardcode the BeardGit Dark / Light values
 * (the app's default theme pair) so the visual suite has a stable
 * baseline that doesn't depend on the Rust theme pipeline running.
 * The values mirror `crates/storage/src/theme.rs` derivation applied
 * to `themes/beardgit_dark.toml` / `themes/beardgit_light.toml`.
 *
 * If you tweak these values, regenerate baselines with
 * `npm run test:visual:update`.
 */

import type {
  DerivedColors,
  ThemeBaseColors,
  ThemeData,
  ThemeEditorData,
  ThemeGraphData,
  ThemeMeta,
} from "../../lib/types";

// ─── Dark (beardgit-dark) ────────────────────────────────────────────

const DARK_META: ThemeMeta = {
  id: "beardgit-dark",
  name: "BeardGit Dark",
  mode: "dark",
  complementary: "beardgit-light",
};

const DARK_BASE: ThemeBaseColors = {
  background: "#15110d",
  foreground: "#ede5d8",
  black: "#4a4238",
  red: "#e0604c",
  green: "#8fbc62",
  yellow: "#e0a458",
  blue: "#6fb1cc",
  magenta: "#c78bb0",
  cyan: "#7cc7b8",
  white: "#cbc1b2",
  bright_black: "#9a8f7d",
  bright_red: "#f08573",
  bright_green: "#abd084",
  bright_yellow: "#f0bd72",
  bright_blue: "#92c9e0",
  bright_magenta: "#dcabc9",
  bright_cyan: "#9cd9cc",
  bright_white: "#f7f1e6",
};

const DARK_DERIVED: DerivedColors = {
  bg_primary: "#15110d",
  bg_secondary: "#201c19",
  bg_toolbar: "#272420",
  text_primary: "#ede5d8",
  text_secondary: "#9a8f7d",
  accent_blue: "#6fb1cc",
  accent_green: "#8fbc62",
  accent_orange: "#e0a458",
  accent_purple: "#c78bb0",
  accent_red: "#e0604c",
  accent_primary: "#d9924f",
  accent_secondary: "#7cc7b8",
  accent_tertiary: "#8fbc62",
  border: "#9a8f7d80",
  selection: "#6fb1cc33",
};

const DARK_GRAPH: ThemeGraphData = {
  lane_colors: [
    "#6fb1cc",
    "#8fbc62",
    "#e0a458",
    "#c78bb0",
    "#e0604c",
    "#93c4d8",
  ],
  background: "#15110d",
  foreground: "#ede5d8",
  text_primary: "#ede5d8",
  text_secondary: "#9a8f7d",
  text_sha: "#6fb1cc",
  selection: "#6fb1cc44",
  head_lane_tint: "#6fb1cc22",
  selection_highlight: "#6fb1cc66",
  dim_opacity: 0.4,
  node_radius: 4,
  merge_radius: 3,
  ref_branch: "#8fbc62",
  ref_remote: "#6fb1cc",
  ref_tag: "#e0a458",
  ref_head: "#c78bb0",
};

const DARK_EDITOR: ThemeEditorData = {
  background: "#15110d",
  foreground: "#ede5d8",
  cursor: "#6fb1cc",
  selection: "#6fb1cc44",
  line_highlight: "#201c1966",
  gutter_bg: "#15110d",
  gutter_fg: "#9a8f7d",
  added_bg: "#1b3829",
  removed_bg: "#3c1e22",
  added_text: "#8fbc62",
  removed_text: "#e0604c",
  syntax_keyword: "#e0604c",
  syntax_string: "#a1ccdd",
  syntax_comment: "#9a8f7d",
  syntax_function: "#c78bb0",
  syntax_type: "#6fb1cc",
  syntax_number: "#6fb1cc",
  syntax_operator: "#e0604c",
  syntax_property: "#8fbc62",
};

// ─── Light (beardgit-light) ──────────────────────────────────────────

const LIGHT_META: ThemeMeta = {
  id: "beardgit-light",
  name: "BeardGit Light",
  mode: "light",
  complementary: "beardgit-dark",
};

const LIGHT_BASE: ThemeBaseColors = {
  background: "#faf5ee",
  foreground: "#3b332a",
  black: "#56473c",
  red: "#bf4a36",
  green: "#5f7d3a",
  yellow: "#aa7430",
  blue: "#37749a",
  magenta: "#96587f",
  cyan: "#3a8579",
  white: "#a39782",
  bright_black: "#6f6453",
  bright_red: "#a23a28",
  bright_green: "#4c682c",
  bright_yellow: "#8f5f1f",
  bright_blue: "#2a607f",
  bright_magenta: "#7d4668",
  bright_cyan: "#2c6f64",
  bright_white: "#fffdf8",
};

const LIGHT_DERIVED: DerivedColors = {
  bg_primary: "#faf5ee",
  bg_secondary: "#f2ede6",
  bg_toolbar: "#ede8e2",
  text_primary: "#3b332a",
  text_secondary: "#6f6453",
  accent_blue: "#37749a",
  accent_green: "#5f7d3a",
  accent_orange: "#aa7430",
  accent_purple: "#96587f",
  accent_red: "#bf4a36",
  accent_primary: "#b3702a",
  accent_secondary: "#3a8579",
  accent_tertiary: "#5f7d3a",
  border: "#6f645380",
  selection: "#37749a33",
};

const LIGHT_GRAPH: ThemeGraphData = {
  lane_colors: [
    "#37749a",
    "#5f7d3a",
    "#aa7430",
    "#96587f",
    "#bf4a36",
    "#6996b3",
  ],
  background: "#faf5ee",
  foreground: "#3b332a",
  text_primary: "#3b332a",
  text_secondary: "#6f6453",
  text_sha: "#37749a",
  selection: "#37749a44",
  head_lane_tint: "#37749a22",
  selection_highlight: "#37749a66",
  dim_opacity: 0.4,
  node_radius: 4,
  merge_radius: 3,
  ref_branch: "#5f7d3a",
  ref_remote: "#37749a",
  ref_tag: "#aa7430",
  ref_head: "#96587f",
};

const LIGHT_EDITOR: ThemeEditorData = {
  background: "#faf5ee",
  foreground: "#3b332a",
  cursor: "#37749a",
  selection: "#37749a44",
  line_highlight: "#f2ede666",
  gutter_bg: "#faf5ee",
  gutter_fg: "#6f6453",
  added_bg: "#d4f8db",
  removed_bg: "#fdd8d8",
  added_text: "#5f7d3a",
  removed_text: "#bf4a36",
  syntax_keyword: "#bf4a36",
  syntax_string: "#375b9a",
  syntax_comment: "#6f6453",
  syntax_function: "#96587f",
  syntax_type: "#37749a",
  syntax_number: "#37749a",
  syntax_operator: "#bf4a36",
  syntax_property: "#5f7d3a",
};

// ─── Public factories ─────────────────────────────────────────────────

export function makeDarkTheme(overrides: Partial<ThemeData> = {}): ThemeData {
  return {
    meta: DARK_META,
    colors: DARK_BASE,
    derived: DARK_DERIVED,
    graph: DARK_GRAPH,
    editor: DARK_EDITOR,
    ...overrides,
  };
}

export function makeLightTheme(overrides: Partial<ThemeData> = {}): ThemeData {
  return {
    meta: LIGHT_META,
    colors: LIGHT_BASE,
    derived: LIGHT_DERIVED,
    graph: LIGHT_GRAPH,
    editor: LIGHT_EDITOR,
    ...overrides,
  };
}

export function makeThemeData(
  mode: "dark" | "light" = "dark",
  overrides: Partial<ThemeData> = {},
): ThemeData {
  return mode === "dark" ? makeDarkTheme(overrides) : makeLightTheme(overrides);
}

export function makeThemeMetaList(): ThemeMeta[] {
  return [DARK_META, LIGHT_META];
}
