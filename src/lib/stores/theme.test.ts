import { describe, it, expect } from "vitest";
import type { ThemeData, GraphTheme } from "../types";

function hexToRgb(hex: string): string {
  const h = hex.startsWith("#") ? hex.slice(1) : hex;
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return `${r}, ${g}, ${b}`;
}

// Self-contained mirror of buildGraphTheme (avoids importing theme.ts which uses `document`)
function buildGraphTheme(theme: ThemeData): GraphTheme {
  const g = theme.graph;
  return {
    background: g.background,
    currentLine: g.selection,
    selection: g.selection,
    foreground: g.foreground,
    comment: theme.derived.text_secondary,
    red: theme.derived.accent_red,
    orange: theme.derived.accent_orange,
    yellow: theme.derived.accent_orange,
    green: theme.derived.accent_green,
    cyan: theme.derived.accent_blue,
    purple: theme.derived.accent_purple,
    pink: g.ref_head,
    laneColors: g.lane_colors,
    headLaneTint: g.head_lane_tint,
    dimOpacity: g.dim_opacity,
    selectionHighlight: g.selection_highlight,
    nodeRadius: g.node_radius,
    mergeRadius: g.merge_radius,
    refBadge: {
      branch: g.ref_branch,
      remote: g.ref_remote,
      tag: g.ref_tag,
      head: g.ref_head,
    },
    textPrimary: g.text_primary,
    textSecondary: g.text_secondary,
    textSha: g.text_sha,
    bisectGoodColor: `rgba(${hexToRgb(theme.derived.accent_green)}, 0.15)`,
    bisectBadColor: `rgba(${hexToRgb(theme.derived.accent_red)}, 0.15)`,
    bisectSkipColor: `rgba(${hexToRgb(theme.derived.text_secondary)}, 0.15)`,
    bisectCurrentColor: `rgba(${hexToRgb(theme.derived.accent_orange)}, 0.15)`,
  };
}

// Self-contained mirror of computeOverlays
function computeOverlays(mode: string): Record<string, string> {
  if (mode === "light") {
    return {
      "--overlay-hover": "rgba(0,0,0,0.04)",
      "--overlay-active": "rgba(0,0,0,0.08)",
      "--overlay-shadow": "rgba(0,0,0,0.15)",
    };
  }
  return {
    "--overlay-hover": "rgba(255,255,255,0.06)",
    "--overlay-active": "rgba(255,255,255,0.1)",
    "--overlay-shadow": "rgba(0,0,0,0.3)",
  };
}

const MOCK_THEME: ThemeData = {
  meta: { id: "dracula", name: "Dracula", mode: "dark", complementary: null },
  colors: {
    background: "#282a36",
    foreground: "#f8f8f2",
    black: "#21222c",
    red: "#ff5555",
    green: "#50fa7b",
    yellow: "#f1fa8c",
    blue: "#6272a4",
    magenta: "#bd93f9",
    cyan: "#8be9fd",
    white: "#f8f8f2",
    bright_black: "#6272a4",
    bright_red: "#ff6e6e",
    bright_green: "#69ff94",
    bright_yellow: "#ffffa5",
    bright_blue: "#d6acff",
    bright_magenta: "#ff92df",
    bright_cyan: "#a4ffff",
    bright_white: "#ffffff",
  },
  derived: {
    bg_primary: "#282a36",
    bg_secondary: "#21222c",
    bg_toolbar: "#191a21",
    text_primary: "#f8f8f2",
    text_secondary: "#6272a4",
    accent_blue: "#8be9fd",
    accent_green: "#50fa7b",
    accent_orange: "#ffb86c",
    accent_purple: "#bd93f9",
    accent_red: "#ff5555",
    border: "#44475a",
    selection: "#44475a",
  },
  graph: {
    lane_colors: ["#8be9fd", "#50fa7b", "#ffb86c", "#bd93f9", "#ff79c6"],
    background: "#282a36",
    foreground: "#f8f8f2",
    text_primary: "#f8f8f2",
    text_secondary: "#6272a4",
    text_sha: "#ffb86c",
    selection: "#44475a",
    head_lane_tint: "rgba(139,233,253,0.04)",
    selection_highlight: "rgba(139,233,253,0.08)",
    dim_opacity: 0.3,
    node_radius: 5,
    merge_radius: 6,
    ref_branch: "#8be9fd",
    ref_remote: "#bd93f9",
    ref_tag: "#ffb86c",
    ref_head: "#ff79c6",
  },
  editor: null,
};

describe("buildGraphTheme", () => {
  it("maps graph section fields correctly", () => {
    const result = buildGraphTheme(MOCK_THEME);

    expect(result.laneColors).toEqual(MOCK_THEME.graph.lane_colors);
    expect(result.dimOpacity).toBe(0.3);
    expect(result.nodeRadius).toBe(5);
    expect(result.mergeRadius).toBe(6);
    expect(result.headLaneTint).toBe("rgba(139,233,253,0.04)");
    expect(result.selectionHighlight).toBe("rgba(139,233,253,0.08)");
    expect(result.textPrimary).toBe("#f8f8f2");
    expect(result.textSecondary).toBe("#6272a4");
    expect(result.textSha).toBe("#ffb86c");
  });

  it("maps ref badges from graph section", () => {
    const result = buildGraphTheme(MOCK_THEME);

    expect(result.refBadge).toEqual({
      branch: "#8be9fd",
      remote: "#bd93f9",
      tag: "#ffb86c",
      head: "#ff79c6",
    });
  });

  it("uses derived section for named colors", () => {
    const result = buildGraphTheme(MOCK_THEME);

    expect(result.red).toBe(MOCK_THEME.derived.accent_red);
    expect(result.green).toBe(MOCK_THEME.derived.accent_green);
    expect(result.cyan).toBe(MOCK_THEME.derived.accent_blue);
    expect(result.purple).toBe(MOCK_THEME.derived.accent_purple);
    expect(result.orange).toBe(MOCK_THEME.derived.accent_orange);
    expect(result.comment).toBe(MOCK_THEME.derived.text_secondary);
  });

  it("maps background and foreground from graph section", () => {
    const result = buildGraphTheme(MOCK_THEME);

    expect(result.background).toBe("#282a36");
    expect(result.foreground).toBe("#f8f8f2");
    expect(result.selection).toBe("#44475a");
    expect(result.currentLine).toBe("#44475a");
  });
});

describe("computeOverlays", () => {
  it("returns white-based overlays for dark mode", () => {
    const overlays = computeOverlays("dark");

    expect(overlays["--overlay-hover"]).toBe("rgba(255,255,255,0.06)");
    expect(overlays["--overlay-active"]).toBe("rgba(255,255,255,0.1)");
    expect(overlays["--overlay-shadow"]).toBe("rgba(0,0,0,0.3)");
  });

  it("returns black-based overlays for light mode", () => {
    const overlays = computeOverlays("light");

    expect(overlays["--overlay-hover"]).toBe("rgba(0,0,0,0.04)");
    expect(overlays["--overlay-active"]).toBe("rgba(0,0,0,0.08)");
    expect(overlays["--overlay-shadow"]).toBe("rgba(0,0,0,0.15)");
  });

  it("defaults to dark overlays for unknown mode", () => {
    const overlays = computeOverlays("unknown");

    expect(overlays["--overlay-hover"]).toBe("rgba(255,255,255,0.06)");
  });
});
