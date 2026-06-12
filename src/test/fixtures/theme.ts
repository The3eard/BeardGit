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
  background: "#151312",
  foreground: "#e8e4de",
  black: "#45403a",
  red: "#e0604c",
  green: "#8fbc62",
  yellow: "#e0a458",
  blue: "#6fb1cc",
  magenta: "#c78bb0",
  cyan: "#7cc7b8",
  white: "#c6c0b8",
  bright_black: "#948e86",
  bright_red: "#f08573",
  bright_green: "#abd084",
  bright_yellow: "#f0bd72",
  bright_blue: "#92c9e0",
  bright_magenta: "#dcabc9",
  bright_cyan: "#9cd9cc",
  bright_white: "#f3f0ea",
};

const DARK_DERIVED: DerivedColors = {
  bg_primary: "#151312",
  bg_secondary: "#201e1d",
  bg_toolbar: "#272524",
  text_primary: "#e8e4de",
  text_secondary: "#948e86",
  accent_blue: "#6fb1cc",
  accent_green: "#8fbc62",
  accent_orange: "#e0a458",
  accent_purple: "#c78bb0",
  accent_red: "#e0604c",
  accent_primary: "#d9924f",
  accent_secondary: "#7cc7b8",
  accent_tertiary: "#8fbc62",
  border: "#948e8680",
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
  background: "#151312",
  foreground: "#e8e4de",
  text_primary: "#e8e4de",
  text_secondary: "#948e86",
  text_sha: "#6fb1cc",
  selection: "#6fb1cc44",
  head_lane_tint: "#d9924f22",
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
  background: "#151312",
  foreground: "#e8e4de",
  cursor: "#6fb1cc",
  selection: "#6fb1cc44",
  line_highlight: "#201e1d66",
  gutter_bg: "#151312",
  gutter_fg: "#948e86",
  added_bg: "#1b3829",
  removed_bg: "#3c1e22",
  added_text: "#8fbc62",
  removed_text: "#e0604c",
  syntax_keyword: "#e0604c",
  syntax_string: "#a1ccdd",
  syntax_comment: "#948e86",
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
  background: "#faf8f5",
  foreground: "#36312b",
  black: "#4f4a43",
  red: "#bf4a36",
  green: "#5f7d3a",
  yellow: "#aa7430",
  blue: "#37749a",
  magenta: "#96587f",
  cyan: "#3a8579",
  white: "#9d968d",
  bright_black: "#6b645c",
  bright_red: "#a23a28",
  bright_green: "#4c682c",
  bright_yellow: "#8f5f1f",
  bright_blue: "#2a607f",
  bright_magenta: "#7d4668",
  bright_cyan: "#2c6f64",
  bright_white: "#ffffff",
};

const LIGHT_DERIVED: DerivedColors = {
  bg_primary: "#faf8f5",
  bg_secondary: "#f2f0ed",
  bg_toolbar: "#edebe8",
  text_primary: "#36312b",
  text_secondary: "#6b645c",
  accent_blue: "#37749a",
  accent_green: "#5f7d3a",
  accent_orange: "#aa7430",
  accent_purple: "#96587f",
  accent_red: "#bf4a36",
  accent_primary: "#b3702a",
  accent_secondary: "#3a8579",
  accent_tertiary: "#5f7d3a",
  border: "#6b645c80",
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
  background: "#faf8f5",
  foreground: "#36312b",
  text_primary: "#36312b",
  text_secondary: "#6b645c",
  text_sha: "#37749a",
  selection: "#37749a44",
  head_lane_tint: "#b3702a22",
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
  background: "#faf8f5",
  foreground: "#36312b",
  cursor: "#37749a",
  selection: "#37749a44",
  line_highlight: "#f2f0ed66",
  gutter_bg: "#faf8f5",
  gutter_fg: "#6b645c",
  added_bg: "#d4f8db",
  removed_bg: "#fdd8d8",
  added_text: "#5f7d3a",
  removed_text: "#bf4a36",
  syntax_keyword: "#bf4a36",
  syntax_string: "#375b9a",
  syntax_comment: "#6b645c",
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
