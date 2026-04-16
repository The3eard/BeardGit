/**
 * Store helper utilities — shared patterns for Svelte store management.
 */

import type { Writable } from "svelte/store";
import { get } from "svelte/store";

/**
 * Fetch data from an async API call and update a store, with loading state management.
 * On error, sets the store to the provided fallback value.
 *
 * @param store   The writable store to update with fetched data.
 * @param loading A writable boolean store set to true while fetching.
 * @param fetcher Async function that returns the data to store.
 * @param fallback Value to set on the store if the fetcher throws.
 */
export async function fetchIntoStore<T>(
  store: Writable<T>,
  loading: Writable<boolean>,
  fetcher: () => Promise<T>,
  fallback: T,
): Promise<void> {
  loading.set(true);
  try {
    const data = await fetcher();
    store.set(data);
  } catch {
    store.set(fallback);
  } finally {
    loading.set(false);
  }
}

/**
 * Fetch a list from an async API call, update the store, and validate
 * that the current selection still exists in the new results.
 *
 * If the selected key is no longer present, clears it to null.
 *
 * @param store       The writable store for the list data.
 * @param loading     A writable boolean store set to true while fetching.
 * @param selectedKey A writable store holding the currently selected item's key.
 * @param fetcher     Async function that returns the list data.
 * @param fallback    Value to set on error.
 * @param getKey      Function to extract a unique key from each item.
 */
export async function fetchListIntoStore<T>(
  store: Writable<T[]>,
  loading: Writable<boolean>,
  selectedKey: Writable<string | null>,
  fetcher: () => Promise<T[]>,
  fallback: T[],
  getKey: (item: T) => string,
): Promise<void> {
  loading.set(true);
  try {
    const data = await fetcher();
    store.set(data);
    const currentKey = get(selectedKey);
    if (currentKey !== null && !data.some((item) => getKey(item) === currentKey)) {
      selectedKey.set(null);
    }
  } catch {
    store.set(fallback);
    selectedKey.set(null);
  } finally {
    loading.set(false);
  }
}

/**
 * Fetch a page of results from an async API call.
 *
 * Page 0 replaces the store; subsequent pages append. Sets `hasMore`
 * based on whether the result count equals the page size.
 *
 * @param store    The writable store for the accumulated list data.
 * @param loading  A writable boolean store set to true while fetching.
 * @param hasMore  A writable boolean store tracking if more pages exist.
 * @param page     The page number (0-based). Page 0 replaces, page 1+ appends.
 * @param fetcher  Async function that returns one page of results.
 * @param pageSize Expected page size. hasMore = results.length >= pageSize.
 */
export async function fetchPageIntoStore<T>(
  store: Writable<T[]>,
  loading: Writable<boolean>,
  hasMore: Writable<boolean>,
  page: number,
  fetcher: () => Promise<T[]>,
  pageSize: number,
): Promise<void> {
  loading.set(true);
  try {
    const data = await fetcher();
    if (page === 0) {
      store.set(data);
    } else {
      const current = get(store);
      store.set([...current, ...data]);
    }
    hasMore.set(data.length >= pageSize);
  } catch {
    if (page === 0) {
      store.set([]);
    }
    hasMore.set(false);
  } finally {
    loading.set(false);
  }
}
