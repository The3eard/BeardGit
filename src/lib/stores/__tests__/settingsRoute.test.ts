/**
 * Unit tests for `settingsRoute.ts` — the Settings deep-link helper.
 *
 * These run in jsdom so `location.hash` is a real (mutable) property
 * and dispatching a `hashchange` event exercises the listener just
 * like the browser would.
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { get } from "svelte/store";
import {
  DEFAULT_CATEGORY,
  bindPendingSectionBridge,
  initSettingsRouteSync,
  seedFromLocation,
  setCategory,
  settingsRoute,
} from "../settingsRoute";
import { pendingSettingsSection } from "../navigation";

beforeEach(() => {
  // Reset URL + store between tests so assertions don't leak state.
  window.location.hash = "";
  settingsRoute.set({ category: DEFAULT_CATEGORY });
  pendingSettingsSection.set(null);
});

afterEach(() => {
  window.location.hash = "";
  settingsRoute.set({ category: DEFAULT_CATEGORY });
  pendingSettingsSection.set(null);
});

describe("settingsRoute store", () => {
  it("setCategory() updates the store and the URL hash", () => {
    setCategory("ai");
    expect(get(settingsRoute).category).toBe("ai");
    expect(window.location.hash).toBe("#ai");
  });

  it("setCategory() supports an anchor and serialises it", () => {
    setCategory("appearance", "theme");
    expect(get(settingsRoute)).toEqual({
      category: "appearance",
      anchor: "theme",
    });
    expect(window.location.hash).toBe("#appearance.theme");
  });

  it("unknown slugs fall back to the default category", () => {
    setCategory("does-not-exist");
    expect(get(settingsRoute).category).toBe(DEFAULT_CATEGORY);
  });

  it("legacy section ids map to the new category (connection → integrations)", () => {
    setCategory("connection");
    expect(get(settingsRoute).category).toBe("integrations");
  });

  it("legacy section ids map to the new category (updates → advanced)", () => {
    setCategory("updates");
    expect(get(settingsRoute).category).toBe("advanced");
  });

  it("seedFromLocation() reads an existing hash on mount", () => {
    window.location.hash = "#git";
    seedFromLocation();
    expect(get(settingsRoute).category).toBe("git");
  });

  it("hashchange events update the store", () => {
    const teardown = initSettingsRouteSync();
    window.location.hash = "#ai";
    window.dispatchEvent(new HashChangeEvent("hashchange"));
    expect(get(settingsRoute).category).toBe("ai");
    teardown();
  });

  it("empty hash resolves to the default category", () => {
    window.location.hash = "";
    seedFromLocation();
    expect(get(settingsRoute).category).toBe(DEFAULT_CATEGORY);
  });

  it("bindPendingSectionBridge mirrors + clears the legacy store", () => {
    const unsub = bindPendingSectionBridge();
    pendingSettingsSection.set("ai");
    expect(get(settingsRoute).category).toBe("ai");
    expect(get(pendingSettingsSection)).toBeNull();
    unsub();
  });

  it("bindPendingSectionBridge maps updates → advanced", () => {
    const unsub = bindPendingSectionBridge();
    pendingSettingsSection.set("updates");
    expect(get(settingsRoute).category).toBe("advanced");
    expect(get(pendingSettingsSection)).toBeNull();
    unsub();
  });
});
