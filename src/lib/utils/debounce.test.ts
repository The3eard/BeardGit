import { describe, it, expect, vi } from "vitest";
import { debounce } from "./debounce";

describe("debounce", () => {
  it("calls the function after the delay", () => {
    vi.useFakeTimers();
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced();
    expect(fn).not.toHaveBeenCalled();

    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledOnce();

    vi.useRealTimers();
  });

  it("resets the timer on subsequent calls", () => {
    vi.useFakeTimers();
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced();
    vi.advanceTimersByTime(50);
    debounced(); // reset
    vi.advanceTimersByTime(50);
    expect(fn).not.toHaveBeenCalled();

    vi.advanceTimersByTime(50);
    expect(fn).toHaveBeenCalledOnce();

    vi.useRealTimers();
  });

  it("passes arguments to the callback", () => {
    vi.useFakeTimers();
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced("hello", 42);
    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledWith("hello", 42);

    vi.useRealTimers();
  });

  it("uses the latest arguments when called multiple times", () => {
    vi.useFakeTimers();
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced("first");
    debounced("second");
    debounced("third");
    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledOnce();
    expect(fn).toHaveBeenCalledWith("third");

    vi.useRealTimers();
  });
});
