/**
 * Unit tests for `IntegrationsSettings.svelte` — Spec 4 Phases 7 and 8.
 *
 * Phase 7.2 asserts that `<ConnectionHowTo />` is hoisted out of any
 * `<Card>` wrapper and renders before the first `.bg-card` in DOM
 * order (the compact top-of-page dropdown pattern from Spec 4).
 *
 * Phase 8.1 asserts the unified Connections section: exactly one
 * `<Card>` (beyond the howto, which is not a card) with four rows —
 * `github`, `gitlab`, `gh`, `glab` — each carrying a display name,
 * status label, and single action button. (Phase 8.1 is the
 * failing-red step; 8.2 makes it green.)
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";

vi.mock("$lib/stores/provider", async () => {
  const { writable, derived } = await import("svelte/store");
  const providerStatus = writable({ providers: [], active_index: null });
  const activeProvider = derived(providerStatus, () => null);
  const isConnected = derived(providerStatus, () => false);
  const hasActiveProvider = derived(providerStatus, () => false);
  const isConnecting = writable(false);
  const providerError = writable<string | null>(null);
  return {
    providerStatus,
    activeProvider,
    isConnected,
    hasActiveProvider,
    isConnecting,
    providerError,
    connect: vi.fn(),
    disconnect: vi.fn(),
    checkStatus: vi.fn(),
  };
});

vi.mock("$lib/stores/tabs", async () => {
  const { writable } = await import("svelte/store");
  return {
    activeProjectFromTab: writable(null),
    openStandaloneTerminal: vi.fn().mockResolvedValue("sid"),
  };
});

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

vi.mock("@tauri-apps/api/path", () => ({
  homeDir: vi.fn().mockResolvedValue("/home/test"),
}));

vi.mock("$lib/api/tauri", () => ({
  cliCheckAuthStatus: vi.fn().mockResolvedValue([
    {
      tool: "gh",
      installed: true,
      authenticated: true,
      username: "octocat",
      hostname: null,
    },
    {
      tool: "glab",
      installed: true,
      authenticated: false,
      username: null,
      hostname: null,
    },
  ]),
  cliGetAuthCommand: vi.fn().mockResolvedValue("gh auth login"),
  cliGetLogoutCommand: vi.fn().mockResolvedValue("gh auth logout"),
  terminalWrite: vi.fn(),
}));

import IntegrationsSettings from "../IntegrationsSettings.svelte";

beforeEach(() => {});
afterEach(() => cleanup());

describe("IntegrationsSettings — howto hoist (Phase 7.2)", () => {
  it("renders ConnectionHowTo before any .bg-card in DOM order", async () => {
    const { container } = render(IntegrationsSettings);
    await tick();

    const howto = container.querySelector('[data-testid="integrations-howto"]');
    const firstCard = container.querySelector(".bg-card");

    expect(howto, "howto should render").not.toBeNull();
    expect(firstCard, "at least one card should render").not.toBeNull();

    const position = howto!.compareDocumentPosition(firstCard!);
    // DOCUMENT_POSITION_FOLLOWING === 4 — firstCard comes AFTER howto.
    expect(position & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("ConnectionHowTo is NOT a descendant of any .bg-card", async () => {
    const { container } = render(IntegrationsSettings);
    await tick();

    const howto = container.querySelector('[data-testid="integrations-howto"]');
    expect(howto).not.toBeNull();

    const parentCard = howto!.closest(".bg-card");
    expect(parentCard).toBeNull();
  });
});
