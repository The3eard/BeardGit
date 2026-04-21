/**
 * Unit tests for `Dialog.svelte`.
 *
 * Verifies:
 * - Renders backdrop + dialog when `open=true`.
 * - Renders nothing when `open=false`.
 * - Pressing `Escape` fires `onClose` and closes the dialog.
 * - Clicking the backdrop fires `onClose` and closes the dialog.
 * - The focus trap wraps `Tab` / `Shift+Tab` around the dialog's
 *   focusable children.
 * - Focus is restored to the previously focused element after close.
 */

import { describe, expect, it, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import { createRawSnippet, tick } from "svelte";
import Dialog from "../Dialog.svelte";

afterEach(() => cleanup());

function bodySnippet(html: string) {
  return createRawSnippet(() => ({ render: () => html }));
}

describe("Dialog", () => {
  it("renders when open=true", () => {
    const { queryByTestId } = render(Dialog, {
      props: { open: true, title: "Test" },
    });
    expect(queryByTestId("bg-dialog")).toBeTruthy();
    expect(queryByTestId("bg-dialog-backdrop")).toBeTruthy();
  });

  it("renders nothing when open=false", () => {
    const { queryByTestId } = render(Dialog, {
      props: { open: false, title: "Test" },
    });
    expect(queryByTestId("bg-dialog")).toBeNull();
    expect(queryByTestId("bg-dialog-backdrop")).toBeNull();
  });

  it("fires onClose when Escape is pressed", async () => {
    const onClose = vi.fn();
    render(Dialog, {
      props: { open: true, title: "Test", onClose },
    });
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("fires onClose when the backdrop is clicked", async () => {
    const onClose = vi.fn();
    const { getByTestId } = render(Dialog, {
      props: { open: true, title: "Test", onClose },
    });
    await fireEvent.click(getByTestId("bg-dialog-backdrop"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("wraps Tab at the end of the focusable children", async () => {
    const { container } = render(Dialog, {
      props: {
        open: true,
        title: "Focus trap",
        children: bodySnippet(
          `<div><button data-testid="first">First</button><button data-testid="second">Second</button><button data-testid="last">Last</button></div>`,
        ),
      },
    });
    await tick();

    const first = container.querySelector(
      '[data-testid="first"]',
    ) as HTMLButtonElement;
    const last = container.querySelector(
      '[data-testid="last"]',
    ) as HTMLButtonElement;
    expect(first).toBeTruthy();
    expect(last).toBeTruthy();

    last.focus();
    expect(document.activeElement).toBe(last);
    await fireEvent.keyDown(window, { key: "Tab" });
    expect(document.activeElement).toBe(first);
  });

  it("wraps Shift+Tab at the start of the focusable children", async () => {
    const { container } = render(Dialog, {
      props: {
        open: true,
        title: "Focus trap",
        children: bodySnippet(
          `<div><button data-testid="first">First</button><button data-testid="last">Last</button></div>`,
        ),
      },
    });
    await tick();

    const first = container.querySelector(
      '[data-testid="first"]',
    ) as HTMLButtonElement;
    const last = container.querySelector(
      '[data-testid="last"]',
    ) as HTMLButtonElement;
    first.focus();
    expect(document.activeElement).toBe(first);
    await fireEvent.keyDown(window, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(last);
  });

  it("restores focus to the previously focused element on close", async () => {
    const trigger = document.createElement("button");
    trigger.textContent = "Open";
    document.body.appendChild(trigger);
    trigger.focus();
    expect(document.activeElement).toBe(trigger);

    const { rerender } = render(Dialog, {
      props: { open: true, title: "Test" },
    });
    await tick();

    await rerender({ open: false, title: "Test" });
    await tick();

    expect(document.activeElement).toBe(trigger);
    trigger.remove();
  });
});
