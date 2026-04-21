import { describe, it, expect } from "vitest";
import { isCacheFresh, GRAPH_CACHE_TTL_MS } from "../project-cache";

describe("project-cache graph slice", () => {
  it("rejects caches older than 7 days", () => {
    const stale = Date.now() - GRAPH_CACHE_TTL_MS - 1;
    expect(isCacheFresh(stale)).toBe(false);
  });

  it("accepts recent caches", () => {
    expect(isCacheFresh(Date.now())).toBe(true);
  });

  it("accepts caches exactly at the TTL boundary", () => {
    const boundary = Date.now() - GRAPH_CACHE_TTL_MS;
    expect(isCacheFresh(boundary)).toBe(true);
  });

  it("exposes the TTL as 7 days in ms", () => {
    expect(GRAPH_CACHE_TTL_MS).toBe(7 * 24 * 60 * 60 * 1000);
  });
});
