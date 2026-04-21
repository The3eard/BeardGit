/**
 * Unit tests for the rebuilt `SettingsPage.svelte` shell.
 *
 * Covers:
 *  - Deep-link bridge: the legacy `pendingSettingsSection` store
 *    still drives the active category, with the IA-era slug mapping
 *    applied (`connection` → integrations, `updates` → advanced, …).
 *  - Search: typing into the top-bar `SearchInput` filters the
 *    flat settings index and rendering the result list.
 *
 * Heavy subcomponents are replaced with a minimal Svelte stub so we
 * dodge the Tauri IPC + OS detection their real implementations
 * pull in. The assertions target the shell-level DOM only.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, fireEvent } from "@testing-library/svelte";
import { tick } from "svelte";
import { get } from "svelte/store";
import Stub from "./__stubs__/Stub.svelte";
import { pendingSettingsSection } from "$lib/stores/navigation";
import {
  DEFAULT_CATEGORY,
  settingsRoute,
} from "$lib/stores/settingsRoute";

// Replace every heavy category component with a bare stub so the
// shell test doesn't pull Tauri / OS dependencies. Each mock also
// exposes the `settingsIndex` export the shell aggregates.
vi.mock("../GeneralSettings.svelte", () => ({
  default: Stub,
  settingsIndex: [
    {
      id: "general.theme",
      label: "Theme",
      description: "Pick a colour theme",
      category: "general",
      anchor: "theme",
    },
    {
      id: "general.density",
      label: "Density",
      description: "Compact vs comfortable",
      category: "general",
      anchor: "density",
    },
  ],
}));
vi.mock("../EditorDiffSettings.svelte", () => ({
  default: Stub,
  settingsIndex: [],
}));
vi.mock("../GitSettings.svelte", () => ({
  default: Stub,
  settingsIndex: [],
}));
vi.mock("../AiSettings.svelte", () => ({
  default: Stub,
  settingsIndex: [],
}));
vi.mock("../IntegrationsSettings.svelte", () => ({
  default: Stub,
  settingsIndex: [],
}));
vi.mock("../AdvancedSettings.svelte", () => ({
  default: Stub,
  settingsIndex: [],
}));

import SettingsPage from "../SettingsPage.svelte";

beforeEach(() => {
  pendingSettingsSection.set(null);
  settingsRoute.set({ category: DEFAULT_CATEGORY });
  window.location.hash = "";
});

afterEach(() => {
  cleanup();
  pendingSettingsSection.set(null);
  settingsRoute.set({ category: DEFAULT_CATEGORY });
  window.location.hash = "";
});

describe("SettingsPage shell", () => {
  it('defaults to the "general" category when nothing is deep-linked', async () => {
    render(SettingsPage);
    await tick();
    const navGeneral = document.querySelector(
      '[data-testid="bg-cat-nav-general"]',
    ) as HTMLButtonElement | null;
    expect(navGeneral).not.toBeNull();
    expect(
      navGeneral!.classList.contains("bg-cat-nav__item--active"),
    ).toBe(true);
  });

  it('mirrors legacy "ai" pending section into the active category', async () => {
    pendingSettingsSection.set("ai");
    render(SettingsPage);
    await tick();

    expect(get(settingsRoute).category).toBe("ai");
    // The bridge clears the legacy store so later manual navigation
    // doesn't replay the deep-link.
    expect(get(pendingSettingsSection)).toBeNull();
  });

  it('maps the legacy "updates" section onto the "advanced" category', async () => {
    render(SettingsPage);
    await tick();

    pendingSettingsSection.set("updates");
    await tick();

    expect(get(settingsRoute).category).toBe("advanced");
    expect(get(pendingSettingsSection)).toBeNull();
  });

  it("aggregates descriptors from multiple categories into one index", async () => {
    const { getByTestId } = render(SettingsPage);
    await tick();

    const input = getByTestId("bg-search-input") as HTMLInputElement;

    // General exposes "Theme" and "Density" rows — the dropdown
    // surfaces both, proving the aggregation still works after
    // Appearance was folded into General.
    await fireEvent.input(input, { target: { value: "the" } });
    await tick();

    const themeMatch = document.querySelector(
      '[data-testid="settings-search-result-general.theme"]',
    );
    expect(themeMatch).not.toBeNull();

    await fireEvent.input(input, { target: { value: "density" } });
    await tick();

    const densityMatch = document.querySelector(
      '[data-testid="settings-search-result-general.density"]',
    );
    expect(densityMatch).not.toBeNull();
  });

  it("renders search results when the query matches the aggregated index", async () => {
    const { getByTestId } = render(SettingsPage);
    await tick();

    const input = getByTestId("bg-search-input") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "density" } });
    await tick();

    const results = getByTestId("settings-search-results");
    expect(results.textContent).toContain("Density");
  });

  it("renders the no-matches empty state when nothing matches", async () => {
    const { getByTestId } = render(SettingsPage);
    await tick();

    const input = getByTestId("bg-search-input") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "zzz-nope-zzz" } });
    await tick();

    const empty = getByTestId("settings-search-none");
    expect(empty).not.toBeNull();
    expect(empty.textContent?.trim().length).toBeGreaterThan(0);
  });

  it("clicking a result sets the route (category + anchor) and clears the query", async () => {
    const { getByTestId } = render(SettingsPage);
    await tick();

    const input = getByTestId("bg-search-input") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "density" } });
    await tick();

    const button = document.querySelector(
      '[data-testid="settings-search-result-general.density"]',
    ) as HTMLButtonElement | null;
    expect(button).not.toBeNull();

    await fireEvent.click(button!);
    await tick();

    const route = get(settingsRoute);
    expect(route.category).toBe("general");
    expect(route.anchor).toBe("density");

    // Clicking clears the query; the dropdown should no longer list
    // the match row.
    expect(input.value).toBe("");
  });

  it('legacy "appearance" deep-link falls back to the general category', async () => {
    render(SettingsPage);
    await tick();

    pendingSettingsSection.set("appearance");
    await tick();

    expect(get(settingsRoute).category).toBe("general");
    expect(get(pendingSettingsSection)).toBeNull();
  });
});
