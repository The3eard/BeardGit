import { describe, it, expect, vi } from "vitest";
import { writable, get } from "svelte/store";
import { fetchIntoStore } from "./store-helpers";

describe("fetchIntoStore", () => {
  it("sets data on success", async () => {
    const store = writable<string[]>([]);
    const loading = writable(false);
    await fetchIntoStore(store, loading, async () => ["a", "b"], []);
    expect(get(store)).toEqual(["a", "b"]);
    expect(get(loading)).toBe(false);
  });

  it("sets fallback on error", async () => {
    const store = writable<string[]>(["old"]);
    const loading = writable(false);
    await fetchIntoStore(store, loading, async () => { throw new Error("fail"); }, []);
    expect(get(store)).toEqual([]);
    expect(get(loading)).toBe(false);
  });

  it("manages loading state", async () => {
    const store = writable<string[]>([]);
    const loading = writable(false);
    const loadingStates: boolean[] = [];
    loading.subscribe(v => loadingStates.push(v));
    await fetchIntoStore(store, loading, async () => ["x"], []);
    expect(loadingStates).toContain(true);
    expect(get(loading)).toBe(false);
  });
});
