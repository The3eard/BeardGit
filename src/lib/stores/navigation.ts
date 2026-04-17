/**
 * Navigation store — tracks the currently-active sidebar view.
 *
 * Extracted out of `+page.svelte` so that cross-cutting concerns (e.g. the
 * `<Xrefs>` component) can programmatically switch views without lifting
 * local state everywhere.
 *
 * The identifiers match the `id` field on Sidebar nav items
 * (`"graph"`, `"changes"`, `"merge-requests"`, `"issues"`, …).
 */

import { writable } from "svelte/store";

/** Currently active sidebar view identifier. */
export const activeViewStore = writable<string>("graph");
