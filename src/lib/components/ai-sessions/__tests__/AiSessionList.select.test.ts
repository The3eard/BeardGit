/**
 * Regression test: clicking any row (including sessions without a
 * `background_status`) writes the session id into
 * `selectedBackgroundSessionId`, so the detail pane populates for every
 * row type.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";

vi.mock("$lib/stores/aiSessions", async () => {
  const { writable } = await import("svelte/store");
  return {
    mergedSessions: writable([
      {
        id: "plain",
        provider: "open_code",
        cwd: "/x",
        kind: "interactive",
        is_active: true,
        started_at: 0,
      },
    ]),
    sessionsLoading: writable(false),
    refreshSessions: vi.fn(),
    dismissSession: vi.fn(),
    startSessionListeners: vi.fn(),
    stopSessionListeners: vi.fn(),
  };
});

import AiSessionList from "../AiSessionList.svelte";
import { selectedBackgroundSessionId } from "$lib/stores/aiBackground";

afterEach(() => {
  cleanup();
  selectedBackgroundSessionId.set(null);
});

describe("AiSessionList selection", () => {
  it("selects any row, not just background runs", async () => {
    const { container } = render(AiSessionList);
    const row = container.querySelector(".session-item") as HTMLElement;
    expect(row).toBeTruthy();
    await fireEvent.click(row);
    expect(get(selectedBackgroundSessionId)).toBe("plain");
  });
});
