/**
 * Unit tests for `UpdateSettings.svelte`.
 *
 * Exercises the public surface of the Settings → Updates panel:
 *
 * - renders the current app version from `VITE_APP_VERSION`
 * - the **Check for updates** button delegates to `checkForUpdates()`
 *   and flows status text onto the panel
 * - the auto-check toggle reflects the persisted preference and writes
 *   back via `setAutoCheckUpdates()`
 */

import { describe, expect, it, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup, waitFor } from "@testing-library/svelte";
import UpdateSettings from "../UpdateSettings.svelte";
import { mockInvokeResponse, invokeMock } from "../../../../test/setup";

// The updater plugin is mocked so `checkForUpdates()` resolves without
// network IO. We expose a mutable holder so each test can swap the
// resolved value.
const checkMock = vi.fn();
vi.mock("@tauri-apps/plugin-updater", () => ({
  check: () => checkMock(),
}));
vi.mock("@tauri-apps/plugin-os", () => ({
  type: () => "linux",
}));
vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: vi.fn(async () => {}),
}));

afterEach(() => {
  cleanup();
  checkMock.mockReset();
});

describe("UpdateSettings", () => {
  it("renders the current version from VITE_APP_VERSION", () => {
    mockInvokeResponse("get_auto_check_updates", true);
    const { getByTestId } = render(UpdateSettings);

    const badge = getByTestId("update-current-version");
    // Vite's define wires this at build time. In the vitest run it is
    // undefined, so the component falls back to "0.0.0". Accept either
    // a version-looking string or the fallback.
    expect(badge.textContent?.trim()).toMatch(/^\d+\.\d+\.\d+/);
  });

  it("drives checkForUpdates when the button is clicked", async () => {
    mockInvokeResponse("get_auto_check_updates", true);
    checkMock.mockResolvedValueOnce(null);
    const { getByTestId } = render(UpdateSettings);

    await fireEvent.click(getByTestId("update-check-btn"));

    await waitFor(() => expect(checkMock).toHaveBeenCalledTimes(1));
    const status = getByTestId("update-status");
    expect(status.textContent?.toLowerCase()).toMatch(/up to date|al día/);
  });

  it("writes the auto-check preference when the toggle changes", async () => {
    mockInvokeResponse("get_auto_check_updates", true);
    const { getByTestId } = render(UpdateSettings);

    const toggle = getByTestId("update-auto-toggle") as HTMLInputElement;
    await waitFor(() => expect(toggle.checked).toBe(true));

    // Uncheck
    await fireEvent.click(toggle);
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "set_auto_check_updates",
        expect.objectContaining({ enabled: false }),
      );
    });
  });

  it("surfaces 'update available' state with an Install button", async () => {
    mockInvokeResponse("get_auto_check_updates", true);
    checkMock.mockResolvedValueOnce({
      version: "7.7.7",
      currentVersion: "0.0.0",
      body: "Notes",
      available: true,
      rawJson: {},
      async downloadAndInstall() {},
      async close() {},
    });
    const { getByTestId, queryByTestId } = render(UpdateSettings);

    await fireEvent.click(getByTestId("update-check-btn"));

    await waitFor(() => expect(queryByTestId("update-install-btn")).not.toBeNull());
    await waitFor(() => {
      const status = getByTestId("update-status");
      expect(status.textContent).toContain("7.7.7");
    });
  });
});
