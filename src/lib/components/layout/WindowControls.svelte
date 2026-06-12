<!--
  WindowControls — minimize / maximize / close buttons for frameless
  windows on Windows and Linux.

  On those platforms the main window ships `decorations: false`
  (tauri.windows.conf.json / tauri.linux.conf.json) so the tab bar can
  double as the title bar; this component supplies the window buttons
  the OS no longer draws. On macOS (native traffic lights, overlay
  title bar) and outside Tauri (browser dev, Playwright) it renders
  nothing.
-->
<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import * as m from "$lib/paraglide/messages";

  let visible = $state(false);
  let maximized = $state(false);
  let unlistenResize: UnlistenFn | null = null;

  onMount(async () => {
    try {
      const { type } = await import("@tauri-apps/plugin-os");
      const platform = type();
      if (platform !== "windows" && platform !== "linux") return;
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      visible = true;
      maximized = await win.isMaximized();
      unlistenResize = await win.onResized(async () => {
        maximized = await win.isMaximized();
      });
    } catch {
      // Not running under Tauri — keep hidden.
    }
  });

  onDestroy(() => {
    unlistenResize?.();
  });

  async function minimize() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  }

  async function toggleMaximize() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().toggleMaximize();
  }

  async function close() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  }
</script>

{#if visible}
  <div class="window-controls" data-testid="window-controls">
    <button
      type="button"
      class="wc-btn"
      aria-label={m.window_controls_minimize()}
      title={m.window_controls_minimize()}
      onclick={minimize}
    >
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <line x1="0" y1="5" x2="10" y2="5" stroke="currentColor" stroke-width="1" />
      </svg>
    </button>
    <button
      type="button"
      class="wc-btn"
      aria-label={maximized ? m.window_controls_restore() : m.window_controls_maximize()}
      title={maximized ? m.window_controls_restore() : m.window_controls_maximize()}
      onclick={toggleMaximize}
    >
      {#if maximized}
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <rect x="0.5" y="2.5" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1" />
          <path d="M 2.5 2.5 V 0.5 H 9.5 V 7.5 H 7.5" fill="none" stroke="currentColor" stroke-width="1" />
        </svg>
      {:else}
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1" />
        </svg>
      {/if}
    </button>
    <button
      type="button"
      class="wc-btn wc-btn--close"
      aria-label={m.window_controls_close()}
      title={m.window_controls_close()}
      onclick={close}
    >
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <line x1="0" y1="0" x2="10" y2="10" stroke="currentColor" stroke-width="1" />
        <line x1="10" y1="0" x2="0" y2="10" stroke="currentColor" stroke-width="1" />
      </svg>
    </button>
  </div>
{/if}

<style>
  .window-controls {
    display: flex;
    align-items: stretch;
    align-self: stretch;
    flex-shrink: 0;
    /* Bleed to the bar's right edge — the bar's own padding would
       otherwise float the close button away from the corner. */
    margin-right: -8px;
    margin-left: 4px;
  }

  .wc-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: default;
    transition: background 0.1s ease, color 0.1s ease;
  }

  .wc-btn:hover {
    background: var(--overlay-hover);
    color: var(--text-primary);
  }

  .wc-btn--close:hover {
    background: var(--accent-red);
    color: var(--bg-primary);
  }
</style>
