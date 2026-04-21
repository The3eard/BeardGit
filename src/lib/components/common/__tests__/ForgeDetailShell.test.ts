/**
 * Unit tests for `ForgeDetailShell.svelte`.
 *
 * Asserts the four states every forge detail pane should cycle
 * through:
 *   - loading — shows a skeleton/spinner identifier.
 *   - error   — shows the banner, the (truncated) reason, and a
 *               retry button that invokes `onRetry` when clicked.
 *   - empty   — shows the provided empty message verbatim.
 *   - content — renders the default `content` snippet.
 *
 * Uses `createRawSnippet` so the test can drive the snippet slot
 * directly without needing a separate wrapper `.svelte` fixture.
 */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import ForgeDetailShell from "../ForgeDetailShell.svelte";

afterEach(() => cleanup());

function contentSnippet(html: string) {
  return createRawSnippet(() => ({ render: () => html }));
}

describe("ForgeDetailShell", () => {
  it("renders loading skeleton when loading=true", () => {
    render(ForgeDetailShell, {
      props: {
        loading: true,
        error: null,
        isEmpty: false,
        emptyMessage: "",
      },
    });
    expect(screen.getByTestId("forge-detail-loading")).toBeTruthy();
  });

  it("renders error banner + retry when error is set", async () => {
    const onRetry = vi.fn();
    render(ForgeDetailShell, {
      props: {
        loading: false,
        error: "boom",
        isEmpty: false,
        emptyMessage: "",
        onRetry,
      },
    });
    const retry = screen.getByRole("button", { name: /retry/i });
    await fireEvent.click(retry);
    expect(onRetry).toHaveBeenCalledOnce();
    expect(screen.getByText(/boom/)).toBeTruthy();
  });

  it("renders emptyMessage when isEmpty=true and no error", () => {
    render(ForgeDetailShell, {
      props: {
        loading: false,
        error: null,
        isEmpty: true,
        emptyMessage: "Nothing here",
      },
    });
    expect(screen.getByText("Nothing here")).toBeTruthy();
  });

  it("renders the content snippet when loaded and non-empty", () => {
    render(ForgeDetailShell, {
      props: {
        loading: false,
        error: null,
        isEmpty: false,
        emptyMessage: "",
        content: contentSnippet("<span>content-marker</span>"),
      },
    });
    expect(screen.getByText("content-marker")).toBeTruthy();
  });

  it("omits the retry button when onRetry is not provided", () => {
    render(ForgeDetailShell, {
      props: {
        loading: false,
        error: "boom",
        isEmpty: false,
        emptyMessage: "",
      },
    });
    expect(screen.queryByRole("button", { name: /retry/i })).toBeNull();
  });

  it("truncates long error reasons to ~80 chars with an ellipsis", () => {
    const longReason = "x".repeat(200);
    render(ForgeDetailShell, {
      props: {
        loading: false,
        error: longReason,
        isEmpty: false,
        emptyMessage: "",
      },
    });
    // The trimmed text should include the ellipsis and be shorter than
    // the original 200-char string.
    const reasonNode = screen.getByText(/x+…/);
    expect(reasonNode.textContent!.length).toBeLessThan(longReason.length);
    expect(reasonNode.textContent!.endsWith("…")).toBe(true);
  });
});
