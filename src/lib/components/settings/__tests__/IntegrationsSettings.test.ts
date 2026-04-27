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
  const providerStatus = writable<{
    providers: Array<{
      kind: "github" | "gitlab";
      instance_url: string;
      user: {
        id: number;
        username: string;
        display_name: string;
        email: string | null;
        avatar_url: string | null;
        profile_url: string;
      };
      project_name: string | null;
    }>;
    active_index: number | null;
  }>({ providers: [], active_index: null });
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
import { providerStatus } from "$lib/stores/provider";
import { cliCheckAuthStatus } from "$lib/api/tauri";
import type { Writable } from "svelte/store";

// Narrowed writable view of the mocked providerStatus store so tests
// can push a synthetic providers list per-scenario. The shape mirrors
// `ProviderStatusResponse` but is typed locally to avoid coupling the
// test to the real store's generic signature.
const providerStatusWritable = providerStatus as unknown as Writable<{
  providers: Array<{
    kind: "github" | "gitlab";
    instance_url: string;
    user: {
      id: number;
      username: string;
      display_name: string;
      email: string | null;
      avatar_url: string | null;
      profile_url: string;
    };
    project_name: string | null;
  }>;
  active_index: number | null;
}>;

const cliCheckAuthStatusMock = vi.mocked(cliCheckAuthStatus);

const DEFAULT_CLI_STATUSES = [
  {
    tool: "gh" as const,
    installed: true,
    authenticated: true,
    username: "octocat",
    error: null,
  },
  {
    tool: "glab" as const,
    installed: true,
    authenticated: false,
    username: null,
    error: null,
  },
];

beforeEach(() => {
  // Reset store + CLI mock to a clean slate so tests don't leak state
  // into each other — the module-scoped `writable(...)` inside the
  // `vi.mock` block is shared across every `describe` run.
  providerStatusWritable.set({ providers: [], active_index: null });
  cliCheckAuthStatusMock.mockResolvedValue(DEFAULT_CLI_STATUSES);
});
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

describe("IntegrationsSettings — unified Connections section (Phase 8)", () => {
  it("renders exactly one .bg-card on the page (howto is not a card)", async () => {
    const { container } = render(IntegrationsSettings);
    await tick();

    const cards = container.querySelectorAll(".bg-card");
    expect(cards.length).toBe(1);
  });

  it("renders four rows — github, gitlab, gh, glab — each with testid", async () => {
    const { container } = render(IntegrationsSettings);
    await tick();

    for (const kind of ["github", "gitlab", "gh", "glab"]) {
      const row = container.querySelector(
        `[data-testid="integrations-row-${kind}"]`,
      );
      expect(row, `row integrations-row-${kind} missing`).not.toBeNull();
    }
  });

  it("each row contains a name, status label, and single action button", async () => {
    const { container } = render(IntegrationsSettings);
    await tick();

    for (const kind of ["github", "gitlab", "gh", "glab"]) {
      const row = container.querySelector(
        `[data-testid="integrations-row-${kind}"]`,
      )!;
      expect(
        row.querySelector('[data-role="name"]'),
        `row ${kind} name`,
      ).not.toBeNull();
      expect(
        row.querySelector('[data-role="status"]'),
        `row ${kind} status`,
      ).not.toBeNull();

      const buttons = row.querySelectorAll('[data-role="action"] button');
      expect(buttons.length, `row ${kind} action button count`).toBe(1);
    }
  });
});

describe("IntegrationsSettings — CLI row piggyback label (Phase 5)", () => {
  // The piggyback refinement on the gh/glab CLI rows: when the CLI is
  // authenticated as user X AND a connected provider PAT of the
  // matching kind is also user X, the CLI row's status label reads
  // "Connected · via {Provider} PAT" instead of the plain per-row
  // "Signed in as X" / "Connected". When usernames differ the label
  // falls through to the plain form — the row is telling the truth
  // about divergent identities rather than hiding the split.

  const githubProvider = (username: string) => ({
    kind: "github" as const,
    instance_url: "https://api.github.com",
    user: {
      id: 1,
      username,
      display_name: username,
      email: null,
      avatar_url: null,
      profile_url: `https://github.com/${username}`,
    },
    project_name: null,
  });

  it("gh row reads 'via GitHub PAT' when CLI + provider usernames match", async () => {
    cliCheckAuthStatusMock.mockResolvedValue([
      {
        tool: "gh",
        installed: true,
        authenticated: true,
        username: "alice",
        error: null,
      },
      {
        tool: "glab",
        installed: false,
        authenticated: false,
        username: null,
        error: null,
      },
    ]);
    providerStatusWritable.set({
      providers: [githubProvider("alice")],
      active_index: 0,
    });

    const { container } = render(IntegrationsSettings);
    // Two ticks: first resolves the `onMount` refresh, second lets
    // Svelte flush the derived `statusLabel` against the updated
    // `cliStatuses` state.
    await tick();
    await tick();

    const row = container.querySelector(
      '[data-testid="integrations-row-gh"]',
    )!;
    const status = row.querySelector('[data-role="status"]')!;
    expect(status.textContent).toContain("via");
    expect(status.textContent).toContain("GitHub");
    expect(status.textContent).not.toContain("alice");
  });

  it("gh row falls through to plain label when usernames differ", async () => {
    cliCheckAuthStatusMock.mockResolvedValue([
      {
        tool: "gh",
        installed: true,
        authenticated: true,
        username: "alice",
        error: null,
      },
      {
        tool: "glab",
        installed: false,
        authenticated: false,
        username: null,
        error: null,
      },
    ]);
    providerStatusWritable.set({
      providers: [githubProvider("bob")],
      active_index: 0,
    });

    const { container } = render(IntegrationsSettings);
    await tick();
    await tick();

    const row = container.querySelector(
      '[data-testid="integrations-row-gh"]',
    )!;
    const status = row.querySelector('[data-role="status"]')!;
    // Plain "Signed in as alice" form — definitely no "via".
    expect(status.textContent).toContain("alice");
    expect(status.textContent).not.toContain("via");
  });
});
