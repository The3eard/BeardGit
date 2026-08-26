/**
 * The `derived` / `graph` half of the theme contract.
 *
 * ## Why this file was rewritten
 *
 * It used to hold a hand-written `MOCK_THEME` and local re-implementations
 * of `buildGraphTheme`, `computeOverlays` and `computeAccentOverlays` —
 * so every assertion compared a copy of the mapping against itself, on a
 * fixture written to agree with `src/lib/types/index.ts` by construction.
 * That is exactly the shape that let all 15 `[editor]` tokens stay inert
 * for months while this suite was green.
 *
 * Now: real functions, real serialized data. The fixture comes from
 * `storage::theme::tests::regenerate_theme_fixtures` and is pinned to live
 * serialization by `test_theme_fixtures_match_live_serialization`.
 */

import { beforeEach, describe, expect, it } from "vitest";

import type { ThemeData } from "../types";
import { applyTheme, buildGraphTheme } from "./theme";
import fixtures from "./__fixtures__/themes.json";

const themes: Record<string, ThemeData> = fixtures;
const DARK = themes["beardgit-dark"];
const LIGHT = themes["github-light"];

function hexToRgb(hex: string): string {
  const h = hex.startsWith("#") ? hex.slice(1) : hex;
  return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)).join(", ");
}

/** Read a custom property off the documentElement's inline style. */
function token(name: string): string {
  return document.documentElement.style.getPropertyValue(name);
}

describe("buildGraphTheme", () => {
  it("maps the graph section field for field", () => {
    const g = DARK.graph;
    const result = buildGraphTheme(DARK);

    expect(result.laneColors).toEqual(g.lane_colors);
    expect(result.background).toBe(g.background);
    expect(result.foreground).toBe(g.foreground);
    expect(result.selection).toBe(g.selection);
    expect(result.currentLine).toBe(g.selection);
    expect(result.textPrimary).toBe(g.text_primary);
    expect(result.textSecondary).toBe(g.text_secondary);
    expect(result.textSha).toBe(g.text_sha);
    expect(result.headLaneTint).toBe(g.head_lane_tint);
    expect(result.selectionHighlight).toBe(g.selection_highlight);
    expect(result.dimOpacity).toBe(g.dim_opacity);
    expect(result.nodeRadius).toBe(g.node_radius);
    expect(result.mergeRadius).toBe(g.merge_radius);
  });

  it("maps ref badges from the graph section", () => {
    const g = DARK.graph;
    expect(buildGraphTheme(DARK).refBadge).toEqual({
      branch: g.ref_branch,
      remote: g.ref_remote,
      tag: g.ref_tag,
      head: g.ref_head,
    });
  });

  it("takes its named colors from the derived section, not the graph one", () => {
    const d = DARK.derived;
    const result = buildGraphTheme(DARK);

    expect(result.red).toBe(d.accent_red);
    expect(result.green).toBe(d.accent_green);
    expect(result.cyan).toBe(d.accent_blue);
    expect(result.purple).toBe(d.accent_purple);
    expect(result.orange).toBe(d.accent_orange);
    expect(result.comment).toBe(d.text_secondary);
  });

  it("tints the bisect states from the derived accents", () => {
    const d = DARK.derived;
    const result = buildGraphTheme(DARK);

    expect(result.bisectGoodColor).toBe(`rgba(${hexToRgb(d.accent_green)}, 0.15)`);
    expect(result.bisectBadColor).toBe(`rgba(${hexToRgb(d.accent_red)}, 0.15)`);
    expect(result.bisectSkipColor).toBe(`rgba(${hexToRgb(d.text_secondary)}, 0.15)`);
    expect(result.bisectCurrentColor).toBe(`rgba(${hexToRgb(d.accent_orange)}, 0.15)`);
  });
});

describe("applyTheme writes the derived palette", () => {
  beforeEach(() => {
    // `applyTheme` only ever sets properties, so without this the tokens
    // from a previous test leak into the next one's reads.
    document.documentElement.removeAttribute("style");
  });

  it("maps every derived color onto its token", () => {
    const d = DARK.derived;
    applyTheme(DARK);

    expect(token("--bg-primary")).toBe(d.bg_primary);
    expect(token("--bg-secondary")).toBe(d.bg_secondary);
    expect(token("--bg-toolbar")).toBe(d.bg_toolbar);
    expect(token("--text-primary")).toBe(d.text_primary);
    expect(token("--text-secondary")).toBe(d.text_secondary);
    expect(token("--text-muted")).toBe(d.text_muted);
    expect(token("--accent-blue")).toBe(d.accent_blue);
    expect(token("--accent-green")).toBe(d.accent_green);
    expect(token("--accent-orange")).toBe(d.accent_orange);
    expect(token("--accent-purple")).toBe(d.accent_purple);
    expect(token("--accent-red")).toBe(d.accent_red);
    expect(token("--accent-primary")).toBe(d.accent_primary);
    expect(token("--accent-secondary")).toBe(d.accent_secondary);
    expect(token("--accent-tertiary")).toBe(d.accent_tertiary);
    expect(token("--border")).toBe(d.border);
    expect(token("--border-strong")).toBe(d.border_strong);
    expect(token("--selection")).toBe(d.selection);
  });

  it("follows the mode for the neutral overlays", () => {
    applyTheme(DARK);
    expect(token("--overlay-hover")).toBe("rgba(255,255,255,0.06)");
    expect(token("--overlay-active")).toBe("rgba(255,255,255,0.1)");
    expect(token("--overlay-shadow")).toBe("rgba(0,0,0,0.3)");
    expect(token("color-scheme")).toBe("dark");

    document.documentElement.removeAttribute("style");
    applyTheme(LIGHT);
    expect(token("--overlay-hover")).toBe("rgba(0,0,0,0.04)");
    expect(token("--overlay-active")).toBe("rgba(0,0,0,0.08)");
    expect(token("--overlay-shadow")).toBe("rgba(0,0,0,0.15)");
    expect(token("color-scheme")).toBe("light");
  });

  it.each([
    ["beardgit-dark"],
    ["github-light"],
  ])("derives the accent overlays from %s's own accents", (id) => {
    const d = themes[id].derived;
    applyTheme(themes[id]);

    expect(token("--overlay-accent-blue")).toBe(`rgba(${hexToRgb(d.accent_blue)}, 0.1)`);
    expect(token("--overlay-accent-red")).toBe(`rgba(${hexToRgb(d.accent_red)}, 0.1)`);
    expect(token("--overlay-accent-green")).toBe(`rgba(${hexToRgb(d.accent_green)}, 0.1)`);
    expect(token("--overlay-accent-orange")).toBe(`rgba(${hexToRgb(d.accent_orange)}, 0.1)`);
    expect(token("--overlay-accent-purple")).toBe(`rgba(${hexToRgb(d.accent_purple)}, 0.1)`);
    expect(token("--overlay-accent-muted")).toBe(`rgba(${hexToRgb(d.text_secondary)}, 0.1)`);
  });

  it("runs the selected-row fill off the signature accent, stronger on light", () => {
    applyTheme(DARK);
    expect(token("--overlay-selected")).toBe(
      `rgba(${hexToRgb(DARK.derived.accent_primary)}, 0.13)`,
    );

    document.documentElement.removeAttribute("style");
    applyTheme(LIGHT);
    expect(token("--overlay-selected")).toBe(
      `rgba(${hexToRgb(LIGHT.derived.accent_primary)}, 0.16)`,
    );
  });
});
