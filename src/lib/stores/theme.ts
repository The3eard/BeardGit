import { writable, get } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import type { ThemeData, GraphTheme } from "../types";
import { getTheme, getUiScale } from "../api/tauri";
import { DEFAULT_GRAPH_THEME } from "../components/graph/graph-renderer";

export const activeTheme = writable<ThemeData | null>(null);

export function buildGraphTheme(theme: ThemeData): GraphTheme {
  const g = theme.graph;
  return {
    background: g.background,
    currentLine: g.selection,
    selection: g.selection,
    foreground: g.foreground,
    comment: theme.colors.text_secondary,
    red: theme.colors.accent_red,
    orange: theme.colors.accent_orange,
    yellow: theme.colors.accent_orange,
    green: theme.colors.accent_green,
    cyan: theme.colors.accent_blue,
    purple: theme.colors.accent_purple,
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
  };
}

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

export function applyTheme(theme: ThemeData): void {
  const el = document.documentElement.style;
  const c = theme.colors;

  el.setProperty("--bg-primary", c.bg_primary);
  el.setProperty("--bg-secondary", c.bg_secondary);
  el.setProperty("--bg-toolbar", c.bg_toolbar);
  el.setProperty("--text-primary", c.text_primary);
  el.setProperty("--text-secondary", c.text_secondary);
  el.setProperty("--accent-blue", c.accent_blue);
  el.setProperty("--accent-green", c.accent_green);
  el.setProperty("--accent-orange", c.accent_orange);
  el.setProperty("--accent-purple", c.accent_purple);
  el.setProperty("--accent-red", c.accent_red);
  el.setProperty("--border", c.border);
  el.setProperty("--selection", c.selection);
  el.setProperty("--theme-mode", theme.meta.mode);

  const overlays = computeOverlays(theme.meta.mode);
  for (const [key, value] of Object.entries(overlays)) {
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
  return theme ? buildGraphTheme(theme) : DEFAULT_GRAPH_THEME;
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

export async function listenThemeChanges(): Promise<void> {
  await listen<ThemeData>("theme-changed", (event) => {
    activeTheme.set(event.payload);
    applyTheme(event.payload);
  });
}

export function applyUiScale(percent: number): void {
  document.documentElement.style.zoom = `${percent}%`;
}

export async function initUiScale(): Promise<void> {
  try {
    const scale = await getUiScale();
    applyUiScale(scale);
  } catch {
    // Default to 100%
  }
}
