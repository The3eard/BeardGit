/**
 * Unit tests for `withTimeout`.
 *
 * Verifies that wrapped promises settle normally when they win the
 * race, reject with a dedicated `TimeoutError` when the window
 * elapses first, and surface the original rejection when they fail
 * before the timeout fires.
 *
 * The timeout test drives Vitest's fake timers so we don't actually
 * wait 15 seconds on CI.
 */

import { afterEach, describe, expect, it, vi } from "vitest";
import { TimeoutError, withTimeout } from "./withTimeout";

afterEach(() => {
  vi.useRealTimers();
});

describe("withTimeout", () => {
  it("resolves with the promise's value if it settles before the window", async () => {
    const p = Promise.resolve(42);
    await expect(withTimeout(p, 1000)).resolves.toBe(42);
  });

  it("rejects with TimeoutError after the window elapses", async () => {
    vi.useFakeTimers();
    // eslint-disable-next-line @typescript-eslint/no-empty-function
    const pending = new Promise<number>(() => {
      /* never resolves */
    });
    const wrapped = withTimeout(pending, 15_000);
    vi.advanceTimersByTime(15_000);
    await expect(wrapped).rejects.toBeInstanceOf(TimeoutError);
  });

  it("propagates the original rejection when it rejects before the timeout", async () => {
    const p = Promise.reject(new Error("boom"));
    await expect(withTimeout(p, 1000)).rejects.toThrow("boom");
  });

  it("TimeoutError carries the ms count in its message and name", async () => {
    vi.useFakeTimers();
    // eslint-disable-next-line @typescript-eslint/no-empty-function
    const pending = new Promise<number>(() => {});
    const wrapped = withTimeout(pending, 2500);
    vi.advanceTimersByTime(2500);
    await expect(wrapped).rejects.toMatchObject({
      name: "TimeoutError",
      message: expect.stringContaining("2500"),
    });
  });
});
