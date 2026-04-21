/**
 * Unit tests for `StatusBar.svelte` deep-link wiring.
 *
 * StatusBar maps each slot's logical target (`"ai"` / `"integrations"` /
 * `"updates"`) onto a concrete Settings sub-section id, stashes it in
 * `pendingSettingsSection`, and flips the top-level view to
 * `"settings"`. These tests simulate the slot click by firing the real
 * button in each slot and asserting the resulting store writes.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get, writable } from "svelte/store";
import {
  activeViewStore,
  pendingSettingsSection,
} from "$lib/stores/navigation";
import type { AiProviderKind, ConnectedProvider } from "$lib/types";

// Mock the stores the slot components pull in so we can seed realistic
// data without touching Tauri.
const mocks = vi.hoisted(() => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable: w } = require("svelte/store") as typeof import("svelte/store");
  const githubProvider = {
    kind: "github",
    instance_url: "https://github.com",
    user: { username: "someone", avatar_url: null },
  } as ConnectedProvider;
  return {
    preferredAiProvider: w<AiProviderKind | null>("claude_code"),
    providerStatus: w<{
      providers: ConnectedProvider[];
      active_index: number | null;
    }>({
      providers: [githubProvider],
      active_index: 0,
    }),
    // ForgeSlot now consumes `projectProvider` directly — seed it with
    // the same GitHub provider so the forge pill still renders.
    projectProvider: w<
      | { kind: "github" | "gitlab"; provider: ConnectedProvider }
      | null
    >({ kind: "github", provider: githubProvider }),
    autoUpdateState: w({ status: "idle" } as unknown as Record<string, unknown>),
  };
});

vi.mock("$lib/stores/ai", () => ({
  preferredAiProvider: mocks.preferredAiProvider,
}));

vi.mock("$lib/stores/provider", () => ({
  providerStatus: mocks.providerStatus,
  projectProvider: mocks.projectProvider,
}));

vi.mock("$lib/stores/autoUpdate", () => ({
  autoUpdateState: mocks.autoUpdateState,
  // `VersionSlot` only reads `autoUpdateState` but the module exports more.
  // No other field is consulted by the slot on render.
  AUTO_UPDATE_TASK_ID: "auto-update",
}));

vi.mock("$lib/stores/tasks", () => ({
  activeTaskCount: writable(0),
  anyRunning: writable(false),
  latestEntry: writable(null),
  hasUnseenError: writable(false),
}));

vi.mock("$lib/stores/tasksPopover", () => ({
  toggleTasksPopover: vi.fn(),
}));

// The network slot renders nothing while online, which is the default in
// JSDOM (`navigator.onLine === true`). No mock needed — we just don't
// touch it.

import StatusBar from "../StatusBar.svelte";

beforeEach(() => {
  pendingSettingsSection.set(null);
  activeViewStore.set("graph");
});

afterEach(() => {
  cleanup();
  pendingSettingsSection.set(null);
  activeViewStore.set("graph");
});

describe("StatusBar.onNavigate mapping", () => {
  it('AI slot click → pendingSettingsSection = "ai" and view = "settings"', async () => {
    const { getByTestId } = render(StatusBar);
    await fireEvent.click(getByTestId("statusbar-ai-slot"));
    expect(get(pendingSettingsSection)).toBe("ai");
    expect(get(activeViewStore)).toBe("settings");
  });

  it('Forge pill click → pendingSettingsSection = "connection" (integrations → connection)', async () => {
    const { getAllByTestId } = render(StatusBar);
    const pill = getAllByTestId("statusbar-forge-pill")[0];
    await fireEvent.click(pill);
    expect(get(pendingSettingsSection)).toBe("connection");
    expect(get(activeViewStore)).toBe("settings");
  });

  it('Version slot click → pendingSettingsSection = "updates"', async () => {
    const { getByTestId } = render(StatusBar);
    await fireEvent.click(getByTestId("statusbar-version-slot"));
    expect(get(pendingSettingsSection)).toBe("updates");
    expect(get(activeViewStore)).toBe("settings");
  });
});
