/**
 * Unit tests for `AiSlot.svelte`.
 *
 * Covers the two render branches:
 *   - With a preferred provider → `ProviderBrandIcon` renders.
 *   - Without a preferred provider → grey dot + "AI" fallback.
 *
 * Click-navigation asserts the Settings section key ("ai") bubbles up.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import type { AiProviderKind } from "$lib/types";

const mocks = vi.hoisted(() => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    preferredAiProvider: writable<AiProviderKind | null>(null),
  };
});

vi.mock("$lib/stores/ai", () => ({
  preferredAiProvider: mocks.preferredAiProvider,
}));

import AiSlot from "../AiSlot.svelte";

beforeEach(() => {
  mocks.preferredAiProvider.set(null);
});

afterEach(() => cleanup());

describe("AiSlot", () => {
  it("renders the fallback label when no preferred provider", async () => {
    const { getByTestId } = render(AiSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    const slot = getByTestId("statusbar-ai-slot");
    expect(slot.getAttribute("data-has-provider")).toBe("false");
    expect(slot.textContent ?? "").toContain("AI");
  });

  it("renders the provider brand icon when preferred is set", async () => {
    mocks.preferredAiProvider.set("claude_code" as AiProviderKind);
    const { getByTestId, container } = render(AiSlot, {
      props: { onNavigate: vi.fn() },
    });
    await tick();
    const slot = getByTestId("statusbar-ai-slot");
    expect(slot.getAttribute("data-has-provider")).toBe("true");
    const svg = container.querySelector("svg.brand-icon");
    expect(svg).toBeTruthy();
  });

  it("calls onNavigate('ai') when clicked", async () => {
    const onNavigate = vi.fn();
    const { getByTestId } = render(AiSlot, { props: { onNavigate } });
    await fireEvent.click(getByTestId("statusbar-ai-slot"));
    expect(onNavigate).toHaveBeenCalledWith("ai");
  });
});
