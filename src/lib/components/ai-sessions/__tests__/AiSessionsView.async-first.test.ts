/**
 * Regression test: AiSessionsView must render <AiSessionList> without
 * awaiting the initial refresh. Before the async-first fix the tab
 * blocked on a network round-trip; the v2 rewrite keeps the same
 * contract but fans out to both `refreshConversations` (transcript
 * store) and `refreshAiBackgroundRuns` (bg-run store) in parallel.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { tick } from "svelte";

vi.mock("$lib/stores/aiBackground", async () => {
  const actual = await vi.importActual<object>("$lib/stores/aiBackground");
  return {
    ...actual,
    // Never resolves — proves the view doesn't await it.
    refreshAiBackgroundRuns: vi.fn(() => new Promise(() => {})),
    startAiBackgroundListeners: vi.fn(() => Promise.resolve()),
  };
});

vi.mock("$lib/stores/aiConversations", async () => {
  const actual = await vi.importActual<object>("$lib/stores/aiConversations");
  return {
    ...actual,
    // Never resolves either — same contract as the bg-run refresh.
    refreshConversations: vi.fn(() => new Promise(() => {})),
  };
});

import AiSessionsView from "../AiSessionsView.svelte";

afterEach(() => cleanup());

describe("AiSessionsView", () => {
  it("paints the list shell immediately without blocking on refresh", async () => {
    const { container } = render(AiSessionsView);
    await tick();
    // Shell class present — list rendered synchronously.
    expect(container.querySelector(".ai-sessions-view")).toBeTruthy();
  });
});
