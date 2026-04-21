/**
 * Unit tests for `SearchInput.svelte`.
 *
 * Verifies:
 * - Typing fires `onSearch` only after the debounce window elapses.
 * - Subsequent keystrokes within the window reset the timer, yielding
 *   one final event with the latest value.
 * - The clear button resets the value and emits `onSearch("")`
 *   immediately.
 * - The clear button is hidden when the value is empty.
 * - `Cmd+K` / `Ctrl+K` focuses the input.
 */

import {
  describe,
  expect,
  it,
  vi,
  afterEach,
  beforeEach,
} from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import SearchInput from "../SearchInput.svelte";

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  cleanup();
});

describe("SearchInput", () => {
  it("emits onSearch after the debounce window elapses", async () => {
    const onSearch = vi.fn();
    const { getByTestId } = render(SearchInput, {
      props: { debounceMs: 150, onSearch },
    });
    const input = getByTestId("bg-search-input") as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "theme" } });
    expect(onSearch).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(150);
    expect(onSearch).toHaveBeenCalledTimes(1);
    expect(onSearch).toHaveBeenCalledWith("theme");
  });

  it("resets the debounce timer on each keystroke", async () => {
    const onSearch = vi.fn();
    const { getByTestId } = render(SearchInput, {
      props: { debounceMs: 150, onSearch },
    });
    const input = getByTestId("bg-search-input") as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "t" } });
    await vi.advanceTimersByTimeAsync(100);
    await fireEvent.input(input, { target: { value: "th" } });
    await vi.advanceTimersByTimeAsync(100);
    expect(onSearch).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(50);
    expect(onSearch).toHaveBeenCalledTimes(1);
    expect(onSearch).toHaveBeenCalledWith("th");
  });

  it("clear button resets value and emits onSearch('') immediately", async () => {
    const onSearch = vi.fn();
    const { getByTestId, queryByTestId } = render(SearchInput, {
      props: { value: "theme", debounceMs: 150, onSearch },
    });

    const clearBtn = getByTestId("bg-search-clear");
    await fireEvent.click(clearBtn);

    expect(onSearch).toHaveBeenCalledTimes(1);
    expect(onSearch).toHaveBeenCalledWith("");

    // Clear button is no longer rendered when value is empty.
    expect(queryByTestId("bg-search-clear")).toBeNull();
  });

  it("hides the clear button when value is empty", () => {
    const { queryByTestId } = render(SearchInput, {
      props: { value: "" },
    });
    expect(queryByTestId("bg-search-clear")).toBeNull();
  });

  it("focuses the input on Cmd+K", async () => {
    const { getByTestId } = render(SearchInput, { props: {} });
    const input = getByTestId("bg-search-input") as HTMLInputElement;

    // Sanity: input is not focused initially.
    input.blur();
    expect(document.activeElement).not.toBe(input);

    await fireEvent.keyDown(window, { key: "k", metaKey: true });
    expect(document.activeElement).toBe(input);
  });

  it("focuses the input on Ctrl+K", async () => {
    const { getByTestId } = render(SearchInput, { props: {} });
    const input = getByTestId("bg-search-input") as HTMLInputElement;
    input.blur();

    await fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(document.activeElement).toBe(input);
  });
});
