/**
 * Sidebar Navigation layout store.
 *
 * Holds the user's persisted order + hidden ids for the Navigation
 * section of the sidebar. Hydrated **once** from the backend at
 * bootstrap (`loadSidebarLayout`) and mutated via `updateLayout`, which
 * patches the store synchronously and debounces the backend save by
 * `DEBOUNCE_MS` so rapid drag / toggle interactions don't thrash the
 * config file.
 *
 * Persistence is app-scoped (not per-repo) — see design spec
 * `docs/superpowers/specs/2026-04-23-sidebar-customization-design.md`
 * §2 Goal #4.
 */
import { writable, get } from "svelte/store";
import { getSidebarNavLayout, setSidebarNavLayout } from "$lib/api/tauri";
import type { SidebarNavLayout } from "$lib/types";

/** Debounce window for persisting layout changes to disk. */
export const DEBOUNCE_MS = 250;

/** Current layout. Start empty — `applyLayout` falls back to the
 *  canonical default until the first `loadSidebarLayout()` completes. */
export const sidebarLayout = writable<SidebarNavLayout>({
  order: [],
  hidden: [],
});

/** Hydrate the store once at app start. Idempotent. */
export async function loadSidebarLayout(): Promise<void> {
  try {
    const layout = await getSidebarNavLayout();
    sidebarLayout.set(layout);
  } catch {
    // Leave the default empty layout in place — `applyLayout` still
    // renders the canonical order, so the user sees a working sidebar
    // even if the IPC roundtrip fails.
  }
}

let saveTimer: ReturnType<typeof setTimeout> | null = null;

/** Patch the layout and schedule a debounced save. */
export function updateLayout(patch: Partial<SidebarNavLayout>): void {
  const next = { ...get(sidebarLayout), ...patch };
  sidebarLayout.set(next);
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    void setSidebarNavLayout(next);
    saveTimer = null;
  }, DEBOUNCE_MS);
}
