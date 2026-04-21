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
});
