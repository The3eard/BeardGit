import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  forgeEnabled,
  providerStatus,
  hasActiveProvider,
  loadForgeEnabled,
  setForgeEnabled,
  tryAutoConnect,
} from "./provider";
import type { ProviderStatusResponse } from "$lib/types";

const connected: ProviderStatusResponse = {
  providers: [
    {
      kind: "github",
      instance_url: "https://api.github.com",
      user: {
        id: 1,
        username: "me",
        display_name: "Me",
        email: null,
        avatar_url: null,
        profile_url: "https://github.com/me",
      },
      project_name: "me/repo",
    },
  ],
  active_index: 0,
};

beforeEach(() => {
  mockInvoke.mockReset();
  forgeEnabled.set(true);
  providerStatus.set({ providers: [], active_index: null });
});

describe("forge master switch", () => {
  it("loadForgeEnabled mirrors the persisted value", async () => {
    mockInvoke.mockResolvedValue(false);
    await loadForgeEnabled();
    expect(mockInvoke).toHaveBeenCalledWith("get_forge_enabled");
    expect(get(forgeEnabled)).toBe(false);
  });

  it("an unmocked / undefined answer keeps the forge on", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await loadForgeEnabled();
    expect(get(forgeEnabled)).toBe(true);
  });

  it("tryAutoConnect does not touch the backend while the switch is off", async () => {
    forgeEnabled.set(false);
    await tryAutoConnect();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("turning the switch off persists and drops the connections", async () => {
    providerStatus.set(connected);
    expect(get(hasActiveProvider)).toBe(true);
    mockInvoke.mockResolvedValue(undefined);

    await setForgeEnabled(false);

    expect(mockInvoke).toHaveBeenCalledWith("set_forge_enabled", { enabled: false });
    expect(get(forgeEnabled)).toBe(false);
    expect(get(providerStatus)).toEqual({ providers: [], active_index: null });
    expect(get(hasActiveProvider)).toBe(false);
  });

  it("turning the switch on persists and auto-connects", async () => {
    forgeEnabled.set(false);
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "try_auto_connect") return [];
      if (cmd === "get_provider_status") return connected;
      return undefined;
    });

    await setForgeEnabled(true);

    expect(mockInvoke).toHaveBeenCalledWith("set_forge_enabled", { enabled: true });
    expect(mockInvoke).toHaveBeenCalledWith("try_auto_connect");
    expect(get(providerStatus)).toEqual(connected);
  });

  it("reverts the store when persisting fails", async () => {
    mockInvoke.mockRejectedValue(new Error("disk"));
    await expect(setForgeEnabled(false)).rejects.toThrow("disk");
    expect(get(forgeEnabled)).toBe(true);
  });
});
