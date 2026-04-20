/**
 * Unit tests for `VersionSlot.svelte`.
 *
 * Validates:
 *   - Version label renders from `VITE_APP_VERSION` (or the fallback).
 *   - Update-available dot appears when `autoUpdateState.status ===
 *     "available"` and disappears otherwise.
 *   - Clicking fires `onNavigate("updates")`.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { UpdateState } from "$lib/stores/autoUpdate";

const mocks = vi.hoisted(() => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    autoUpdateState: writable<UpdateState>({ status: "idle" }),
  };
});

vi.mock("$lib/stores/autoUpdate", () => ({
  autoUpdateState: mocks.autoUpdateState,
}));

import VersionSlot from "../VersionSlot.svelte";

beforeEach(() => {
  mocks.autoUpdateState.set({ status: "idle" });
});

afterEach(() => cleanup());

describe("VersionSlot", () => {
  it("renders a version string prefixed with 'v'", async () => {
    const { getByTestId } = render(VersionSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    const el = getByTestId("statusbar-version-slot");
    expect(el.textContent ?? "").toMatch(/v\d+\.\d+\.\d+/);
  });

  it("does not show the update dot when status is idle", async () => {
    const { queryByTestId } = render(VersionSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    expect(queryByTestId("statusbar-version-update-dot")).toBeNull();
  });

  it("renders the update dot when status === 'available'", async () => {
    mocks.autoUpdateState.set({
      status: "available",
      availableVersion: "9.9.9",
    });
    const { getByTestId } = render(VersionSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    expect(getByTestId("statusbar-version-update-dot")).toBeTruthy();
    expect(
      getByTestId("statusbar-version-slot").getAttribute(
        "data-update-available",
      ),
    ).toBe("true");
  });

  it("fires onNavigate('updates') when clicked", async () => {
    const onNavigate = vi.fn();
    const { getByTestId } = render(VersionSlot, { props: { onNavigate } });
    await fireEvent.click(getByTestId("statusbar-version-slot"));
    expect(onNavigate).toHaveBeenCalledWith("updates");
  });
});
