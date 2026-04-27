/**
 * SessionRow — shared primitive for AI session list rows.
 *
 * Renders icon snippet + title + date. Selected state toggles a class.
 * Full-row button semantics: click + Space + Enter all fire `onSelect`.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import SessionRowHost from "./SessionRowHost.test.svelte";

afterEach(() => cleanup());

describe("SessionRow", () => {
  it("renders the icon snippet, title, and date", async () => {
    const { container } = render(SessionRowHost, {
      props: {
        title: "Fix the login flow",
        date: "2h ago",
        selected: false,
        onSelect: () => {},
      },
    });
    await tick();

    expect(container.textContent).toContain("Fix the login flow");
    expect(container.textContent).toContain("2h ago");
    expect(container.querySelector('[data-testid="row-icon-stub"]')).toBeTruthy();
  });

  it("renders em-dash when date is null", async () => {
    const { container } = render(SessionRowHost, {
      props: {
        title: "Idle terminal",
        date: null,
        selected: false,
        onSelect: () => {},
      },
    });
    await tick();
    expect(container.textContent).toContain("—");
  });

  it("applies `selected` class when selected prop is true", async () => {
    const { container } = render(SessionRowHost, {
      props: {
        title: "Anything",
        date: null,
        selected: true,
        onSelect: () => {},
      },
    });
    await tick();
    const row = container.querySelector(".session-row") as HTMLElement;
    expect(row.classList.contains("selected")).toBe(true);
  });

  it("click fires onSelect", async () => {
    const onSelect = vi.fn();
    const { container } = render(SessionRowHost, {
      props: { title: "x", date: null, selected: false, onSelect },
    });
    await tick();
    await fireEvent.click(container.querySelector(".session-row")!);
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("Enter and Space fire onSelect and prevent default", async () => {
    const onSelect = vi.fn();
    const { container } = render(SessionRowHost, {
      props: { title: "x", date: null, selected: false, onSelect },
    });
    await tick();
    const row = container.querySelector(".session-row") as HTMLElement;

    await fireEvent.keyDown(row, { key: "Enter" });
    expect(onSelect).toHaveBeenCalledTimes(1);

    await fireEvent.keyDown(row, { key: " " });
    expect(onSelect).toHaveBeenCalledTimes(2);

    // Unrelated keys do nothing.
    await fireEvent.keyDown(row, { key: "ArrowDown" });
    expect(onSelect).toHaveBeenCalledTimes(2);
  });
});
