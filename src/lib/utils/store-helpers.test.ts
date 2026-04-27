import { describe, it, expect, vi } from "vitest";
import { writable, get } from "svelte/store";
import { fetchIntoStore, fetchListIntoStore, fetchPageIntoStore } from "./store-helpers";

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

describe("fetchListIntoStore", () => {
  it("sets data and preserves valid selection", async () => {
    const store = writable<{ id: string }[]>([]);
    const loading = writable(false);
    const selectedKey = writable<string | null>("b");

    await fetchListIntoStore(
      store, loading, selectedKey,
      async () => [{ id: "a" }, { id: "b" }, { id: "c" }],
      [],
      (item) => item.id,
    );

    expect(get(store)).toEqual([{ id: "a" }, { id: "b" }, { id: "c" }]);
    expect(get(selectedKey)).toBe("b");
    expect(get(loading)).toBe(false);
  });

  it("clears selection when selected key no longer exists", async () => {
    const store = writable<{ id: string }[]>([]);
    const loading = writable(false);
    const selectedKey = writable<string | null>("deleted");

    await fetchListIntoStore(
      store, loading, selectedKey,
      async () => [{ id: "a" }, { id: "b" }],
      [],
      (item) => item.id,
    );

    expect(get(selectedKey)).toBe(null);
  });

  it("sets fallback and clears selection on error", async () => {
    const store = writable<{ id: string }[]>([{ id: "old" }]);
    const loading = writable(false);
    const selectedKey = writable<string | null>("old");

    await fetchListIntoStore(
      store, loading, selectedKey,
      async () => { throw new Error("fail"); },
      [],
      (item) => item.id,
    );

    expect(get(store)).toEqual([]);
    expect(get(selectedKey)).toBe(null);
    expect(get(loading)).toBe(false);
  });

  it("manages loading state", async () => {
    const store = writable<{ id: string }[]>([]);
    const loading = writable(false);
    const selectedKey = writable<string | null>(null);
    const states: boolean[] = [];
    loading.subscribe(v => states.push(v));

    await fetchListIntoStore(
      store, loading, selectedKey,
      async () => [{ id: "x" }],
      [],
      (item) => item.id,
    );

    expect(states).toContain(true);
    expect(get(loading)).toBe(false);
  });
});

describe("fetchPageIntoStore", () => {
  it("replaces store on page 0", async () => {
    const store = writable<string[]>(["old"]);
    const loading = writable(false);
    const hasMore = writable(false);

    await fetchPageIntoStore(store, loading, hasMore, 0, async () => ["a", "b", "c"], 3);

    expect(get(store)).toEqual(["a", "b", "c"]);
    expect(get(hasMore)).toBe(true);
    expect(get(loading)).toBe(false);
  });

  it("appends to store on page > 0", async () => {
    const store = writable<string[]>(["a", "b", "c"]);
    const loading = writable(false);
    const hasMore = writable(true);

    await fetchPageIntoStore(store, loading, hasMore, 1, async () => ["d", "e"], 3);

    expect(get(store)).toEqual(["a", "b", "c", "d", "e"]);
    expect(get(hasMore)).toBe(false); // 2 < 3
  });

  it("sets hasMore false when results < pageSize", async () => {
    const store = writable<string[]>([]);
    const loading = writable(false);
    const hasMore = writable(false);

    await fetchPageIntoStore(store, loading, hasMore, 0, async () => ["a"], 10);

    expect(get(hasMore)).toBe(false);
  });

  it("sets fallback on page 0 error", async () => {
    const store = writable<string[]>(["old"]);
    const loading = writable(false);
    const hasMore = writable(true);

    await fetchPageIntoStore(
      store, loading, hasMore, 0,
      async () => { throw new Error("fail"); },
      10,
    );

    expect(get(store)).toEqual([]);
    expect(get(hasMore)).toBe(false);
    expect(get(loading)).toBe(false);
  });

  it("does not change store on page > 0 error", async () => {
    const store = writable<string[]>(["a", "b"]);
    const loading = writable(false);
    const hasMore = writable(true);

    await fetchPageIntoStore(
      store, loading, hasMore, 1,
      async () => { throw new Error("fail"); },
      10,
    );

    expect(get(store)).toEqual(["a", "b"]);
    expect(get(hasMore)).toBe(false);
    expect(get(loading)).toBe(false);
  });
});
