/**
 * Theme fixtures.
 *
 * `ThemeData` is what `resolve_startup_theme` / `get_theme` return —
 * the Rust side derives `derived`, `graph`, and `editor` sections from
 * the base 16 colours; we hardcode plausible GitHub-ish values for
 * each mode so the visual suite has a stable baseline that doesn't
 * depend on the Rust theme pipeline running.
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

// ─── Dark (github-dark-ish) ──────────────────────────────────────────

const DARK_META: ThemeMeta = {
  id: "github-dark",
  name: "GitHub Dark",
  mode: "dark",
  complementary: "github-light",
};

const DARK_BASE: ThemeBaseColors = {
  background: "#0d1117",
  foreground: "#e6edf3",
  black: "#484f58",
  red: "#ff7b72",
  green: "#3fb950",
  yellow: "#d29922",
  blue: "#58a6ff",
  magenta: "#bc8cff",
  cyan: "#76e3ea",
  white: "#b1bac4",
  bright_black: "#6e7681",
  bright_red: "#ff7b72",
  bright_green: "#56d364",
  bright_yellow: "#e3b341",
  bright_blue: "#79c0ff",
  bright_magenta: "#d2a8ff",
  bright_cyan: "#a5e9ee",
  bright_white: "#f0f6fc",
};

const DARK_DERIVED: DerivedColors = {
  bg_primary: "#0d1117",
  bg_secondary: "#161b22",
  bg_toolbar: "#0d1117",
  text_primary: "#e6edf3",
  text_secondary: "#8b949e",
  accent_blue: "#58a6ff",
  accent_green: "#3fb950",
  accent_orange: "#d29922",
  accent_purple: "#bc8cff",
  accent_red: "#f85149",
  accent_primary: "#58a6ff",
  accent_secondary: "#bc8cff",
  accent_tertiary: "#3fb950",
  border: "#30363d",
  selection: "#1f6feb40",
};

const DARK_GRAPH: ThemeGraphData = {
  lane_colors: [
    "#58a6ff",
    "#3fb950",
    "#d29922",
    "#bc8cff",
    "#f85149",
    "#76e3ea",
  ],
  background: "#0d1117",
  foreground: "#e6edf3",
  text_primary: "#e6edf3",
  text_secondary: "#8b949e",
  text_sha: "#8b949e",
  selection: "#1f6feb40",
  head_lane_tint: "#1f6feb26",
  selection_highlight: "#1f6feb40",
  dim_opacity: 0.5,
  node_radius: 5,
  merge_radius: 4,
  ref_branch: "#58a6ff",
  ref_remote: "#bc8cff",
  ref_tag: "#3fb950",
  ref_head: "#d29922",
};

const DARK_EDITOR: ThemeEditorData = {
  background: "#0d1117",
  foreground: "#e6edf3",
  cursor: "#58a6ff",
  selection: "#1f6feb40",
  line_highlight: "#161b22",
  gutter_bg: "#0d1117",
  gutter_fg: "#6e7681",
  added_bg: "#2ea04326",
  removed_bg: "#f8514926",
  added_text: "#3fb950",
  removed_text: "#f85149",
  syntax_keyword: "#ff7b72",
  syntax_string: "#a5d6ff",
  syntax_comment: "#8b949e",
  syntax_function: "#d2a8ff",
  syntax_type: "#79c0ff",
  syntax_number: "#79c0ff",
  syntax_operator: "#ff7b72",
  syntax_property: "#79c0ff",
};

// ─── Light (github-light-ish) ────────────────────────────────────────

const LIGHT_META: ThemeMeta = {
  id: "github-light",
  name: "GitHub Light",
  mode: "light",
  complementary: "github-dark",
};

const LIGHT_BASE: ThemeBaseColors = {
  background: "#ffffff",
  foreground: "#1f2328",
  black: "#24292f",
  red: "#cf222e",
  green: "#116329",
  yellow: "#4d2d00",
  blue: "#0969da",
  magenta: "#8250df",
  cyan: "#1b7c83",
  white: "#6e7781",
  bright_black: "#57606a",
  bright_red: "#a40e26",
  bright_green: "#1a7f37",
  bright_yellow: "#633c01",
  bright_blue: "#218bff",
  bright_magenta: "#a475f9",
  bright_cyan: "#3192aa",
  bright_white: "#8c959f",
};

const LIGHT_DERIVED: DerivedColors = {
  bg_primary: "#ffffff",
  bg_secondary: "#f6f8fa",
  bg_toolbar: "#ffffff",
  text_primary: "#1f2328",
  text_secondary: "#656d76",
  accent_blue: "#0969da",
  accent_green: "#1a7f37",
  accent_orange: "#bc4c00",
  accent_purple: "#8250df",
  accent_red: "#cf222e",
  accent_primary: "#0969da",
  accent_secondary: "#8250df",
  accent_tertiary: "#1a7f37",
  border: "#d0d7de",
  selection: "#0969da33",
};

const LIGHT_GRAPH: ThemeGraphData = {
  lane_colors: [
    "#0969da",
    "#1a7f37",
    "#bc4c00",
    "#8250df",
    "#cf222e",
    "#1b7c83",
  ],
  background: "#ffffff",
  foreground: "#1f2328",
  text_primary: "#1f2328",
  text_secondary: "#656d76",
  text_sha: "#656d76",
  selection: "#0969da33",
  head_lane_tint: "#0969da1a",
  selection_highlight: "#0969da26",
  dim_opacity: 0.5,
  node_radius: 5,
  merge_radius: 4,
  ref_branch: "#0969da",
  ref_remote: "#8250df",
  ref_tag: "#1a7f37",
  ref_head: "#bc4c00",
};

const LIGHT_EDITOR: ThemeEditorData = {
  background: "#ffffff",
  foreground: "#1f2328",
  cursor: "#0969da",
  selection: "#0969da33",
  line_highlight: "#f6f8fa",
  gutter_bg: "#ffffff",
  gutter_fg: "#8c959f",
  added_bg: "#1a7f3726",
  removed_bg: "#cf222e26",
  added_text: "#1a7f37",
  removed_text: "#cf222e",
  syntax_keyword: "#cf222e",
  syntax_string: "#0a3069",
  syntax_comment: "#6e7781",
  syntax_function: "#8250df",
  syntax_type: "#0550ae",
  syntax_number: "#0550ae",
  syntax_operator: "#cf222e",
  syntax_property: "#0550ae",
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
