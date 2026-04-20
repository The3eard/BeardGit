<script lang="ts">
  import { onMount } from "svelte";
  import "../app.css";
  import { tryAutoConnect } from "$lib/stores/provider";
  import { initLocale } from "$lib/stores/locale";
  import { initTaskStore, cleanupTaskStore } from "$lib/stores/taskPanel";
  import { initUiScale } from "$lib/stores/theme";
  import { initShortcutListener } from "$lib/stores/shortcuts";
  import { runStartupCheck } from "$lib/stores/autoUpdate";
  import { openProjectTab, closeTab } from "$lib/stores/projects";
  import ToastContainer from "$lib/components/ui/ToastContainer.svelte";
  let { children } = $props();

  /**
   * E2E test hooks — only exposed when BEARDGIT_E2E=true is set at build
   * time (the tauri-driver / docker harness sets this). The production
   * bundle strips the window.__E2E__ surface to avoid leaking test APIs
   * to end users. See e2e/helpers/project.ts for consumers.
   */
  if (typeof window !== "undefined" && import.meta.env.VITE_BEARDGIT_E2E === "true") {
    (window as unknown as Record<string, unknown>).__E2E__ = {
      openProject: (path: string) => openProjectTab(path),
      closeTab: (index: number) => closeTab(index),
    };
  }

  // Disable default browser context menu globally
  function handleContextMenu(e: MouseEvent) {
    // Allow context menu on input/textarea for copy/paste
    const target = e.target as HTMLElement | null;
    if (target?.tagName === 'INPUT' || target?.tagName === 'TEXTAREA') return;
    e.preventDefault();
  }

  onMount(() => {
    initLocale();
    initUiScale();
    tryAutoConnect();
    initTaskStore();
    runStartupCheck();
    const cleanupShortcuts = initShortcutListener();
    return () => {
      cleanupTaskStore();
      cleanupShortcuts();
    };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div oncontextmenu={handleContextMenu}>
  {@render children()}
</div>

<ToastContainer />

<style>
  div {
    display: contents;
  }
</style>
