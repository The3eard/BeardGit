<!--
  Popup context menu with optional one-level submenus.

  Flat items keep the original shape: `label`, `action`, `separator`,
  `disabled`. Items with a `children` array render a right-pointing
  chevron; hovering the parent opens a flyout anchored to its right
  edge. Nested submenus are intentionally out of scope.
-->
<script lang="ts">
  /** Single menu item. Leaf when `action` is set, parent when `children` is set. */
  export interface MenuItem {
    label?: string;
    action?: () => void;
    separator?: boolean;
    disabled?: boolean;
    /** One-level submenu. Parent items with children do not fire `action`. */
    children?: MenuItem[];
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

  /** Index (within `items`) of the currently-open submenu, or -1. */
  let openSubmenu = $state(-1);

  function handleClick(item: MenuItem) {
    if (item.disabled) return;
    if (item.children && item.children.length > 0) return; // parents don't fire
    item.action?.();
    onClose();
  }

  function handleChildClick(child: MenuItem) {
    if (child.disabled) return;
    child.action?.();
    onClose();
  }

  function handleBackdrop() {
    onClose();
  }
</script>

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="backdrop"
    onclick={handleBackdrop}
    onkeydown={(e) => { if (e.key === 'Escape') handleBackdrop(); }}
    oncontextmenu={(e) => { e.preventDefault(); handleBackdrop(); }}
  ></div>
  <div class="context-menu" style="left: {x}px; top: {y}px">
    {#each items as item, i}
      {#if item.separator}
        <div class="separator"></div>
      {:else}
        <div
          class="menu-item-wrap"
          onmouseleave={() => { if (openSubmenu === i) openSubmenu = -1; }}
          role="none"
        >
          <button
            class="menu-item"
            class:disabled={item.disabled}
            class:has-children={!!item.children}
            onclick={() => handleClick(item)}
            onmouseenter={() => (openSubmenu = item.children ? i : -1)}
          >
            <span
              class="menu-item-label"
              onmouseenter={() => (openSubmenu = item.children ? i : -1)}
              role="none"
            >{item.label}</span>
            {#if item.children && item.children.length > 0}
              <span class="submenu-chevron nf">{''}</span>
            {/if}
          </button>
          {#if item.children && openSubmenu === i}
            <div class="context-menu submenu">
              {#each item.children as child}
                {#if child.separator}
                  <div class="separator"></div>
                {:else}
                  <button
                    class="menu-item"
                    class:disabled={child.disabled}
                    onclick={() => handleChildClick(child)}
                  >
                    {child.label}
                  </button>
                {/if}
              {/each}
            </div>
          {/if}
        </div>
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

  .menu-item-wrap {
    position: relative;
  }

  .menu-item {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 6px 12px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }

  .menu-item-label {
    flex: 1;
  }

  .submenu-chevron {
    font-size: 9px;
    color: var(--text-secondary);
    margin-left: 8px;
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

  .submenu {
    position: absolute;
    top: 0;
    left: 100%;
    margin-left: 2px;
  }
</style>
