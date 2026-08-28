/**
 * Unit tests for `AdvancedSettings.svelte` — the Updates section's
 * unsigned-build notice, and the Diagnostics log-level selector.
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

const getLogLevelMock = vi.fn(async () => "info");
const setLogLevelMock = vi.fn(async (_level: string) => {});

// `LOG_LEVELS` comes from the REAL module, not a stub. Hardcoding it here
// would make the "offers exactly the three levels" test assert against its
// own mock — it would stay green if the shipped list gained a level the
// backend rejects.
vi.mock("$lib/api/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("$lib/api/tauri")>();
  return {
    getAutoCheckUpdates: vi.fn().mockResolvedValue(true),
    setAutoCheckUpdates: vi.fn(),
    openLogDirectory: vi.fn(),
    clearLayoutCache: vi.fn(),
    getLogLevel: () => getLogLevelMock(),
    setLogLevel: (level: string) => setLogLevelMock(level),
    LOG_LEVELS: actual.LOG_LEVELS,
  };
});

import AdvancedSettings from "../AdvancedSettings.svelte";
import { autoUpdateState } from "$lib/stores/autoUpdate";

beforeEach(() => {
  osTypeMock.mockReset();
  osTypeMock.mockImplementation(() => "macos");
  getLogLevelMock.mockReset();
  getLogLevelMock.mockImplementation(async () => "info");
  setLogLevelMock.mockReset();
  setLogLevelMock.mockImplementation(async () => {});
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

describe("AdvancedSettings — log level selector", () => {
  it("offers exactly the three levels the backend accepts", async () => {
    const { getByTestId } = await renderSettled();

    // Must stay in sync with `storage::logging::LOG_LEVELS` — anything the
    // selector offers that `normalize_level` rejects is a dead option.
    const select = getByTestId("log-level-select") as HTMLSelectElement;
    expect([...select.options].map((o) => o.value)).toEqual([
      "error",
      "info",
      "debug",
    ]);
  });

  it("stays disabled until the persisted level has loaded", async () => {
    // A selector showing the default before hydration is indistinguishable
    // from one showing a loaded value, and a change made in that window
    // used to be silently overwritten by onMount.
    let release: (value: string) => void = () => {};
    getLogLevelMock.mockImplementation(
      () => new Promise<string>((resolve) => (release = resolve)),
    );

    const { getByTestId } = render(AdvancedSettings);
    await tick();
    expect(
      (getByTestId("log-level-select") as HTMLSelectElement).disabled,
    ).toBe(true);

    release("debug");
    await new Promise((resolve) => setTimeout(resolve, 0));
    await tick();

    const select = getByTestId("log-level-select") as HTMLSelectElement;
    expect(select.disabled).toBe(false);
    expect(select.value).toBe("debug");
  });

  it("hydrates from the persisted level", async () => {
    getLogLevelMock.mockImplementation(async () => "debug");

    const { getByTestId } = await renderSettled();

    expect((getByTestId("log-level-select") as HTMLSelectElement).value).toBe(
      "debug",
    );
  });

  it("falls back to info when the persisted value is unrecognized", async () => {
    // A hand-edited settings.json shouldn't leave the selector showing a
    // level the backend would reject.
    getLogLevelMock.mockImplementation(async () => "loud");

    const { getByTestId } = await renderSettled();

    expect((getByTestId("log-level-select") as HTMLSelectElement).value).toBe(
      "info",
    );
  });

  it("stays usable when reading the persisted level fails", async () => {
    // The gate must lift on the rejection path too. Without this, moving
    // `logLevelReady = true` out of the `.finally` leaves the selector
    // permanently disabled — and every other test here still passes.
    getLogLevelMock.mockImplementation(async () => {
      throw new Error("ipc unavailable");
    });

    const { getByTestId } = await renderSettled();

    const select = getByTestId("log-level-select") as HTMLSelectElement;
    expect(select.disabled).toBe(false);
    expect(select.value).toBe("info");
  });

  it("does not gate the selector behind the auto-check load", async () => {
    // These are independent IPC calls; a hang in one must not disable the
    // other's control.
    const tauri = await import("$lib/api/tauri");
    vi.mocked(tauri.getAutoCheckUpdates).mockImplementation(
      () => new Promise(() => {}),
    );

    const { getByTestId } = await renderSettled();

    expect(
      (getByTestId("log-level-select") as HTMLSelectElement).disabled,
    ).toBe(false);
  });

  it("persists the new level on change", async () => {
    const { getByTestId } = await renderSettled();
    const select = getByTestId("log-level-select") as HTMLSelectElement;

    select.value = "debug";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    await tick();

    expect(setLogLevelMock).toHaveBeenCalledWith("debug");
  });

  it("reverts the selector when the backend rejects the level", async () => {
    setLogLevelMock.mockImplementation(async () => {
      throw new Error("nope");
    });
    const { getByTestId } = await renderSettled();
    const select = getByTestId("log-level-select") as HTMLSelectElement;

    select.value = "debug";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve, 0));
    await tick();

    // The selector must never claim a level that didn't take effect.
    expect(
      (getByTestId("log-level-select") as HTMLSelectElement).value,
    ).toBe("info");
  });
});
