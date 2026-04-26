/**
 * Unit tests for IconButton.svelte.
 *
 * Asserts the rendered `<button>` carries the right tooltip / a11y wiring,
 * fires onclick when enabled, suppresses onclick while disabled or
 * loading, and swaps the glyph for a spinner during loading.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import IconButton from "../IconButton.svelte";

afterEach(() => cleanup());

describe("IconButton", () => {
  it("renders the icon glyph and exposes description as title + aria-label", () => {
    const { container } = render(IconButton, {
      props: { icon: "", description: "New branch" },
    });
    const btn = container.querySelector("button")!;
    expect(btn.getAttribute("title")).toBe("New branch");
    expect(btn.getAttribute("aria-label")).toBe("New branch");
    expect(container.querySelector(".ic-btn__glyph")?.textContent).toBe("");
  });

  it("ariaLabel overrides the description-derived aria-label", () => {
    const { container } = render(IconButton, {
      props: {
        icon: "",
        description: "Close panel",
        ariaLabel: "Close",
      },
    });
    const btn = container.querySelector("button")!;
    expect(btn.getAttribute("aria-label")).toBe("Close");
    // The native tooltip still reflects the description.
    expect(btn.getAttribute("title")).toBe("Close panel");
  });

  it("fires onclick when enabled", async () => {
    const onclick = vi.fn();
    const { container } = render(IconButton, {
      props: { icon: "", description: "Refresh", onclick },
    });
    await fireEvent.click(container.querySelector("button")!);
    expect(onclick).toHaveBeenCalledTimes(1);
  });

  it("does not fire onclick when disabled", async () => {
    const onclick = vi.fn();
    const { container } = render(IconButton, {
      props: {
        icon: "",
        description: "Refresh",
        disabled: true,
        onclick,
      },
    });
    const btn = container.querySelector("button")!;
    expect(btn.disabled).toBe(true);
    await fireEvent.click(btn);
    expect(onclick).not.toHaveBeenCalled();
  });

  it("does not fire onclick while loading", async () => {
    const onclick = vi.fn();
    const { container } = render(IconButton, {
      props: {
        icon: "",
        description: "Refresh",
        loading: true,
        onclick,
      },
    });
    const btn = container.querySelector("button")!;
    expect(btn.disabled).toBe(true);
    expect(btn.getAttribute("aria-busy")).toBe("true");
    await fireEvent.click(btn);
    expect(onclick).not.toHaveBeenCalled();
  });

  it("renders the spinner glyph in place of the icon while loading", () => {
    const { container } = render(IconButton, {
      props: { icon: "", description: "Refresh", loading: true },
    });
    expect(container.querySelector(".ic-btn__glyph")?.textContent).toBe("");
  });

  it("forwards testid to the underlying button", () => {
    const { container } = render(IconButton, {
      props: {
        icon: "",
        description: "Refresh",
        testid: "my-refresh-btn",
      },
    });
    expect(container.querySelector('[data-testid="my-refresh-btn"]')).not.toBeNull();
  });

  it("applies the danger tone class when tone='danger'", () => {
    const { container } = render(IconButton, {
      props: {
        icon: "",
        description: "Delete asset",
        tone: "danger",
      },
    });
    expect(
      container.querySelector("button")?.classList.contains("ic-btn--danger"),
    ).toBe(true);
  });
});
