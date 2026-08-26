/**
 * Unit tests for `AdvancedSettings.svelte` — the Updates section.
 *
 * These pin the *second* pre-install surface. The unsigned-build warning
 * (macOS Gatekeeper / Windows SmartScreen) has to be readable before the
 * download starts, because on Windows the NSIS installer replaces the
 * binary and kills the process — there is no post-install surface left to
 * render it in. The startup toast covers users who see the toast; this
 * panel covers everyone who turned the startup check off, clicked
 * "Later", or checked manually.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";

const osTypeMock = vi.fn(() => "macos");

vi.mock("@tauri-apps/plugin-os", () => ({
  type: () => osTypeMock(),
}));

vi.mock("$lib/api/tauri", () => ({
  getAutoCheckUpdates: vi.fn().mockResolvedValue(true),
  setAutoCheckUpdates: vi.fn(),
  openLogDirectory: vi.fn(),
  clearLayoutCache: vi.fn(),
}));

import AdvancedSettings from "../AdvancedSettings.svelte";
import { autoUpdateState } from "$lib/stores/autoUpdate";

beforeEach(() => {
  osTypeMock.mockReset();
  osTypeMock.mockImplementation(() => "macos");
});

afterEach(() => {
  autoUpdateState.set({ status: "idle" });
  cleanup();
});

/**
 * Render and let `onMount`'s async OS probe settle. `detectOs()` awaits a
 * dynamic `import()`, so a macrotask turn is needed before the reactive
 * helper line re-renders — `tick()` alone only flushes Svelte's queue.
 */
async function renderSettled() {
  const rendered = render(AdvancedSettings);
  await new Promise((resolve) => setTimeout(resolve, 0));
  await tick();
  return rendered;
}

describe("AdvancedSettings — unsigned-build notice", () => {
  it.each([
    ["macos", /gatekeeper/i],
    ["windows", /smartscreen/i],
  ] as const)(
    "shows the %s notice alongside the available version",
    async (os, re) => {
      osTypeMock.mockImplementation(() => os);
      autoUpdateState.set({ status: "available", availableVersion: "9.1.0" });

      const { container } = await renderSettled();

      expect(container.textContent).toContain("9.1.0");
      expect(container.textContent).toMatch(re);
    },
  );

  it("shows no notice on linux, which has no unsigned-binary gate", async () => {
    osTypeMock.mockImplementation(() => "linux");
    autoUpdateState.set({ status: "available", availableVersion: "9.1.0" });

    const { container } = await renderSettled();

    expect(container.textContent).toContain("9.1.0");
    expect(container.textContent).not.toMatch(/gatekeeper|smartscreen/i);
  });

  it("shows no notice while no update is available", async () => {
    autoUpdateState.set({ status: "up_to_date" });

    const { container } = await renderSettled();

    // Positive anchor first, so a component that threw or rendered
    // nothing fails here instead of passing the negative assertion.
    expect(container.textContent).toMatch(/up to date/i);
    expect(container.textContent).not.toMatch(/gatekeeper|smartscreen/i);
  });
});
