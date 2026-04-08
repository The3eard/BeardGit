/**
 * Keyboard shortcuts store — central registry, platform detection, and key matching.
 *
 * Global shortcuts (with modifiers) always fire. Contextual shortcuts (bare keys)
 * only fire when no input, textarea, or contenteditable element is focused.
 */

import { writable } from "svelte/store";

/** A keyboard shortcut binding. */
export interface Shortcut {
  /** Unique identifier, e.g. "nav.graph" or "git.fetch". */
  id: string;
  /** Key descriptor. */
  keys: ShortcutKeys;
  /** Human-readable label for the cheat sheet. */
  label: string;
  /** Category for grouping in the cheat sheet. */
  category: string;
  /** Action to run when the shortcut fires. */
  action: () => void;
}

export interface ShortcutKeys {
  /** Whether the platform modifier is required (Cmd on macOS, Ctrl otherwise). */
  mod?: boolean;
  shift?: boolean;
  alt?: boolean;
  /** The key value (e.g. "f", "1", "Tab", "/", "?", "Home", "End"). */
  key: string;
}

/** Whether the current platform is macOS. */
export const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.userAgent);

/** Format a shortcut for display (e.g. "Cmd+Shift+F" or "Ctrl+Shift+F"). */
export function formatShortcut(keys: ShortcutKeys): string {
  const parts: string[] = [];
  if (keys.mod) parts.push(isMac ? "\u2318" : "Ctrl");
  if (keys.shift) parts.push(isMac ? "\u21E7" : "Shift");
  if (keys.alt) parts.push(isMac ? "\u2325" : "Alt");

  const keyDisplay = keyDisplayName(keys.key);
  parts.push(keyDisplay);

  return parts.join(isMac ? "" : "+");
}

function keyDisplayName(key: string): string {
  const map: Record<string, string> = {
    Tab: isMac ? "\u21E5" : "Tab",
    Home: "Home",
    End: "End",
    Escape: isMac ? "\u238B" : "Esc",
    "/": "/",
    "?": "?",
    ",": ",",
  };
  if (map[key]) return map[key];
  // Single character keys: uppercase for display
  if (key.length === 1) return key.toUpperCase();
  return key;
}

/** The shortcut registry. */
export const shortcuts = writable<Shortcut[]>([]);

/** Whether the cheat sheet overlay is visible. */
export const showCheatSheet = writable(false);

/** Toggle the cheat sheet overlay visibility. */
export function toggleCheatSheet(): void {
  showCheatSheet.update((v) => !v);
}

/** Register a batch of shortcuts, replacing any with matching ids. */
export function registerShortcuts(newShortcuts: Shortcut[]): void {
  shortcuts.update((existing) => {
    const ids = new Set(newShortcuts.map((s) => s.id));
    const filtered = existing.filter((s) => !ids.has(s.id));
    return [...filtered, ...newShortcuts];
  });
}

/** Unregister shortcuts by id. */
export function unregisterShortcuts(ids: string[]): void {
  const idSet = new Set(ids);
  shortcuts.update((existing) => existing.filter((s) => !idSet.has(s.id)));
}

/** Check if an element is an input that should suppress bare-key shortcuts. */
function isInputFocused(): boolean {
  const el = document.activeElement;
  if (!el) return false;
  const tag = el.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  if ((el as HTMLElement).isContentEditable) return true;
  // CodeMirror editors use role="textbox"
  if (el.getAttribute("role") === "textbox") return true;
  return false;
}

/** Check if a KeyboardEvent matches a ShortcutKeys descriptor. */
function matchesKeys(e: KeyboardEvent, keys: ShortcutKeys): boolean {
  const modPressed = isMac ? e.metaKey : e.ctrlKey;
  if (keys.mod && !modPressed) return false;
  if (!keys.mod && modPressed) return false;
  if ((keys.shift ?? false) !== e.shiftKey) return false;
  if ((keys.alt ?? false) !== e.altKey) return false;
  return e.key === keys.key || e.key.toLowerCase() === keys.key.toLowerCase();
}

/**
 * The global keydown handler. Call this from +layout.svelte.
 * Returns a cleanup function.
 */
export function initShortcutListener(): () => void {
  function handleKeyDown(e: KeyboardEvent) {
    let matched = false;

    shortcuts.subscribe((list) => {
      for (const shortcut of list) {
        if (!matchesKeys(e, shortcut.keys)) continue;

        // Bare-key shortcuts (no mod) only fire when no input focused
        const hasModifier = shortcut.keys.mod || shortcut.keys.alt;
        if (!hasModifier && isInputFocused()) continue;

        e.preventDefault();
        e.stopPropagation();
        shortcut.action();
        matched = true;
        break;
      }
    })(); // immediately unsubscribe (sync read)

    void matched;
  }

  window.addEventListener("keydown", handleKeyDown, { capture: true });
  return () => window.removeEventListener("keydown", handleKeyDown, { capture: true });
}
