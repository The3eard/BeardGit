/**
 * Unit tests for `SettingsPage.svelte`.
 *
 * Covers the deep-link bridge used by the statusbar slots (AI / Forge /
 * Version) to jump directly into a specific Settings sub-section
 * instead of landing on the default "Connection" tab. The signal is the
 * `pendingSettingsSection` store: when it holds a known section id the
 * page mirrors it into its local `activeSection` state and then clears
 * the store so subsequent manual navigations start fresh.
 *
 * Mocks of the heavy subcomponents (ProviderSetup, AiSettings,
 * UpdateSettings, …) are minimal Svelte components so we dodge the
 * Tauri IPC + OS detection their real implementations pull in — the
 * assertions target the nav buttons, not the panel content.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { get } from "svelte/store";
import Stub from "./__stubs__/Stub.svelte";
import { pendingSettingsSection } from "$lib/stores/navigation";

vi.mock("../../auth/ProviderSetup.svelte", () => ({ default: Stub }));
vi.mock("../AppearanceSettings.svelte", () => ({ default: Stub }));
vi.mock("../GitConfigSettings.svelte", () => ({ default: Stub }));
vi.mock("../AiSettings.svelte", () => ({ default: Stub }));
vi.mock("../UpdateSettings.svelte", () => ({ default: Stub }));
vi.mock("../CliAuthSection.svelte", () => ({ default: Stub }));
vi.mock("../ConnectionHowTo.svelte", () => ({ default: Stub }));

import SettingsPage from "../SettingsPage.svelte";

beforeEach(() => {
  pendingSettingsSection.set(null);
});

afterEach(() => {
  cleanup();
  pendingSettingsSection.set(null);
});

describe("SettingsPage deep-link bridge", () => {
  it('defaults to the "connection" section when no pending section is set', async () => {
    const { getByTestId } = render(SettingsPage);
    await tick();

    const navConnection = getByTestId("settings-nav-connection");
    expect(navConnection.classList.contains("active")).toBe(true);
  });

  it('mirrors "ai" into activeSection when the store holds it at mount', async () => {
    pendingSettingsSection.set("ai");
    const { getByTestId } = render(SettingsPage);
    await tick();

    const navAi = getByTestId("settings-nav-ai");
    expect(navAi.classList.contains("active")).toBe(true);
    // Bridge clears the store so a later manual click doesn't replay it.
    expect(get(pendingSettingsSection)).toBeNull();
  });

  it('mirrors "updates" into activeSection when the store updates while mounted', async () => {
    const { getByTestId } = render(SettingsPage);
    await tick();

    pendingSettingsSection.set("updates");
    await tick();

    const navUpdates = getByTestId("settings-nav-updates");
    expect(navUpdates.classList.contains("active")).toBe(true);
    expect(get(pendingSettingsSection)).toBeNull();
  });

  it("ignores unknown section ids (defensive — bad data shouldn't break the page)", async () => {
    const { getByTestId } = render(SettingsPage);
    await tick();

    pendingSettingsSection.set("not-a-real-section");
    await tick();

    // Stays on the current default.
    const navConnection = getByTestId("settings-nav-connection");
    expect(navConnection.classList.contains("active")).toBe(true);
    // Unknown id is not a valid "handled" signal — it's left alone so a
    // follow-up valid write still fires.
    expect(get(pendingSettingsSection)).toBe("not-a-real-section");
  });
});
