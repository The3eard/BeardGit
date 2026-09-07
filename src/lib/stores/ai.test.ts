import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  aiEnabled,
  aiProviders,
  aiProvidersDetecting,
  hasAiProvider,
  detectAiProviders,
  loadAiEnabled,
  setAiEnabled,
} from "./ai";
import type { AvailableAiProvider } from "$lib/types";

const claude: AvailableAiProvider = {
  kind: "claude_code",
  binary_path: "/usr/local/bin/claude",
  version: "1.0.0",
};

beforeEach(() => {
  mockInvoke.mockReset();
  aiEnabled.set(true);
  aiProviders.set([]);
  aiProvidersDetecting.set(true);
});

describe("AI master switch", () => {
  it("hasAiProvider is false while the switch is off even with providers detected", () => {
    aiProviders.set([claude]);
    expect(get(hasAiProvider)).toBe(true);
    aiEnabled.set(false);
    expect(get(hasAiProvider)).toBe(false);
  });

  it("detectAiProviders probes nothing while the switch is off", async () => {
    aiEnabled.set(false);
    aiProviders.set([claude]);
    await detectAiProviders();
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(get(aiProviders)).toEqual([]);
    expect(get(aiProvidersDetecting)).toBe(false);
  });

  it("detectAiProviders probes when the switch is on", async () => {
    mockInvoke.mockImplementation(async (cmd: string) =>
      cmd === "ai_get_providers" ? [claude] : undefined,
    );
    await detectAiProviders();
    expect(mockInvoke).toHaveBeenCalledWith("ai_refresh_detection");
    expect(get(aiProviders)).toEqual([claude]);
  });

  it("loadAiEnabled mirrors the persisted value", async () => {
    mockInvoke.mockResolvedValue(false);
    await loadAiEnabled();
    expect(mockInvoke).toHaveBeenCalledWith("get_ai_enabled");
    expect(get(aiEnabled)).toBe(false);
  });

  it("turning the switch off persists and drops detected providers", async () => {
    aiProviders.set([claude]);
    mockInvoke.mockResolvedValue(undefined);
    await setAiEnabled(false);
    expect(mockInvoke).toHaveBeenCalledWith("set_ai_enabled", { enabled: false });
    expect(get(aiEnabled)).toBe(false);
    expect(get(aiProviders)).toEqual([]);
  });

  it("turning the switch on persists and re-runs detection", async () => {
    aiEnabled.set(false);
    mockInvoke.mockImplementation(async (cmd: string) =>
      cmd === "ai_get_providers" ? [claude] : undefined,
    );
    await setAiEnabled(true);
    expect(mockInvoke).toHaveBeenCalledWith("set_ai_enabled", { enabled: true });
    expect(mockInvoke).toHaveBeenCalledWith("ai_refresh_detection");
    expect(get(aiProviders)).toEqual([claude]);
  });

  it("reverts the store when persisting fails", async () => {
    mockInvoke.mockRejectedValue(new Error("disk"));
    await expect(setAiEnabled(false)).rejects.toThrow("disk");
    expect(get(aiEnabled)).toBe(true);
  });
});
