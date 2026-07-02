import { describe, it, expect } from "vitest";
import { clampMenuPosition } from "./menu-position";

const VIEWPORT = { viewportWidth: 1000, viewportHeight: 800 };

describe("clampMenuPosition", () => {
  it("leaves a menu that fits at the cursor untouched", () => {
    const { left, top } = clampMenuPosition(100, 100, 180, 240, VIEWPORT);
    expect(left).toBe(100);
    expect(top).toBe(100);
  });

  it("flips leftward when the menu would overflow the right edge", () => {
    // cursor near right edge: 950 + 180 = 1130 > 1000 - 8
    const { left } = clampMenuPosition(950, 100, 180, 240, VIEWPORT);
    expect(left).toBe(950 - 180); // opens to the left of the cursor
    expect(left + 180).toBeLessThanOrEqual(1000);
  });

  it("flips upward when the menu would overflow the bottom edge", () => {
    const { top } = clampMenuPosition(100, 700, 180, 240, VIEWPORT);
    expect(top).toBe(700 - 240);
    expect(top + 240).toBeLessThanOrEqual(800);
  });

  it("clamps to the margin when the menu is larger than the viewport", () => {
    // menu bigger than the window: flip goes negative, clamp to margin
    const { left, top } = clampMenuPosition(280, 280, 350, 350, {
      viewportWidth: 300,
      viewportHeight: 300,
      margin: 8,
    });
    expect(left).toBe(8);
    expect(top).toBe(8);
  });

  it("keeps the menu at least `margin` from the top-left edge", () => {
    const { left, top } = clampMenuPosition(2, 2, 180, 240, VIEWPORT);
    expect(left).toBeGreaterThanOrEqual(8);
    expect(top).toBeGreaterThanOrEqual(8);
  });
});
