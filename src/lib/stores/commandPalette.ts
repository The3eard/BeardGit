/**
 * Command palette open/close state. Driven by:
 *   - the Cmd+Shift+P / Ctrl+Shift+P shortcut registered in
 *     `+page.svelte`'s shortcut bootstrap
 *   - a programmatic `openCommandPalette()` from any place that wants to
 *     surface the picker (statusbar entry-point, future "?")
 */
import { writable } from "svelte/store";

export const commandPaletteOpen = writable<boolean>(false);

export function openCommandPalette(): void {
  commandPaletteOpen.set(true);
}

export function closeCommandPalette(): void {
  commandPaletteOpen.set(false);
}

export function toggleCommandPalette(): void {
  commandPaletteOpen.update((v) => !v);
}
