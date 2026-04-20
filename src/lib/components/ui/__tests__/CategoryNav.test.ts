/**
 * Unit tests for `CategoryNav.svelte`.
 *
 * Verifies:
 * - Clicking an item selects it (emits onSelect + updates activeId).
 * - Pressing Enter on a focused item selects it.
 * - Pressing Space on a focused item selects it.
 * - ArrowDown / ArrowUp move focus between items.
 * - ArrowDown wraps from the last item to the first.
 * - ArrowUp wraps from the first item to the last.
 * - Home jumps to the first item; End jumps to the last.
 */

import { describe, expect, it, vi, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import CategoryNav from "../CategoryNav.svelte";

afterEach(() => cleanup());

const CATEGORIES = [
  { id: "general", label: "General" },
  { id: "appearance", label: "Appearance" },
  { id: "ai", label: "AI" },
];

describe("CategoryNav", () => {
  it("selects a category on click", async () => {
    const onSelect = vi.fn();
    const { getByTestId } = render(CategoryNav, {
      props: {
        categories: CATEGORIES,
        activeId: "general",
        onSelect,
      },
    });

    await fireEvent.click(getByTestId("bg-cat-nav-appearance"));
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith("appearance");
  });

  it("selects a category on Enter", async () => {
    const onSelect = vi.fn();
    const { getByTestId } = render(CategoryNav, {
      props: {
        categories: CATEGORIES,
        activeId: "general",
        onSelect,
      },
    });

    const item = getByTestId("bg-cat-nav-appearance");
    item.focus();
    await fireEvent.keyDown(item, { key: "Enter" });
    expect(onSelect).toHaveBeenCalledWith("appearance");
  });

  it("selects a category on Space", async () => {
    const onSelect = vi.fn();
    const { getByTestId } = render(CategoryNav, {
      props: {
        categories: CATEGORIES,
        activeId: "general",
        onSelect,
      },
    });

    const item = getByTestId("bg-cat-nav-ai");
    item.focus();
    await fireEvent.keyDown(item, { key: " " });
    expect(onSelect).toHaveBeenCalledWith("ai");
  });

  it("ArrowDown moves focus to the next item", async () => {
    const { getByTestId } = render(CategoryNav, {
      props: { categories: CATEGORIES, activeId: "general" },
    });
    const first = getByTestId("bg-cat-nav-general");
    first.focus();
    await fireEvent.keyDown(first, { key: "ArrowDown" });
    expect(document.activeElement).toBe(getByTestId("bg-cat-nav-appearance"));
  });

  it("ArrowUp moves focus to the previous item", async () => {
    const { getByTestId } = render(CategoryNav, {
      props: { categories: CATEGORIES, activeId: "appearance" },
    });
    const second = getByTestId("bg-cat-nav-appearance");
    second.focus();
    await fireEvent.keyDown(second, { key: "ArrowUp" });
    expect(document.activeElement).toBe(getByTestId("bg-cat-nav-general"));
  });

  it("ArrowDown wraps from the last to the first item", async () => {
    const { getByTestId } = render(CategoryNav, {
      props: { categories: CATEGORIES, activeId: "ai" },
    });
    const last = getByTestId("bg-cat-nav-ai");
    last.focus();
    await fireEvent.keyDown(last, { key: "ArrowDown" });
    expect(document.activeElement).toBe(getByTestId("bg-cat-nav-general"));
  });

  it("ArrowUp wraps from the first to the last item", async () => {
    const { getByTestId } = render(CategoryNav, {
      props: { categories: CATEGORIES, activeId: "general" },
    });
    const first = getByTestId("bg-cat-nav-general");
    first.focus();
    await fireEvent.keyDown(first, { key: "ArrowUp" });
    expect(document.activeElement).toBe(getByTestId("bg-cat-nav-ai"));
  });

  it("Home jumps to the first item, End jumps to the last", async () => {
    const { getByTestId } = render(CategoryNav, {
      props: { categories: CATEGORIES, activeId: "appearance" },
    });
    const middle = getByTestId("bg-cat-nav-appearance");
    middle.focus();
    await fireEvent.keyDown(middle, { key: "End" });
    expect(document.activeElement).toBe(getByTestId("bg-cat-nav-ai"));

    const last = getByTestId("bg-cat-nav-ai");
    await fireEvent.keyDown(last, { key: "Home" });
    expect(document.activeElement).toBe(getByTestId("bg-cat-nav-general"));
  });
});
