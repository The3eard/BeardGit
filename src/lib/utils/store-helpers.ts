/**
 * Store helper utilities — shared patterns for Svelte store management.
 */

import type { Writable } from "svelte/store";

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
