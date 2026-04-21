/**
 * Unit tests for `GeneralSettings.svelte` — enforces the post-extraction
 * shape: a single Look & Feel `Card` whose body is rendered by
 * `LookAndFeelSection.svelte`.
 *
 * Asserts there is exactly one of each `data-setting-anchor` and exactly
 * one Look & Feel heading (no duplicate inner-vs-outer title), and that
 * the `settingsIndex` export still declares every row the shell search
 * needs.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";

vi.mock("$lib/stores/locale", async () => {
  const { writable } = await import("svelte/store");
  const currentLocale = writable<string>("en-US");
  const changeLocale = vi.fn(async (_: string) => {});
  return { currentLocale, changeLocale };
});

vi.mock("$lib/api/tauri", () => ({
  listThemes: vi
    .fn()
    .mockResolvedValue([
      { id: "dark", name: "Dark" },
      { id: "light", name: "Light" },
    ]),
  getThemeAuto: vi.fn().mockResolvedValue(true),
  setTheme: vi.fn(),
  setThemeAuto: vi.fn(),
  getUiScale: vi.fn().mockResolvedValue(100),
  setUiScale: vi.fn(),
}));

vi.mock("$lib/stores/theme", async () => {
  const { writable } = await import("svelte/store");
  return {
    activeTheme: writable({
      meta: { id: "dark", name: "Dark" },
    }),
    applyUiScale: vi.fn(),
  };
});

import GeneralSettings, {
  settingsIndex,
} from "../GeneralSettings.svelte";

beforeEach(() => {});
afterEach(() => cleanup());

describe("GeneralSettings (post-extraction)", () => {
  it("renders exactly one [data-setting-anchor=\"theme\"] element", async () => {
    const { container } = render(GeneralSettings);
    await tick();
    const matches = container.querySelectorAll('[data-setting-anchor="theme"]');
    expect(matches.length).toBe(1);
  });

  it("renders exactly one [data-setting-anchor=\"language\"] element", async () => {
    const { container } = render(GeneralSettings);
    await tick();
    const matches = container.querySelectorAll(
      '[data-setting-anchor="language"]',
    );
    expect(matches.length).toBe(1);
  });

  it("renders a single Look & Feel heading (no duplicate inner-vs-outer title)", async () => {
    const { container } = render(GeneralSettings);
    await tick();
    const matches = container.querySelectorAll(
      '[data-testid="look-and-feel-heading"]',
    );
    expect(matches.length).toBe(1);
  });

  it("exports descriptors for every Look & Feel row", () => {
    const ids = settingsIndex.map((s) => s.id);
    expect(ids).toContain("general.theme");
    expect(ids).toContain("general.language");
    expect(ids).toContain("general.theme-auto");
    expect(ids).toContain("general.ui-scale");
  });
});
