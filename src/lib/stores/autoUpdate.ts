/**
 * Auto-update store — typed wrapper over `@tauri-apps/plugin-updater`.
 *
 * Exposes a single `autoUpdateState` svelte store that represents the
 * current phase of the update lifecycle, plus helper functions for
 * driving it from the UI:
 *
 *   idle → checking → available → downloading → ready → (relaunch)
 *                             \_→ up_to_date
 *                             \_→ error
 *
 * This module is the canonical surface for Phase 2+ of the in-app
 * auto-update initiative. The legacy [`updater.ts`](./updater.ts)
 * store continues to work for the existing toast-driven flow; Phase 3
 * adds the `runStartupCheck()` wrapper around this store.
 *
 * ## Design notes
 *
 * - The plugin imports are dynamic so unit tests can vi.mock them and
 *   the store still loads in Node (no Tauri runtime needed at import
 *   time).
 * - `needsReauthNotice` becomes `true` once the download completes on
 *   macOS / Windows so the install flow can render the apology dialog
 *   (Phase 4). On Linux it stays `false`.
 * - Per-OS "don't show again" persistence lives in the settings store
 *   (Phase 4). This module just exposes OS detection and the state
 *   machine.
 */

import { writable, type Readable, derived } from "svelte/store";
import type { Update } from "@tauri-apps/plugin-updater";
import { addToast, updateToast, removeToast } from "./toast";
import * as m from "$lib/paraglide/messages";
import {
  getAutoCheckUpdates,
  getReauthDismissed,
  setReauthDismissed,
} from "$lib/api/tauri";
import type { TaskEntry } from "$lib/types/tasks";

/** Phase of the update lifecycle. */
export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "up_to_date"
  | "error";

/** Snapshot of the current update lifecycle. */
export interface UpdateState {
  /** Current phase. */
  status: UpdateStatus;
  /** Version string of the available update, if any. */
  availableVersion?: string;
  /** Release notes / changelog for the available update. */
  releaseNotes?: string;
  /** Human-readable error message. Only set when `status === "error"`. */
  error?: string;
  /** Bytes downloaded so far. */
  downloadedBytes?: number;
  /** Total bytes expected in the download, when known. */
  totalBytes?: number;
}

/** Platform identifier returned by `@tauri-apps/plugin-os`. */
export type AutoUpdateOs = "macos" | "windows" | "linux" | "other";

/**
 * The current update state. Components subscribe to this to render
 * spinners, toasts, progress bars, or the re-auth dialog.
 */
export const autoUpdateState = writable<UpdateState>({ status: "idle" });

/**
 * Flips to `true` after a successful download on macOS / Windows, so
 * the install flow can prompt the user with the Gatekeeper / SmartScreen
 * apology dialog before relaunching.
 */
export const needsReauthNotice = writable(false);

/** Read-only view over `autoUpdateState`. */
export const autoUpdateStateReadonly: Readable<UpdateState> = derived(
  autoUpdateState,
  (s) => s,
);

/** Internal handle to the in-flight `Update` resource (if any). */
let currentUpdate: Update | null = null;

/** Detect the running operating system via `@tauri-apps/plugin-os`. */
export async function detectOs(): Promise<AutoUpdateOs> {
  try {
    const { type } = await import("@tauri-apps/plugin-os");
    const t = type();
    if (t === "macos" || t === "windows" || t === "linux") return t;
    return "other";
  } catch {
    return "other";
  }
}

/**
 * Probe the updater endpoint once. Transitions the store to
 * `checking` → `available` (with metadata) or `up_to_date` or `error`.
 *
 * Returns the `UpdateStatus` the store settled on so callers can react
 * without having to re-subscribe.
 */
export async function checkForUpdates(): Promise<UpdateStatus> {
  autoUpdateState.set({ status: "checking" });
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();
    if (!update) {
      autoUpdateState.set({ status: "up_to_date" });
      return "up_to_date";
    }
    currentUpdate = update;
    autoUpdateState.set({
      status: "available",
      availableVersion: update.version,
      releaseNotes: update.body,
    });
    return "available";
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    autoUpdateState.set({ status: "error", error: message });
    return "error";
  }
}

/**
 * Download the pending update and install it. Transitions the store
 * through `downloading` (with progress) → `ready`. On macOS / Windows
 * the `needsReauthNotice` flag is flipped once the download completes,
 * so the caller may show the Gatekeeper / SmartScreen apology dialog
 * before invoking `relaunchApp()`.
 *
 * @param confirmedReauth When `true`, the caller has already shown the
 * apology dialog (or the user dismissed it previously). Skips setting
 * `needsReauthNotice` on macOS / Windows.
 */
export async function downloadAndInstall(
  confirmedReauth = false,
): Promise<UpdateStatus> {
  if (!currentUpdate) {
    autoUpdateState.set({ status: "error", error: "no_update_available" });
    return "error";
  }

  const update = currentUpdate;
  let totalBytes: number | undefined;
  let downloadedBytes = 0;

  autoUpdateState.set({
    status: "downloading",
    availableVersion: update.version,
    releaseNotes: update.body,
    downloadedBytes: 0,
  });

  try {
    await update.downloadAndInstall((event) => {
      if (event.event === "Started") {
        totalBytes = event.data.contentLength;
        autoUpdateState.update((s) => ({
          ...s,
          status: "downloading",
          totalBytes,
          downloadedBytes: 0,
        }));
      } else if (event.event === "Progress") {
        downloadedBytes += event.data.chunkLength;
        autoUpdateState.update((s) => ({
          ...s,
          status: "downloading",
          downloadedBytes,
        }));
      }
    });

    autoUpdateState.set({
      status: "ready",
      availableVersion: update.version,
      releaseNotes: update.body,
      downloadedBytes,
      totalBytes,
    });

    if (!confirmedReauth) {
      const os = await detectOs();
      if (os === "macos" || os === "windows") {
        needsReauthNotice.set(true);
      }
    }

    return "ready";
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    autoUpdateState.set({ status: "error", error: message });
    return "error";
  }
}

/**
 * Relaunch the app into the freshly installed update. Safe to call
 * only after `downloadAndInstall()` resolved to `"ready"`.
 */
export async function relaunchApp(): Promise<void> {
  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
}

/**
 * Kick off the install flow, honouring the persisted per-OS
 * re-authorization-notice dismissal.
 *
 * Called by the toast Install action and the Settings "Install" button
 * once `checkForUpdates()` has resolved to `"available"`. On macOS and
 * Windows the flow consults [`getReauthDismissed`](../api/tauri.ts);
 * when the user has *not* previously dismissed the dialog, it flips the
 * [`needsReauthNotice`] store to `true` and returns — the caller must
 * render `ReauthNoticeDialog` and call
 * [`confirmReauthAndInstall`] once the user confirms.
 *
 * On Linux (or when the notice is already dismissed) it proceeds
 * straight to [`downloadAndInstall`].
 *
 * Returns the final `UpdateStatus` if the download runs inline, or
 * `"available"` when the dialog was surfaced and we're awaiting the
 * user's confirmation.
 */
export async function startInstallFlow(): Promise<UpdateStatus> {
  const os = await detectOs();
  if (os === "macos" || os === "windows") {
    let dismissed = false;
    try {
      dismissed = await getReauthDismissed(os);
    } catch {
      // If the IPC call fails, err on the side of showing the dialog —
      // worst case the user clicks through an extra click.
      dismissed = false;
    }
    if (!dismissed) {
      needsReauthNotice.set(true);
      return "available";
    }
  }
  return downloadAndInstall(true);
}

/**
 * Called by the `ReauthNoticeDialog` after the user clicks **Update now**.
 *
 * Clears the in-session notice flag, persists the dismissal when the
 * user ticked the "Don't show this again" checkbox, then proceeds with
 * the actual download-and-install. `dismissForever` is the current
 * value of that checkbox.
 */
export async function confirmReauthAndInstall(
  dismissForever: boolean,
): Promise<UpdateStatus> {
  needsReauthNotice.set(false);
  if (dismissForever) {
    const os = await detectOs();
    if (os === "macos" || os === "windows") {
      try {
        await setReauthDismissed(os, true);
      } catch {
        // Persistence failures are non-fatal — the flag only affects the
        // next update cycle, not this one.
      }
    }
  }
  return downloadAndInstall(true);
}

/**
 * Called by the `ReauthNoticeDialog` after the user clicks **Cancel**.
 *
 * Clears the in-session flag and resets the store to `idle` so the
 * next "Install" click starts a fresh flow. Does not persist anything.
 */
export function cancelReauthFlow(): void {
  needsReauthNotice.set(false);
  autoUpdateState.update((s) =>
    s.status === "available" ? s : { status: "idle" },
  );
}

/**
 * Suppress the re-auth notice for subsequent prompts in the current
 * session. Persistence across restarts is the caller's responsibility
 * (wired in Phase 4 via `AppConfig`).
 */
export function dismissReauthForThisOs(): void {
  needsReauthNotice.set(false);
}

/**
 * Reset the store to `idle` — used by tests and by the settings UI
 * after the user dismisses an error banner.
 */
export function resetAutoUpdateState(): void {
  autoUpdateState.set({ status: "idle" });
  needsReauthNotice.set(false);
  currentUpdate = null;
  clearAutoUpdateStartedAt();
}

// ---------------------------------------------------------------------------
// Startup probe (Phase 3)
// ---------------------------------------------------------------------------

/** Session-storage key recording the last startup probe timestamp (ms). */
const LAST_CHECK_KEY = "lastUpdateCheckAt";

/** Debounce window for the startup probe (ms). */
const STARTUP_DEBOUNCE_MS = 60_000;

/**
 * Silently probe for updates on app startup. If an update is found,
 * emits a non-blocking toast with **Install** / **Later** actions.
 *
 * Skips in these cases:
 *
 * - The `auto_check_updates` preference is `false`.
 * - Running in dev mode (`import.meta.env.DEV`).
 * - Another probe fired within the last 60 seconds (tracked in
 *   `sessionStorage.lastUpdateCheckAt`).
 *
 * All failures are swallowed — an offline or 404 startup check must
 * not surface UI noise. Users invoke the manual check from Settings
 * when they want an inline error.
 */
export async function runStartupCheck(): Promise<void> {
  // Dev-mode guard: the updater plugin has no bundle metadata under
  // `tauri dev`, so any probe would error out.
  if (import.meta.env.DEV) return;

  // Preference guard: user opted out via Settings → Updates.
  try {
    const enabled = await getAutoCheckUpdates();
    if (!enabled) return;
  } catch {
    // If the IPC call fails, be conservative and don't probe.
    return;
  }

  // Debounce: 60-second window keyed on sessionStorage so relaunches
  // within the same session don't thrash the endpoint.
  try {
    if (typeof sessionStorage !== "undefined") {
      const raw = sessionStorage.getItem(LAST_CHECK_KEY);
      if (raw) {
        const lastAt = Number.parseInt(raw, 10);
        if (
          !Number.isNaN(lastAt) &&
          Date.now() - lastAt < STARTUP_DEBOUNCE_MS
        ) {
          return;
        }
      }
      sessionStorage.setItem(LAST_CHECK_KEY, String(Date.now()));
    }
  } catch {
    // sessionStorage is unavailable in some test environments — fall
    // through and probe anyway.
  }

  const outcome = await checkForUpdates().catch(() => "error" as UpdateStatus);
  if (outcome !== "available") return;

  const pendingVersion =
    getStateSnapshot().availableVersion ?? "";

  emitUpdateAvailableToast(pendingVersion);
}

/** Cheap synchronous view of the store (no subscription bookkeeping). */
function getStateSnapshot(): UpdateState {
  let snapshot: UpdateState = { status: "idle" };
  const unsubscribe = autoUpdateState.subscribe((s) => {
    snapshot = s;
  });
  unsubscribe();
  return snapshot;
}

/**
 * Render the "update available" toast with Install / Later actions.
 * Exposed for unit tests; the production caller is `runStartupCheck()`.
 */
export function emitUpdateAvailableToast(version: string): void {
  const toastId = addToast({
    message: m.update_available({ version }),
    type: "info",
    duration: null,
    actions: [
      {
        label: m.update_install(),
        onclick: () => {
          void startDownloadFromToast(toastId);
        },
      },
      {
        label: m.update_later(),
        onclick: () => removeToast(toastId),
      },
    ],
  });
}

/**
 * Drive the download-and-install flow from the startup toast, mutating
 * the same toast through the `downloading` → `ready` phases so the
 * user sees a coherent lifecycle.
 *
 * The toast mirrors per-chunk download progress by subscribing to
 * [`autoUpdateState`] for the lifetime of the download. Progress is
 * rendered as a thin bar beneath the message via the `progress` field
 * on [`ToastOptions`](./toast.ts) — a temporary surface until the
 * unified tasks drawer (cluster 0.3) takes over.
 */
async function startDownloadFromToast(toastId: string): Promise<void> {
  updateToast(toastId, {
    message: m.update_downloading({ percent: "0" }),
    actions: [],
    dismissible: false,
    duration: null,
    progress: 0,
  });

  // Mirror `autoUpdateState.downloadedBytes / totalBytes` into the toast
  // so the user sees a live progress bar instead of a spinner.
  const unsubscribe = autoUpdateState.subscribe((state) => {
    if (state.status !== "downloading") return;
    const total = state.totalBytes ?? 0;
    const done = state.downloadedBytes ?? 0;
    const fraction = total > 0 ? Math.min(1, done / total) : undefined;
    const percentLabel =
      fraction !== undefined ? String(Math.round(fraction * 100)) : "0";
    updateToast(toastId, {
      message: m.update_downloading({ percent: percentLabel }),
      progress: fraction,
      dismissible: false,
      duration: null,
    });
  });

  const outcome = await startInstallFlow().catch(
    () => "error" as UpdateStatus,
  );

  unsubscribe();

  if (outcome === "ready") {
    updateToast(toastId, {
      message: m.update_ready(),
      type: "success",
      dismissible: true,
      duration: null,
      progress: undefined,
      actions: [
        {
          label: m.update_restart(),
          onclick: () => {
            void relaunchApp();
          },
        },
      ],
    });
  } else if (outcome === "available") {
    // The re-auth dialog is showing — remove the transient progress
    // toast; the dialog now owns the UX until the user confirms or
    // cancels. If the user confirms, the restart toast will be emitted
    // from `confirmReauthAndInstall` → `downloadAndInstall` downstream.
    removeToast(toastId);
  } else {
    removeToast(toastId);
    addToast({
      message: m.update_error(),
      type: "error",
      duration: 5000,
    });
  }
}

// ---------------------------------------------------------------------------
// Tasks-drawer contract (Phase 6)
// ---------------------------------------------------------------------------

/**
 * Stable id reused across every emission of the auto-update
 * [`TaskEntry`](../types/tasks.ts). The aggregator store keys entries by
 * id, so using a constant here means successive update phases
 * (`checking → available → downloading → ready`) upsert a single row in
 * the drawer instead of piling up multiple rows.
 */
export const AUTO_UPDATE_TASK_ID = "auto-update";

/**
 * Remembered `startedAt` timestamp for the auto-update row.
 *
 * The Tauri updater plugin doesn't expose a single "I just started"
 * event — each phase transition is observed separately — so we stamp the
 * first emission and carry the same value forward, clearing the stamp
 * whenever the store settles back to `idle`/`up_to_date`.
 */
let autoUpdateStartedAt: number | null = null;

function updateTaskStartedAt(status: UpdateStatus): number {
  if (status === "checking" || status === "available") {
    autoUpdateStartedAt ??= Date.now();
  } else if (status === "error" || status === "ready") {
    autoUpdateStartedAt ??= Date.now();
  }
  return autoUpdateStartedAt ?? Date.now();
}

/**
 * Called from {@link resetAutoUpdateState} (and any future idle-reset
 * path) so the next lifecycle starts from a clean wall-clock.
 */
function clearAutoUpdateStartedAt(): void {
  autoUpdateStartedAt = null;
}

/**
 * Cancel an in-flight update download.
 *
 * `tauri-plugin-updater` doesn't expose a cancellation API yet (upstream
 * issue tracked in the spec) — the best the UI can do is drop its
 * reference to the in-flight handle and flip the store back to `idle` so
 * the drawer row is dismissed. The underlying HTTP request then observes
 * the abort the next time the runtime polls the stream; libgit2-style
 * partial cleanup is a non-goal until the plugin grows proper cancel
 * support.
 *
 * Exposed so the unified tasks-drawer router
 * (`src/lib/stores/tasks.ts::cancelTaskById`) can route `app_update`
 * cancellations to a single well-known entry point.
 */
export function cancelUpdateDownload(): void {
  currentUpdate = null;
  clearAutoUpdateStartedAt();
  autoUpdateState.set({ status: "idle" });
  needsReauthNotice.set(false);
}

/**
 * Derived, read-only view of the update lifecycle formatted as a
 * [`TaskEntry`](../types/tasks.ts) so the unified tasks drawer can render
 * it alongside AI background runs, long git fetches, etc.
 *
 * Maps `checking | available | downloading | ready | error` onto
 * {@link TaskEntry}; returns `null` for `idle` and `up_to_date` so the
 * drawer hides the row when there's nothing to show.
 *
 * The output matches the spec's `TaskEntry` shape (kind `"app_update"`,
 * `startedAt`/`finishedAt` ms timestamps, `"running" | "success" |
 * "error" | "cancelled"` status, declarative `actions`). The aggregator
 * store consumes this directly — no adapter.
 */
export const updateTask: Readable<TaskEntry | null> = derived(
  autoUpdateState,
  (state): TaskEntry | null => {
    switch (state.status) {
      case "idle":
      case "up_to_date":
        clearAutoUpdateStartedAt();
        return null;
      case "checking": {
        const startedAt = updateTaskStartedAt("checking");
        return {
          id: AUTO_UPDATE_TASK_ID,
          kind: "app_update",
          title: m.update_checking(),
          startedAt,
          status: "running",
          actions: [],
        };
      }
      case "available": {
        const startedAt = updateTaskStartedAt("available");
        return {
          id: AUTO_UPDATE_TASK_ID,
          kind: "app_update",
          title: m.update_available({ version: state.availableVersion ?? "" }),
          subtitle: state.releaseNotes,
          startedAt,
          status: "running",
          actions: [],
        };
      }
      case "downloading": {
        const startedAt = updateTaskStartedAt("checking");
        const total = state.totalBytes ?? 0;
        const done = state.downloadedBytes ?? 0;
        const fraction = total > 0 ? Math.min(1, done / total) : undefined;
        const percent =
          fraction !== undefined ? Math.round(fraction * 100) : undefined;
        const percentLabel = percent !== undefined ? String(percent) : "0";
        return {
          id: AUTO_UPDATE_TASK_ID,
          kind: "app_update",
          title: m.update_downloading({ percent: percentLabel }),
          startedAt,
          status: "running",
          progress: {
            determinate: percent !== undefined,
            current: done || undefined,
            total: total || undefined,
            percent,
          },
          actions: [],
        };
      }
      case "ready": {
        const startedAt = updateTaskStartedAt("ready");
        return {
          id: AUTO_UPDATE_TASK_ID,
          kind: "app_update",
          title: m.update_ready(),
          startedAt,
          finishedAt: Date.now(),
          status: "success",
          actions: [],
        };
      }
      case "error": {
        const startedAt = updateTaskStartedAt("error");
        return {
          id: AUTO_UPDATE_TASK_ID,
          kind: "app_update",
          title: m.update_error(),
          startedAt,
          finishedAt: Date.now(),
          status: "error",
          errorMessage: state.error,
          actions: [],
        };
      }
    }
  },
);
