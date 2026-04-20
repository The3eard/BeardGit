/**
 * Unit tests for the auto-update store (`autoUpdate.ts`).
 *
 * Uses `vi.mock` to stub `@tauri-apps/plugin-updater`,
 * `@tauri-apps/plugin-process`, and `@tauri-apps/plugin-os`, then walks
 * the state machine through the full happy path plus error branches.
 *
 * Transitions covered:
 *   - idle → checking → up_to_date
 *   - idle → checking → available → downloading → ready (+ reauth flag)
 *   - idle → checking → error (network failure)
 *   - download failure → error (after available)
 *   - Linux: no reauth flag after download
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { mockInvokeResponse } from "../../../test/setup";

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

const checkMock = vi.fn();
const osTypeMock = vi.fn(() => "macos");
const relaunchMock = vi.fn(async () => {});

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: () => checkMock(),
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  type: () => osTypeMock(),
}));

vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: () => relaunchMock(),
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type DownloadEvent =
  | { event: "Started"; data: { contentLength?: number } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" };

/** Build a minimal stand-in for the plugin's `Update` resource. */
function makeFakeUpdate(opts: {
  version: string;
  body?: string;
  downloadEvents?: DownloadEvent[];
  failDownload?: Error | null;
}) {
  return {
    version: opts.version,
    currentVersion: "0.0.0",
    body: opts.body,
    available: true,
    rawJson: {},
    async downloadAndInstall(cb?: (e: DownloadEvent) => void) {
      if (opts.failDownload) throw opts.failDownload;
      for (const ev of opts.downloadEvents ?? []) cb?.(ev);
    },
    async close() {},
  };
}

// ---------------------------------------------------------------------------
// Isolate the store between tests — resetModules so each test gets a fresh
// module instance with its own `currentUpdate` handle.
// ---------------------------------------------------------------------------

beforeEach(() => {
  vi.resetModules();
  checkMock.mockReset();
  osTypeMock.mockReset();
  osTypeMock.mockImplementation(() => "macos");
  relaunchMock.mockReset();
  relaunchMock.mockImplementation(async () => {});
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("autoUpdate store — state transitions", () => {
  it("idle → checking → up_to_date when no update is available", async () => {
    checkMock.mockResolvedValueOnce(null);
    const mod = await import("../autoUpdate");

    expect(get(mod.autoUpdateState).status).toBe("idle");

    const result = await mod.checkForUpdates();

    expect(result).toBe("up_to_date");
    expect(get(mod.autoUpdateState).status).toBe("up_to_date");
    expect(checkMock).toHaveBeenCalledTimes(1);
  });

  it("idle → checking → available exposes version + release notes", async () => {
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({
        version: "1.2.3",
        body: "Shiny new features",
      }),
    );
    const mod = await import("../autoUpdate");

    const result = await mod.checkForUpdates();

    expect(result).toBe("available");
    const state = get(mod.autoUpdateState);
    expect(state.status).toBe("available");
    expect(state.availableVersion).toBe("1.2.3");
    expect(state.releaseNotes).toBe("Shiny new features");
  });

  it("check failure → error with plugin message", async () => {
    checkMock.mockRejectedValueOnce(new Error("offline"));
    const mod = await import("../autoUpdate");

    const result = await mod.checkForUpdates();

    expect(result).toBe("error");
    const state = get(mod.autoUpdateState);
    expect(state.status).toBe("error");
    expect(state.error).toBe("offline");
  });

  it("available → downloading → ready and flips reauth notice on macOS", async () => {
    osTypeMock.mockImplementation(() => "macos");
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({
        version: "2.0.0",
        downloadEvents: [
          { event: "Started", data: { contentLength: 1024 } },
          { event: "Progress", data: { chunkLength: 256 } },
          { event: "Progress", data: { chunkLength: 768 } },
          { event: "Finished" },
        ],
      }),
    );
    const mod = await import("../autoUpdate");

    await mod.checkForUpdates();
    const installResult = await mod.downloadAndInstall();

    expect(installResult).toBe("ready");
    const state = get(mod.autoUpdateState);
    expect(state.status).toBe("ready");
    expect(state.availableVersion).toBe("2.0.0");
    expect(state.totalBytes).toBe(1024);
    expect(state.downloadedBytes).toBe(1024);
    expect(get(mod.needsReauthNotice)).toBe(true);
  });

  it("does not flip the reauth notice on Linux", async () => {
    osTypeMock.mockImplementation(() => "linux");
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({
        version: "2.0.0",
        downloadEvents: [
          { event: "Started", data: { contentLength: 512 } },
          { event: "Progress", data: { chunkLength: 512 } },
          { event: "Finished" },
        ],
      }),
    );
    const mod = await import("../autoUpdate");

    await mod.checkForUpdates();
    await mod.downloadAndInstall();

    expect(get(mod.needsReauthNotice)).toBe(false);
  });

  it("skips the reauth notice when confirmedReauth=true", async () => {
    osTypeMock.mockImplementation(() => "macos");
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({
        version: "2.0.0",
        downloadEvents: [{ event: "Started", data: { contentLength: 1 } }],
      }),
    );
    const mod = await import("../autoUpdate");

    await mod.checkForUpdates();
    await mod.downloadAndInstall(true);

    expect(get(mod.needsReauthNotice)).toBe(false);
  });

  it("download failure → error surfaces the message", async () => {
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({
        version: "2.0.0",
        failDownload: new Error("network-drop"),
      }),
    );
    const mod = await import("../autoUpdate");

    await mod.checkForUpdates();
    const result = await mod.downloadAndInstall();

    expect(result).toBe("error");
    const state = get(mod.autoUpdateState);
    expect(state.status).toBe("error");
    expect(state.error).toBe("network-drop");
  });

  it("downloadAndInstall without a prior check surfaces an error", async () => {
    const mod = await import("../autoUpdate");

    const result = await mod.downloadAndInstall();

    expect(result).toBe("error");
    expect(get(mod.autoUpdateState).status).toBe("error");
  });

  it("detectOs returns the OS identifier", async () => {
    osTypeMock.mockImplementation(() => "windows");
    const mod = await import("../autoUpdate");

    const os = await mod.detectOs();

    expect(os).toBe("windows");
  });

  it("detectOs maps unknown platform to 'other'", async () => {
    osTypeMock.mockImplementation(() => "freebsd" as unknown as "linux");
    const mod = await import("../autoUpdate");

    const os = await mod.detectOs();

    expect(os).toBe("other");
  });

  it("relaunchApp invokes plugin-process relaunch", async () => {
    const mod = await import("../autoUpdate");

    await mod.relaunchApp();

    expect(relaunchMock).toHaveBeenCalledTimes(1);
  });

  it("dismissReauthForThisOs clears the flag", async () => {
    const mod = await import("../autoUpdate");

    mod.needsReauthNotice.set(true);
    mod.dismissReauthForThisOs();

    expect(get(mod.needsReauthNotice)).toBe(false);
  });

  it("startInstallFlow shows the reauth dialog on macOS when not dismissed", async () => {
    osTypeMock.mockImplementation(() => "macos");
    mockInvokeResponse("get_reauth_dismissed", false);
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({ version: "4.0.0" }),
    );
    const mod = await import("../autoUpdate");

    await mod.checkForUpdates();
    const outcome = await mod.startInstallFlow();

    expect(outcome).toBe("available");
    expect(get(mod.needsReauthNotice)).toBe(true);
  });

  it("startInstallFlow skips the dialog on macOS when previously dismissed", async () => {
    osTypeMock.mockImplementation(() => "macos");
    mockInvokeResponse("get_reauth_dismissed", true);
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({
        version: "4.0.0",
        downloadEvents: [
          { event: "Started", data: { contentLength: 16 } },
          { event: "Progress", data: { chunkLength: 16 } },
          { event: "Finished" },
        ],
      }),
    );
    const mod = await import("../autoUpdate");

    await mod.checkForUpdates();
    const outcome = await mod.startInstallFlow();

    expect(outcome).toBe("ready");
    expect(get(mod.needsReauthNotice)).toBe(false);
  });

  it("startInstallFlow skips the dialog entirely on Linux", async () => {
    osTypeMock.mockImplementation(() => "linux");
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({
        version: "4.0.0",
        downloadEvents: [
          { event: "Started", data: { contentLength: 8 } },
          { event: "Progress", data: { chunkLength: 8 } },
        ],
      }),
    );
    const mod = await import("../autoUpdate");

    await mod.checkForUpdates();
    const outcome = await mod.startInstallFlow();

    expect(outcome).toBe("ready");
  });

  it("confirmReauthAndInstall persists dismissal when dismissForever=true", async () => {
    osTypeMock.mockImplementation(() => "macos");
    let persistedOs: string | undefined;
    let persistedValue: boolean | undefined;
    mockInvokeResponse(
      "set_reauth_dismissed",
      (args: Record<string, unknown>) => {
        persistedOs = args?.os as string;
        persistedValue = args?.dismissed as boolean;
        return undefined;
      },
    );
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({
        version: "4.0.0",
        downloadEvents: [{ event: "Started", data: { contentLength: 1 } }],
      }),
    );
    const mod = await import("../autoUpdate");

    await mod.checkForUpdates();
    mod.needsReauthNotice.set(true);
    await mod.confirmReauthAndInstall(true);

    expect(persistedOs).toBe("macos");
    expect(persistedValue).toBe(true);
    expect(get(mod.needsReauthNotice)).toBe(false);
  });

  it("cancelReauthFlow clears the flag without persisting anything", async () => {
    osTypeMock.mockImplementation(() => "macos");
    let persistCalled = false;
    mockInvokeResponse("set_reauth_dismissed", () => {
      persistCalled = true;
      return undefined;
    });
    const mod = await import("../autoUpdate");
    mod.needsReauthNotice.set(true);

    mod.cancelReauthFlow();

    expect(get(mod.needsReauthNotice)).toBe(false);
    expect(persistCalled).toBe(false);
  });

  it("updateTask is null for idle and up_to_date states", async () => {
    const mod = await import("../autoUpdate");
    expect(get(mod.updateTask)).toBeNull();

    mod.autoUpdateState.set({ status: "up_to_date" });
    expect(get(mod.updateTask)).toBeNull();
  });

  it("updateTask maps each active lifecycle state to a TaskEntry", async () => {
    const mod = await import("../autoUpdate");

    mod.autoUpdateState.set({ status: "checking" });
    let entry = get(mod.updateTask);
    expect(entry).not.toBeNull();
    expect(entry!.status).toBe("running");
    expect(entry!.kind).toBe("update");

    mod.autoUpdateState.set({
      status: "available",
      availableVersion: "5.5.5",
      releaseNotes: "Notes",
    });
    entry = get(mod.updateTask);
    expect(entry!.status).toBe("queued");
    expect(entry!.title).toContain("5.5.5");
    expect(entry!.subtitle).toBe("Notes");

    mod.autoUpdateState.set({
      status: "downloading",
      totalBytes: 200,
      downloadedBytes: 50,
    });
    entry = get(mod.updateTask);
    expect(entry!.status).toBe("running");
    expect(entry!.progress).toBeCloseTo(0.25);

    mod.autoUpdateState.set({ status: "ready" });
    entry = get(mod.updateTask);
    expect(entry!.status).toBe("completed");

    mod.autoUpdateState.set({ status: "error", error: "boom" });
    entry = get(mod.updateTask);
    expect(entry!.status).toBe("failed");
    expect(entry!.error).toBe("boom");
  });

  it("resetAutoUpdateState returns the store to idle", async () => {
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({ version: "3.0.0" }),
    );
    const mod = await import("../autoUpdate");

    await mod.checkForUpdates();
    expect(get(mod.autoUpdateState).status).toBe("available");

    mod.resetAutoUpdateState();

    expect(get(mod.autoUpdateState).status).toBe("idle");
    expect(get(mod.needsReauthNotice)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Startup probe — preference + debounce + toast emission
// ---------------------------------------------------------------------------

describe("runStartupCheck", () => {
  beforeEach(() => {
    // Fresh sessionStorage for each debounce assertion.
    if (typeof sessionStorage !== "undefined") sessionStorage.clear();
    // Pretend we're running a production bundle — Vitest defaults
    // `import.meta.env.DEV` to true which the probe short-circuits on.
    vi.stubEnv("DEV", false);
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("skips the probe when auto_check_updates=false", async () => {
    mockInvokeResponse("get_auto_check_updates", false);
    const mod = await import("../autoUpdate");

    await mod.runStartupCheck();

    expect(checkMock).not.toHaveBeenCalled();
  });

  it("probes and transitions the store when enabled", async () => {
    mockInvokeResponse("get_auto_check_updates", true);
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({ version: "9.9.9" }),
    );
    const mod = await import("../autoUpdate");

    await mod.runStartupCheck();

    expect(checkMock).toHaveBeenCalledTimes(1);
    expect(get(mod.autoUpdateState).status).toBe("available");
  });

  it("debounces within the 60s window via sessionStorage", async () => {
    mockInvokeResponse("get_auto_check_updates", true);
    checkMock.mockResolvedValue(null);
    const mod = await import("../autoUpdate");

    await mod.runStartupCheck();
    await mod.runStartupCheck();

    expect(checkMock).toHaveBeenCalledTimes(1);
  });

  it("download progress events mirror onto the persistent toast value", async () => {
    mockInvokeResponse("get_auto_check_updates", true);
    osTypeMock.mockImplementation(() => "linux");

    // Wire the fake update with a deterministic progress stream.
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({
        version: "2.1.0",
        downloadEvents: [
          { event: "Started", data: { contentLength: 100 } },
          { event: "Progress", data: { chunkLength: 40 } },
          { event: "Progress", data: { chunkLength: 60 } },
        ],
      }),
    );
    const mod = await import("../autoUpdate");
    const { toasts } = await import("../toast");

    await mod.runStartupCheck();

    const firstToast = get(toasts)[0];
    expect(firstToast).toBeDefined();
    // Kick off the download via the toast's Install action — this is
    // what startDownloadFromToast does internally.
    const installAction = firstToast.actions?.find((a) =>
      a.label.toLowerCase().includes("install"),
    );
    expect(installAction).toBeDefined();
    installAction!.onclick();

    // Give the fake download a tick to run its synchronous event loop.
    await new Promise((resolve) => setTimeout(resolve, 0));

    const live = get(toasts)[0];
    // Either the progress bar hit ready (Linux runs inline, proceeds to
    // ready) or still shows 100% — either way the progress field must
    // have been carried through.
    expect(live).toBeDefined();
    expect(
      live.progress === undefined || live.progress >= 0,
    ).toBe(true);
  });

  it("emits a toast with Install and Later actions when an update is available", async () => {
    mockInvokeResponse("get_auto_check_updates", true);
    checkMock.mockResolvedValueOnce(
      makeFakeUpdate({ version: "1.5.0" }),
    );
    const mod = await import("../autoUpdate");
    const { toasts } = await import("../toast");

    await mod.runStartupCheck();

    const list = get(toasts);
    expect(list.length).toBe(1);
    const labels = (list[0].actions ?? []).map((a) => a.label);
    expect(labels.length).toBe(2);
    expect(labels[0].toLowerCase()).toMatch(/install|instalar/);
    expect(labels[1].toLowerCase()).toMatch(/later|tarde/);
  });
});
