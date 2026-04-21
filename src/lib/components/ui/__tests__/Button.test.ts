/**
 * Unit tests for `Button.svelte`.
 *
 * Verifies:
 * - Default variant + size render the expected BEM classes.
 * - The `variant` prop swaps the variant class.
 * - The `size` prop swaps the size class.
 * - `loading=true` shows a spinner, disables the button, and suppresses
 *   the `onclick` handler.
 * - `disabled=true` suppresses the `onclick` handler.
 * - A plain click with no special props fires the handler.
 * - The `icon` prop renders a leading glyph element.
 */

import { describe, expect, it, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import Button from "../Button.svelte";

afterEach(() => cleanup());

describe("Button", () => {
  it("renders with default variant (secondary) and size (md) classes", () => {
    const { container } = render(Button, { props: {} });
    const btn = container.querySelector("button")!;
    expect(btn).toBeTruthy();
    expect(btn.classList.contains("bg-btn--secondary")).toBe(true);
    expect(btn.classList.contains("bg-btn--md")).toBe(true);
    expect(btn.getAttribute("data-variant")).toBe("secondary");
    expect(btn.getAttribute("data-size")).toBe("md");
  });

  it("applies the correct class for each variant", () => {
    for (const variant of ["primary", "secondary", "ghost", "danger"] as const) {
      const { container, unmount } = render(Button, { props: { variant } });
      const btn = container.querySelector("button")!;
      expect(btn.classList.contains(`bg-btn--${variant}`)).toBe(true);
      unmount();
    }
  });

  it("applies the correct class for each size", () => {
    for (const size of ["sm", "md", "lg"] as const) {
      const { container, unmount } = render(Button, { props: { size } });
      const btn = container.querySelector("button")!;
      expect(btn.classList.contains(`bg-btn--${size}`)).toBe(true);
      unmount();
    }
  });

  it("fires onclick when enabled and not loading", async () => {
    const onclick = vi.fn();
    const { container } = render(Button, { props: { onclick } });
    const btn = container.querySelector("button")!;
    await fireEvent.click(btn);
    expect(onclick).toHaveBeenCalledTimes(1);
  });

  it("renders a spinner + disables button + suppresses onclick when loading", async () => {
    const onclick = vi.fn();
    const { container } = render(Button, {
      props: { loading: true, onclick },
    });
    const btn = container.querySelector("button")!;
    expect(btn.disabled).toBe(true);
    expect(btn.getAttribute("aria-busy")).toBe("true");
    expect(container.querySelector(".bg-btn__spinner")).toBeTruthy();
    await fireEvent.click(btn);
    expect(onclick).not.toHaveBeenCalled();
  });

  it("suppresses onclick when disabled", async () => {
    const onclick = vi.fn();
    const { container } = render(Button, {
      props: { disabled: true, onclick },
    });
    const btn = container.querySelector("button")!;
    expect(btn.disabled).toBe(true);
    await fireEvent.click(btn);
    expect(onclick).not.toHaveBeenCalled();
  });

  it("renders an icon glyph when icon prop is provided", () => {
    const { container } = render(Button, {
      props: { icon: "\uF021" },
    });
    const iconEl = container.querySelector(".bg-btn__icon");
    expect(iconEl).toBeTruthy();
    expect(iconEl!.textContent).toBe("\uF021");
  });
});
