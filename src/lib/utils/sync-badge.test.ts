import { describe, it, expect } from "vitest";
import { shouldShowSyncBadge, formatSyncBadge } from "./sync-badge";

describe("shouldShowSyncBadge", () => {
  it("shows for a positive count", () => {
    expect(shouldShowSyncBadge(1)).toBe(true);
    expect(shouldShowSyncBadge(42)).toBe(true);
  });

  it("hides at zero (in sync, no upstream, or detached HEAD)", () => {
    expect(shouldShowSyncBadge(0)).toBe(false);
  });

  it("hides for missing / non-finite counts", () => {
    expect(shouldShowSyncBadge(null)).toBe(false);
    expect(shouldShowSyncBadge(undefined)).toBe(false);
    expect(shouldShowSyncBadge(NaN)).toBe(false);
    expect(shouldShowSyncBadge(-3)).toBe(false);
  });
});

describe("formatSyncBadge", () => {
  it("shows small counts verbatim", () => {
    expect(formatSyncBadge(1)).toBe("1");
    expect(formatSyncBadge(9)).toBe("9");
    expect(formatSyncBadge(42)).toBe("42");
    expect(formatSyncBadge(99)).toBe("99");
  });

  it("caps absurd counts at 99+", () => {
    expect(formatSyncBadge(100)).toBe("99+");
    expect(formatSyncBadge(12345)).toBe("99+");
  });
});
