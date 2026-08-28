import { describe, expect, it } from "vitest";
import { computeVirtualWindow, virtualRowStyle } from "./virtualWindow";

const base = {
  count: 10_000,
  rowHeight: 24,
  scrollTop: 0,
  viewportHeight: 480,
  threshold: 500,
};

describe("computeVirtualWindow", () => {
  it("opts out without a uniform row height", () => {
    expect(computeVirtualWindow({ ...base, rowHeight: undefined })).toBeNull();
    expect(computeVirtualWindow({ ...base, rowHeight: 0 })).toBeNull();
  });

  it("opts out at or below the threshold", () => {
    expect(computeVirtualWindow({ ...base, count: 500 })).toBeNull();
    expect(computeVirtualWindow({ ...base, count: 501 })).not.toBeNull();
  });

  it("mounts a viewport's worth plus overscan, not the whole list", () => {
    const w = computeVirtualWindow(base)!;
    // 480 / 24 = 20 visible, + 6 overscan above and below.
    expect(w.end - w.start).toBe(32);
    expect(w.start).toBe(0);
    expect(w.totalHeight).toBe(240_000);
  });

  it("does not scroll past the start", () => {
    const w = computeVirtualWindow({ ...base, scrollTop: 0 })!;
    expect(w.start).toBe(0);
  });

  it("follows the scroll offset", () => {
    const w = computeVirtualWindow({ ...base, scrollTop: 2400 })!;
    // Row 100 is at the top; overscan pulls the mount point back by 6.
    expect(w.start).toBe(94);
  });

  it("clamps the end to the item count at the bottom", () => {
    const w = computeVirtualWindow({ ...base, scrollTop: 24 * 9_999 })!;
    expect(w.end).toBe(10_000);
    expect(w.start).toBeLessThan(10_000);
  });

  it("still returns a usable window before the viewport is measured", () => {
    // First paint: nothing has been measured yet, so the assumed height has
    // to produce a non-empty window or the list renders blank.
    const w = computeVirtualWindow({ ...base, viewportHeight: 0 })!;
    expect(w.end).toBeGreaterThan(w.start);
  });

  it("keeps the window inside the list when count is just over threshold", () => {
    const w = computeVirtualWindow({ ...base, count: 501, scrollTop: 0 })!;
    expect(w.start).toBe(0);
    expect(w.end).toBeLessThanOrEqual(501);
  });
});

describe("virtualRowStyle", () => {
  it("anchors a row at its absolute offset", () => {
    expect(virtualRowStyle(3, 24)).toContain("top: 72px");
    expect(virtualRowStyle(3, 24)).toContain("height: 24px");
  });
});
