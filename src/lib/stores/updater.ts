/**
 * Updater store — checks for app updates on launch and manages
 * the download → restart flow via the toast system.
 */

import { addToast, updateToast, removeToast } from "./toast";
import * as m from "$lib/paraglide/messages";
import type { Update } from "@tauri-apps/plugin-updater";

/**
 * Check for available updates and show a toast if one is found.
 * Called once from +layout.svelte on mount. Silently no-ops in
 * dev mode or if the updater plugin is unavailable.
 */
export async function checkForAppUpdate(): Promise<void> {
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();
    if (!update) return;

    const version = update.version;

    const toastId = addToast({
      message: m.update_available({ version }),
      type: "info",
      duration: null,
      actions: [
        {
          label: m.update_download(),
          onclick: () => downloadUpdate(toastId, update),
        },
      ],
    });
  } catch {
    // Silently ignore — dev mode, no network, plugin not available
  }
}

async function downloadUpdate(toastId: string, update: Update): Promise<void> {
  let contentLength = 0;
  let downloaded = 0;

  updateToast(toastId, {
    message: m.update_downloading({ percent: "0" }),
    actions: [],
    dismissible: false,
  });

  try {
    await update.downloadAndInstall((event) => {
      if (event.event === "Started") {
        contentLength = event.data.contentLength ?? 0;
      } else if (event.event === "Progress") {
        downloaded += event.data.chunkLength;
        const percent = contentLength > 0
          ? Math.round((downloaded / contentLength) * 100)
          : 0;
        updateToast(toastId, {
          message: m.update_downloading({ percent: String(percent) }),
        });
      } else if (event.event === "Finished") {
        // Handled below
      }
    });

    updateToast(toastId, {
      message: m.update_ready(),
      type: "success",
      dismissible: true,
      actions: [
        {
          label: m.update_restart(),
          onclick: () => restartApp(toastId),
        },
      ],
    });
  } catch {
    removeToast(toastId);
    addToast({
      message: m.update_error(),
      type: "error",
      duration: 5000,
    });
  }
}

async function restartApp(toastId: string): Promise<void> {
  removeToast(toastId);
  try {
    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
  } catch {
    addToast({
      message: m.update_error(),
      type: "error",
      duration: 5000,
    });
  }
}
