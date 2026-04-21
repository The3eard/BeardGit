<script lang="ts">
  import { onMount } from "svelte";
  import "../app.css";
  import { tryAutoConnect } from "$lib/stores/provider";
  import { initLocale } from "$lib/stores/locale";
  import { initTaskStore, cleanupTaskStore } from "$lib/stores/taskPanel";
  import { initTasksStore, stopTasksStore } from "$lib/stores/tasks";
  import { initUiScale } from "$lib/stores/theme";
  import { initShortcutListener } from "$lib/stores/shortcuts";
  import { runStartupCheck } from "$lib/stores/autoUpdate";
  import { openProjectTab, closeTab } from "$lib/stores/projects";
  import {
    startMutationListener,
    stopMutationListener,
  } from "$lib/stores/mutations";
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

  let stopMutations: (() => void) | null = null;

  onMount(() => {
    initLocale();
    initUiScale();
    tryAutoConnect();
    initTaskStore();
    // Unified tasks drawer aggregator — wires the 3 bridges (task://update
    // Tauri events, aiBackgroundRuns, autoUpdate.updateTask) into the
    // statusbar's Tasks slot count + the drawer's feed. Missing this call
    // was the root cause of the statusbar showing zero active tasks even
    // when git ops / AI runs / update downloads were in flight.
    void initTasksStore();
    // `startMutationListener` registers the single `project-mutated` Tauri
    // listener so store refreshes happen reactively on backend mutations
    // (Phase 2 of the reactivity foundation). We stash the teardown in a
    // closure-scoped handle so Vite HMR (which re-runs onMount on module
    // reload) doesn't orphan the listener.
    void startMutationListener().then(() => {
      stopMutations = stopMutationListener;
    });
    runStartupCheck();
    const cleanupShortcuts = initShortcutListener();
    return () => {
      stopMutations?.();
      cleanupTaskStore();
      stopTasksStore();
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
