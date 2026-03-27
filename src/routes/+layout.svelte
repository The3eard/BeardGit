<script lang="ts">
  import { onMount } from "svelte";
  import "../app.css";
  import { tryAutoConnect } from "$lib/stores/provider";
  import { initLocale } from "$lib/stores/locale";
  import { initTaskStore, cleanupTaskStore } from "$lib/stores/tasks";
  let { children } = $props();

  // Disable default browser context menu globally
  function handleContextMenu(e: MouseEvent) {
    // Allow context menu on input/textarea for copy/paste
    const target = e.target as HTMLElement | null;
    if (target?.tagName === 'INPUT' || target?.tagName === 'TEXTAREA') return;
    e.preventDefault();
  }

  onMount(() => {
    initLocale();
    tryAutoConnect();
    initTaskStore();
    return () => cleanupTaskStore();
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div oncontextmenu={handleContextMenu}>
  {@render children()}
</div>

<style>
  div {
    height: 100vh;
    display: contents;
  }
</style>
