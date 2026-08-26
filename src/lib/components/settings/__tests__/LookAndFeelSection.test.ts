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
  /** Render and let the async onMount probes settle. */
  async function renderSettled() {
    const rendered = render(LookAndFeelSection);
    await new Promise((resolve) => setTimeout(resolve, 0));
    await tick();
    return rendered;
  }

  beforeEach(() => {
    checkThemeContrastMock.mockReset();
    checkThemeContrastMock.mockImplementation(async (name: string) => ({
      theme_id: name,
      warnings: [],
    }));
  });

  it("shows no notice for a theme that passes", async () => {
    const { queryByTestId } = await renderSettled();
    expect(queryByTestId("theme-contrast-notice")).toBeNull();
  });

  it("lists each failing token with its ratio", async () => {
    checkThemeContrastMock.mockImplementation(async (name: string) => ({
      theme_id: name,
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

  it("re-audits when the theme changes and clears a stale notice", async () => {
    checkThemeContrastMock.mockImplementation(async (name: string) => ({
      theme_id: name,
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

    const { getByTestId, queryByTestId, container } = await renderSettled();
    expect(getByTestId("theme-contrast-notice")).toBeTruthy();

    const select = container.querySelector("#theme-select") as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: "light" } });
    await new Promise((resolve) => setTimeout(resolve, 0));
    await tick();

    expect(queryByTestId("theme-contrast-notice")).toBeNull();
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
