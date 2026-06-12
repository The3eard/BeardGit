<script lang="ts">
  import { shortcuts, showCheatSheet, toggleCheatSheet, formatShortcut } from "../../stores/shortcuts";
  import type { Shortcut } from "../../stores/shortcuts";
  import { IconButton } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";

  let overlayEl: HTMLDivElement | undefined = $state();

  function handleBackdrop(e: MouseEvent) {
    if (overlayEl && !overlayEl.contains(e.target as Node)) {
      toggleCheatSheet();
    }
  }

  // Listen for Escape to close the overlay when visible
  $effect(() => {
    if (!$showCheatSheet) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        toggleCheatSheet();
      }
    }
    window.addEventListener("keydown", onKey, { capture: true });
    return () => window.removeEventListener("keydown", onKey, { capture: true });
  });

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
  <div class="overlay-backdrop" onclick={handleBackdrop} onkeydown={() => {}} role="presentation">
    <div class="shortcut-overlay" bind:this={overlayEl}>
      <div class="overlay-header">
        <h2>{m.shortcuts_title()}</h2>
        <IconButton icon={"\uF00D"} description={m.tooltip_close()} size="lg" onclick={toggleCheatSheet} />
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
    background: rgba(0, 0, 0, 0.5); /* beardgit:allow-hex: modal backdrop neutral */
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .shortcut-overlay {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: var(--shadow-modal);
    width: min(720px, 90vw);
    max-height: min(600px, 80vh);
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
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
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
    font-size: var(--font-size-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent-primary);
    margin: 0 0 8px;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--border);
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 0;
    gap: 12px;
  }

  .shortcut-label {
    font-size: var(--font-size-md);
    color: var(--text-primary);
  }

  .shortcut-keys {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 8px;
    white-space: nowrap;
  }
</style>
