<script lang="ts">
  import { shortcuts, showCheatSheet, toggleCheatSheet, formatShortcut } from "../../stores/shortcuts";
  import type { Shortcut } from "../../stores/shortcuts";
  import * as m from "$lib/paraglide/messages";

  let overlayEl: HTMLDivElement | undefined = $state();

  function handleBackdrop(e: MouseEvent) {
    if (overlayEl && !overlayEl.contains(e.target as Node)) {
      toggleCheatSheet();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape" || e.key === "?") {
      e.preventDefault();
      toggleCheatSheet();
    }
  }

  /** Group shortcuts by category. */
  let grouped = $derived.by(() => {
    const map = new Map<string, Shortcut[]>();
    for (const s of $shortcuts) {
      // Skip duplicate search shortcut from display
      if (s.id === "graph.searchMod") continue;
      const list = map.get(s.category) ?? [];
      list.push(s);
      map.set(s.category, list);
    }
    return map;
  });
</script>

{#if $showCheatSheet}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay-backdrop" onclick={handleBackdrop} onkeydown={handleKeyDown}>
    <div class="shortcut-overlay" bind:this={overlayEl}>
      <div class="overlay-header">
        <h2>{m.shortcuts_title()}</h2>
        <button class="close-btn" onclick={toggleCheatSheet}>{"\uEA76"}</button>
      </div>
      <div class="overlay-grid">
        {#each [...grouped.entries()] as [category, items]}
          <div class="category-section">
            <h3 class="category-title">{category}</h3>
            {#each items as shortcut}
              <div class="shortcut-row">
                <span class="shortcut-label">{shortcut.label}</span>
                <kbd class="shortcut-keys">{formatShortcut(shortcut.keys)}</kbd>
              </div>
            {/each}
          </div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .shortcut-overlay {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    width: min(640px, 90vw);
    max-height: min(520px, 80vh);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .overlay-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--border);
  }

  .overlay-header h2 {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 16px;
    font-family: var(--font-icons);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    transition: background 0.1s;
  }

  .close-btn:hover {
    background: var(--overlay-hover);
  }

  .overlay-grid {
    padding: 12px 20px 20px;
    overflow-y: auto;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px 32px;
  }

  .category-section {
    break-inside: avoid;
  }

  .category-title {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    margin: 0 0 6px;
    padding-bottom: 4px;
    border-bottom: 1px solid var(--border);
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 3px 0;
  }

  .shortcut-label {
    font-size: 12px;
    color: var(--text-primary);
  }

  .shortcut-keys {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 6px;
    white-space: nowrap;
  }
</style>
