/**
 * Unit test for the AI Settings provider icon — Phase 6 of Spec 2.
 *
 * Asserts that each `.provider-row` renders the shared
 * `ProviderIcon` component (an `<img class="provider-icon">`) instead
 * of the old nerd-font glyph span. Keeps AI Settings aligned with the
 * rest of the session-list / statusbar surfaces that already show the
 * native Anthropic / OpenAI / OpenCode logos.
 */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";

vi.mock("$lib/stores/ai", async () => {
  const { writable } = await import("svelte/store");
  return {
    aiProviders: writable([
      { kind: "claude_code", binary_path: "/x", version: "1" },
    ]),
    preferredAiProvider: writable("claude_code"),
    detectAiProviders: vi.fn(),
    setPreferredProvider: vi.fn(),
    loadPreferredProvider: vi.fn(),
  };
});
vi.mock("$lib/api/tauri", () => ({
  aiBackgroundGetSettings: vi.fn().mockResolvedValue({
    worktree_root: null,
    concurrency_cap: 3,
    auto_accept_permissions: false,
  }),
  aiBackgroundSetSettings: vi.fn(),
}));

import AiSettings from "../AiSettings.svelte";

afterEach(() => cleanup());

describe("AiSettings provider icon", () => {
  it("renders ProviderIcon in each provider row", async () => {
    const { container } = render(AiSettings);
    await tick();
    const rows = container.querySelectorAll(".provider-row");
    expect(rows.length).toBeGreaterThan(0);
    rows.forEach((row) => {
      expect(row.querySelector("img.provider-icon")).toBeTruthy();
    });
  });

  // Lockdown — Spec 4 Phase 5 re-verifies Spec 2's Phase 6 landing so a
  // future refactor can't silently regress back to the old nerd-font span.
  it("renders exactly one ProviderIcon per ALL_KINDS row (3 total)", async () => {
    const { container } = render(AiSettings);
    await tick();
    const rows = container.querySelectorAll(".provider-row");
    expect(rows.length).toBe(3);
    const icons = container.querySelectorAll("img.provider-icon");
    expect(icons.length).toBe(3);
  });

  it("never resurrects the legacy `.provider-icon.nf` nerd-font span", async () => {
    const { container } = render(AiSettings);
    await tick();
    // Old markup was `<span class="provider-icon nf">{icon}</span>` —
    // any surviving <span class="provider-icon"> would be the regression.
    const legacySpans = container.querySelectorAll("span.provider-icon");
    expect(legacySpans.length).toBe(0);
  });

  it("passes the row's provider kind to ProviderIcon (alt text check)", async () => {
    const { container } = render(AiSettings);
    await tick();
    const alts = Array.from(
      container.querySelectorAll<HTMLImageElement>("img.provider-icon"),
    ).map((img) => img.alt);
    expect(alts).toEqual([
      "claude_code icon",
      "codex icon",
      "open_code icon",
    ]);
  });

  // Spec 4 Phase 6 — the providers Card no longer carries a broken-glyph
  // refresh button in its `actions` snippet. Detection re-runs on mount
  // so the button is redundant; its `\uF021` glyph was also missing from
  // the shipped Nerd Font, rendering a blank square.
  it("does not render any button in the providers Card `actions` slot", async () => {
    const { container } = render(AiSettings);
    await tick();
    const actionButtons = container.querySelectorAll(
      ".bg-card__actions button",
    );
    expect(actionButtons.length).toBe(0);
  });
});
