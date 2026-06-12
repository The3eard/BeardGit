import { writable, get } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { ThemeData, GraphTheme } from "../types";
import { getTheme, getUiScale } from "../api/tauri";
import { defaultGraphTheme } from "../components/graph/graph-renderer";

export const activeTheme = writable<ThemeData | null>(null);

/** Convert "#RRGGBB" to "r, g, b" for use in rgba(). */
function hexToRgb(hex: string): string {
  const h = hex.startsWith("#") ? hex.slice(1) : hex;
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return `${r}, ${g}, ${b}`;
}

export function buildGraphTheme(theme: ThemeData): GraphTheme {
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

/**
 * Derive six `--overlay-accent-*` CSS variables from the active theme's accent
 * colours and `text_secondary`, each at 10 % alpha.  These are written to
 * `document.documentElement` by `applyTheme` alongside the base tokens and
 * the existing `--overlay-hover/active/shadow` set.
 *
 * Theme JSON files do NOT need to declare these — they are computed at runtime
 * from the existing `ThemeData.derived` fields via `hexToRgb`.
 */
function computeAccentOverlays(d: ThemeData["derived"]): Record<string, string> {
  return {
    "--overlay-accent-blue":   `rgba(${hexToRgb(d.accent_blue)}, 0.1)`,
    "--overlay-accent-red":    `rgba(${hexToRgb(d.accent_red)}, 0.1)`,
    "--overlay-accent-green":  `rgba(${hexToRgb(d.accent_green)}, 0.1)`,
    "--overlay-accent-orange": `rgba(${hexToRgb(d.accent_orange)}, 0.1)`,
    "--overlay-accent-purple": `rgba(${hexToRgb(d.accent_purple)}, 0.1)`,
    "--overlay-accent-muted":  `rgba(${hexToRgb(d.text_secondary)}, 0.1)`,
  };
}

function computeOverlays(mode: string): Record<string, string> {
  if (mode === "light") {
    return {
      "--overlay-hover": "rgba(0,0,0,0.04)",
      "--overlay-active": "rgba(0,0,0,0.08)",
      "--overlay-shadow": "rgba(0,0,0,0.15)",
      "--shadow-overlay": "0 4px 12px rgba(0,0,0,0.14)",
      "--shadow-modal": "0 12px 32px rgba(0,0,0,0.22)",
    };
  }
  return {
    "--overlay-hover": "rgba(255,255,255,0.06)",
    "--overlay-active": "rgba(255,255,255,0.1)",
    "--overlay-shadow": "rgba(0,0,0,0.3)",
    "--shadow-overlay": "0 4px 12px rgba(0,0,0,0.35)",
    "--shadow-modal": "0 12px 32px rgba(0,0,0,0.45)",
  };
}

export function applyTheme(theme: ThemeData): void {
  const el = document.documentElement.style;
  const d = theme.derived;

  el.setProperty("--bg-primary", d.bg_primary);
  el.setProperty("--bg-secondary", d.bg_secondary);
  el.setProperty("--bg-toolbar", d.bg_toolbar);
  el.setProperty("--text-primary", d.text_primary);
  el.setProperty("--text-secondary", d.text_secondary);
  el.setProperty("--accent-blue", d.accent_blue);
  el.setProperty("--accent-green", d.accent_green);
  el.setProperty("--accent-orange", d.accent_orange);
  el.setProperty("--accent-purple", d.accent_purple);
  el.setProperty("--accent-red", d.accent_red);
  // Per-theme signature accents. New components (and progressive
  // migrations of existing ones) lean on `--accent-primary` for
  // primary actions / focus / spinner so each theme can assert its
  // own identity instead of every theme being "blue-flavoured".
  el.setProperty("--accent-primary", d.accent_primary);
  el.setProperty("--accent-secondary", d.accent_secondary);
  el.setProperty("--accent-tertiary", d.accent_tertiary);
  el.setProperty("--border", d.border);
  el.setProperty("--selection", d.selection);
  el.setProperty("--theme-mode", theme.meta.mode);
  // Native controls (checkbox, select, scrollbar) follow the theme's
  // mode instead of always rendering light. Mirrors the static default
  // in app.css `:root`.
  el.setProperty("color-scheme", theme.meta.mode);

  const overlays = computeOverlays(theme.meta.mode);
  for (const [key, value] of Object.entries(overlays)) {
    el.setProperty(key, value);
  }

  const accentOverlays = computeAccentOverlays(d);
  for (const [key, value] of Object.entries(accentOverlays)) {
    el.setProperty(key, value);
  }

  updateCachedStatusColors();
}

let cachedStatusColors: Record<string, string> = {};

function updateCachedStatusColors(): void {
  const style = getComputedStyle(document.documentElement);
  cachedStatusColors = {
    success: style.getPropertyValue("--accent-green").trim(),
    failed: style.getPropertyValue("--accent-red").trim(),
    timed_out: style.getPropertyValue("--accent-red").trim(),
    running: style.getPropertyValue("--accent-blue").trim(),
    pending: style.getPropertyValue("--accent-orange").trim(),
    queued: style.getPropertyValue("--accent-orange").trim(),
    manual: style.getPropertyValue("--accent-purple").trim(),
    canceled: style.getPropertyValue("--text-secondary").trim(),
    skipped: style.getPropertyValue("--text-secondary").trim(),
  };
}

export function getThemedStatusColor(status: string): string {
  return cachedStatusColors[status] || "";
}

export function currentGraphTheme(): GraphTheme {
  const theme = get(activeTheme);
  return theme ? buildGraphTheme(theme) : defaultGraphTheme();
}

export async function initTheme(themeName: string): Promise<void> {
  try {
    const theme = await getTheme(themeName);
    activeTheme.set(theme);
    applyTheme(theme);
  } catch (e) {
    console.error("Failed to load theme:", e);
  }
}

let unlistenThemeChanged: UnlistenFn | null = null;

export async function listenThemeChanges(): Promise<void> {
  // Idempotent: a second call without teardown would leak the prior listener.
  if (unlistenThemeChanged) return;
  unlistenThemeChanged = await listen<ThemeData>("theme-changed", (event) => {
    activeTheme.set(event.payload);
    applyTheme(event.payload);
  });
}

/** Tear down the theme-changed listener (teardown symmetry / HMR safety). */
export function stopThemeListener(): void {
  unlistenThemeChanged?.();
  unlistenThemeChanged = null;
}

export async function applyUiScale(percent: number): Promise<void> {
  const scaleFactor = percent / 100;
  await getCurrentWebview().setZoom(scaleFactor);
}

export async function initUiScale(): Promise<void> {
  try {
    const scale = await getUiScale();
    await applyUiScale(scale);
  } catch {
    // Default to 100%
  }
}
