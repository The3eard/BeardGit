/**
 * Unit tests for `NetworkSlot.svelte`.
 *
 * The slot listens on `window.online` / `window.offline` events and
 * reads the initial `navigator.onLine` boolean. The tests simulate
 * those synthetic events and assert the DOM toggle.
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";
import NetworkSlot from "../NetworkSlot.svelte";

afterEach(() => cleanup());

beforeEach(() => {
  // Force navigator.onLine back to true for each test so the slot
  // starts hidden. `navigator.onLine` is writable on jsdom via
  // Object.defineProperty.
  Object.defineProperty(navigator, "onLine", {
    configurable: true,
    value: true,
  });
});

describe("NetworkSlot", () => {
  it("is hidden when online", async () => {
    const { queryByTestId } = render(NetworkSlot);
    await tick();
    expect(queryByTestId("statusbar-network-slot")).toBeNull();
  });

  it("renders when an 'offline' event fires", async () => {
    const { queryByTestId } = render(NetworkSlot);
    await tick();
    expect(queryByTestId("statusbar-network-slot")).toBeNull();

    Object.defineProperty(navigator, "onLine", {
      configurable: true,
      value: false,
    });
    window.dispatchEvent(new Event("offline"));
    await tick();

    const el = queryByTestId("statusbar-network-slot");
    expect(el).toBeTruthy();
    expect(el?.classList.contains("offline")).toBe(true);
  });

  it("hides again when the 'online' event fires", async () => {
    Object.defineProperty(navigator, "onLine", {
      configurable: true,
      value: false,
    });
    const { queryByTestId } = render(NetworkSlot);
    // Fire offline first so mount state + event both agree.
    window.dispatchEvent(new Event("offline"));
    await tick();
    expect(queryByTestId("statusbar-network-slot")).toBeTruthy();

    Object.defineProperty(navigator, "onLine", {
      configurable: true,
      value: true,
    });
    window.dispatchEvent(new Event("online"));
    await tick();
    expect(queryByTestId("statusbar-network-slot")).toBeNull();
  });
});
