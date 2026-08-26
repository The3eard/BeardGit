/**
 * Unit tests for `LookAndFeelSection.svelte` — the extracted visual
 * preferences section (language / theme-auto / theme / UI scale) shared
 * between the "General" category (in v1) and any future surfaces.
 *
 * The component is extracted out of the old `GeneralSettings.svelte`
 * Look & Feel block so the parent `Card` owns the single heading,
 * eliminating the duplicated "Look & feel" label (spec problem 1).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, fireEvent } from "@testing-library/svelte";
import { tick } from "svelte";

vi.mock("$lib/stores/locale", async () => {
  const { writable } = await import("svelte/store");
  const currentLocale = writable<string>("en-US");
  const changeLocale = vi.fn(async (_: string) => {});
  return { currentLocale, changeLocale };
});

const checkThemeContrastMock = vi.fn(async (name: string) => ({
  theme_id: name,
  warnings: [] as Array<Record<string, unknown>>,
  unaudited: [] as string[],
}));

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
  checkThemeContrast: (name: string) => checkThemeContrastMock(name),
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

import type { ThemeData } from "$lib/types";
import themeFixtures from "../../../stores/__fixtures__/themes.json";
import LookAndFeelSection from "../LookAndFeelSection.svelte";
import { currentLocale, changeLocale } from "$lib/stores/locale";

beforeEach(() => {
  (changeLocale as unknown as ReturnType<typeof vi.fn>).mockClear();
  currentLocale.set("en-US");
});

afterEach(() => cleanup());

describe("LookAndFeelSection", () => {
  it("renders a single language <select> with both locale options", async () => {
    const { container } = render(LookAndFeelSection);
    await tick();

    const languageSelects = container.querySelectorAll<HTMLSelectElement>(
      'select#language-select',
    );
    expect(languageSelects.length).toBe(1);

    const options = Array.from(
      languageSelects[0].querySelectorAll("option"),
    ).map((opt) => opt.value);
    expect(options).toContain("en-US");
    expect(options).toContain("es-ES");
  });

  it("renders the theme-auto checkbox, theme <select>, and UI-scale <select>", async () => {
    const { container } = render(LookAndFeelSection);
    await tick();

    expect(
      container.querySelector<HTMLInputElement>("input#theme-auto"),
    ).not.toBeNull();
    expect(
      container.querySelector<HTMLSelectElement>("select#theme-select"),
    ).not.toBeNull();
    expect(
      container.querySelector<HTMLSelectElement>("select#scale-select"),
    ).not.toBeNull();
  });

  it("fires changeLocale when the language select changes", async () => {
    const { container } = render(LookAndFeelSection);
    await tick();

    const select = container.querySelector<HTMLSelectElement>(
      "select#language-select",
    )!;
    await fireEvent.change(select, { target: { value: "es-ES" } });
    await tick();

    expect(changeLocale).toHaveBeenCalledWith("es-ES");
  });

  it("exposes a data-setting-anchor for each settings row", async () => {
    const { container } = render(LookAndFeelSection);
    await tick();

    for (const anchor of ["language", "theme-auto", "theme", "ui-scale"]) {
      const el = container.querySelector(
        `[data-setting-anchor="${anchor}"]`,
      );
      expect(el, `expected [data-setting-anchor="${anchor}"]`).not.toBeNull();
    }
  });
});

// ── Contrast notice ─────────────────────────────────────────────────────
//
// The policy this encodes: a low-contrast theme is *reported*, never
// corrected. So the notice must appear without blocking selection, and it
// must not appear for a theme that passes.

describe("LookAndFeelSection — contrast notice", () => {
  /**
   * A complete `ThemeData` with only `meta` varied.
   *
   * Built from the generated fixture rather than hand-rolled, so the store
   * gets the real serde shape — the component only reads `meta.id`, but
   * `activeTheme` is typed `ThemeData` and a stub would need a cast.
   */
  function themeWith(id: string, name: string, mode: string): ThemeData {
    const base = (themeFixtures as unknown as Record<string, ThemeData>)[
      "beardgit-light"
    ];
    return { ...base, meta: { id, name, mode, complementary: null } };
  }

  /** Render and let the async onMount probes settle. */
  async function renderSettled() {
    const rendered = render(LookAndFeelSection);
    await new Promise((resolve) => setTimeout(resolve, 0));
    await tick();
    return rendered;
  }

  beforeEach(async () => {
    checkThemeContrastMock.mockReset();
    checkThemeContrastMock.mockImplementation(async (name: string) => ({
      theme_id: name,
      warnings: [],
      unaudited: [],
    }));
    // The mocked `activeTheme` is a module-level singleton, so a test that
    // switches it would otherwise leak into the next one.
    const { activeTheme } = await import("$lib/stores/theme");
    activeTheme.set(themeWith("dark", "Dark", "dark"));
  });

  it("shows no notice for a theme that passes", async () => {
    const { queryByTestId } = await renderSettled();
    expect(queryByTestId("theme-contrast-notice")).toBeNull();
  });

  it("lists each failing token with its ratio", async () => {
    checkThemeContrastMock.mockImplementation(async (name: string) => ({
      theme_id: name,
      unaudited: [],
      warnings: [
        {
          token: "text_secondary",
          foreground: "#4c566a",
          background: "#2e3440",
          ratio: 1.69,
          required: 4.5,
        },
      ],
    }));

    const { getByTestId } = await renderSettled();

    const notice = getByTestId("theme-contrast-notice");
    expect(notice.textContent).toContain("text_secondary");
    expect(notice.textContent).toContain("1.69");
    expect(notice.textContent).toContain("4.5");
  });

  it("re-audits when the active theme changes, clearing a stale notice", async () => {
    // Drives `activeTheme` rather than the <select>, because that is the
    // real trigger: the OS dark/light auto-switch changes the theme through
    // the `theme-changed` listener and never touches `handleThemeChange`.
    // A one-shot read on mount left the notice describing the old theme.
    checkThemeContrastMock.mockImplementation(async (name: string) => ({
      theme_id: name,
      unaudited: [],
      warnings:
        name === "dark"
          ? [
              {
                token: "text_muted",
                foreground: "#3a405b",
                background: "#1a1b26",
                ratio: 1.68,
                required: 3.0,
              },
            ]
          : [],
    }));

    const { getByTestId, queryByTestId } = await renderSettled();
    expect(getByTestId("theme-contrast-notice")).toBeTruthy();

    const { activeTheme } = await import("$lib/stores/theme");
    activeTheme.set(themeWith("light", "Light", "light"));
    await new Promise((resolve) => setTimeout(resolve, 0));
    await tick();

    expect(queryByTestId("theme-contrast-notice")).toBeNull();
  });

  it("surfaces a notice when the auto-switch lands on a failing theme", async () => {
    // The inverse direction, which is the one that actually hid a problem:
    // follow-system flips to a low-contrast theme and the panel must start
    // warning, not stay silent because it audited once on mount.
    checkThemeContrastMock.mockImplementation(async (name: string) => ({
      theme_id: name,
      unaudited: [],
      warnings:
        name === "light"
          ? [
              {
                token: "text_secondary",
                foreground: "#89888d",
                background: "#ffffff",
                ratio: 3.52,
                required: 4.5,
              },
            ]
          : [],
    }));

    const { getByTestId, queryByTestId } = await renderSettled();
    expect(queryByTestId("theme-contrast-notice")).toBeNull();

    const { activeTheme } = await import("$lib/stores/theme");
    activeTheme.set(themeWith("light", "Light", "light"));
    await new Promise((resolve) => setTimeout(resolve, 0));
    await tick();

    expect(getByTestId("theme-contrast-notice").textContent).toContain(
      "text_secondary",
    );
  });

  it("shows unmeasurable tokens instead of reporting the theme clean", async () => {
    // `validate_color` accepts `rgba(…)` and the themes README documents it,
    // so a user can pin an unmeasurable `text-secondary`. Gating the notice
    // on `warnings` alone reported that theme as passing precisely because
    // it had never been checked.
    checkThemeContrastMock.mockImplementation(async (name: string) => ({
      theme_id: name,
      warnings: [],
      unaudited: ["text_secondary"],
    }));

    const { getByTestId } = await renderSettled();

    expect(getByTestId("theme-contrast-notice").textContent).toContain(
      "text_secondary",
    );
  });

  it("selecting a theme turns off follow-system and applies it", async () => {
    // Covers `handleThemeChange` itself. The contrast tests drive
    // `activeTheme` directly (that is the real trigger), which left this
    // path — including the themeAuto-disable branch — untested.
    const tauri = await import("$lib/api/tauri");
    const { getByTestId, container } = await renderSettled();
    void getByTestId;

    const select = container.querySelector("#theme-select") as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: "light" } });

    expect(vi.mocked(tauri.setThemeAuto)).toHaveBeenCalledWith(false);
    expect(vi.mocked(tauri.setTheme)).toHaveBeenCalledWith("light");
  });

  it("stays silent when the audit itself fails", async () => {
    // Advisory only: a broken audit must not block theme selection or
    // surface a scary banner.
    checkThemeContrastMock.mockImplementation(async () => {
      throw new Error("ipc unavailable");
    });

    const { queryByTestId, container } = await renderSettled();

    expect(queryByTestId("theme-contrast-notice")).toBeNull();
    expect(container.querySelector("#theme-select")).toBeTruthy();
  });
});
