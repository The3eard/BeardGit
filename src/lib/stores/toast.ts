/**
 * Toast notification store — reusable, stackable toast system.
 *
 * Manages a list of up to 3 visible toasts. Each toast can be
 * created, mutated (for multi-step flows like update download),
 * and removed. Auto-dismiss is handled by the Toast component.
 */

import { writable } from "svelte/store";

const MAX_TOASTS = 3;
let nextId = 0;

export interface ToastAction {
  label: string;
  onclick: () => void;
}

export interface Toast {
  id: string;
  message: string;
  type: "info" | "success" | "warning" | "error";
  actions?: ToastAction[];
  dismissible: boolean;
  duration: number | null;
  /**
   * Optional 0..1 progress fraction. When present, the toast renders a
   * thin progress bar beneath the message — used by the auto-update
   * download lifecycle until the unified tasks drawer (cluster 0.3)
   * takes over. Leave `undefined` for normal toasts.
   */
  progress?: number;
}

export type ToastOptions = {
  message: string;
  type: "info" | "success" | "warning" | "error";
  actions?: ToastAction[];
  dismissible?: boolean;
  duration?: number | null;
  /** Optional 0..1 download progress fraction. See {@link Toast.progress}. */
  progress?: number;
};

export const toasts = writable<Toast[]>([]);

/** Add a toast. Returns its ID for later mutation or removal. */
export function addToast(options: ToastOptions): string {
  const id = `toast-${nextId++}`;
  const toast: Toast = {
    id,
    message: options.message,
    type: options.type,
    actions: options.actions,
    dismissible: options.dismissible ?? true,
    duration: options.duration === undefined ? 5000 : options.duration,
    progress: options.progress,
  };
  toasts.update((list) => {
    const next = [...list, toast];
    return next.length > MAX_TOASTS ? next.slice(next.length - MAX_TOASTS) : next;
  });
  return id;
}

/** Update an existing toast in place (for multi-step flows). */
export function updateToast(id: string, partial: Partial<ToastOptions>): void {
  toasts.update((list) =>
    list.map((t) => (t.id === id ? { ...t, ...partial } : t))
  );
}

/** Remove a toast by ID. */
export function removeToast(id: string): void {
  toasts.update((list) => list.filter((t) => t.id !== id));
}
