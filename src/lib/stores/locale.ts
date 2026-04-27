import { writable } from "svelte/store";
import * as api from "../api/tauri";
import { setLocale as setParaglideLocale, locales } from "$lib/paraglide/runtime";

type Locale = (typeof locales)[number];

export const currentLocale = writable<string>("en-US");

/** Load persisted locale from backend and apply it. */
export async function initLocale() {
  try {
    const saved = await api.getLocale();
    setParaglideLocale(saved as Locale);
    currentLocale.set(saved);
  } catch {
    // Default to en-US on error
  }
}

/** Change locale: persist to backend, then reload the app to apply everywhere. */
export async function changeLocale(locale: string) {
  await api.setLocaleConfig(locale);
  window.location.reload();
}
