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
