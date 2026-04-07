<script lang="ts">
  export interface MenuItem {
    label?: string;
    action?: () => void;
    separator?: boolean;
    disabled?: boolean;
  }

  let {
    items,
    x,
    y,
    visible,
    onClose,
  }: {
    items: MenuItem[];
    x: number;
    y: number;
    visible: boolean;
    onClose: () => void;
  } = $props();

  function handleClick(item: MenuItem) {
    if (item.disabled) return;
    item.action?.();
    onClose();
  }

  function handleBackdrop() {
    onClose();
  }
</script>

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={handleBackdrop} onkeydown={(e) => { if (e.key === 'Escape') handleBackdrop(); }} oncontextmenu={(e) => { e.preventDefault(); handleBackdrop(); }}></div>
  <div class="context-menu" style="left: {x}px; top: {y}px">
    {#each items as item}
      {#if item.separator}
        <div class="separator"></div>
      {:else}
        <button
          class="menu-item"
          class:disabled={item.disabled}
          onclick={() => handleClick(item)}
        >
          {item.label}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 999;
  }

  .context-menu {
    position: fixed;
    z-index: 1000;
    background: var(--bg-toolbar);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 0;
    min-width: 180px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  }

  .menu-item {
    display: block;
    width: 100%;
    padding: 6px 12px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }

  .menu-item:hover:not(.disabled) {
    background: rgba(88, 166, 255, 0.15);
  }

  .menu-item.disabled {
    color: var(--text-secondary);
    opacity: 0.5;
    cursor: not-allowed;
  }

  .separator {
    height: 1px;
    background: var(--border);
    margin: 4px 0;
  }
</style>
