<!--
  LabelPicker — shared searchable dropdown of repository labels with
  multi-select and an Apply button. Used by both issues and mr-pr consumers.

  Takes the label list + loading state via props so consumers can point
  it at any label-producing store.
-->
<script lang="ts">
  import type { Label } from "../../types";
  import * as m from "$lib/paraglide/messages";
  import Button from "$lib/components/ui/Button.svelte";

  interface Props {
    /** All repository labels to show. */
    labels: Label[];
    /** Whether the labels list is currently loading. */
    loading: boolean;
    /** Labels currently selected (names). */
    current: string[];
    /** Fired on Apply with the added / removed label names. */
    onApply: (added: string[], removed: string[]) => void;
    /** Fired when the user dismisses the picker. */
    onCancel: () => void;
  }

  let { labels, loading, current, onApply, onCancel }: Props = $props();

  let query = $state("");
  // Snapshot `current` at mount time — the picker intentionally only
  // captures the initial value and then operates on its own selection set.
  // svelte-ignore state_referenced_locally
  let selected: Set<string> = $state(new Set<string>(current));

  let filtered = $derived(
    labels.filter(
      (l) =>
        query.trim() === "" ||
        l.name.toLowerCase().includes(query.trim().toLowerCase()),
    ),
  );

  function toggle(name: string) {
    if (selected.has(name)) selected.delete(name);
    else selected.add(name);
    selected = new Set(selected);
  }

  function apply() {
    const currentSet = new Set(current);
    const added: string[] = [];
    const removed: string[] = [];
    for (const n of selected) if (!currentSet.has(n)) added.push(n);
    for (const n of currentSet) if (!selected.has(n)) removed.push(n);
    onApply(added, removed);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
  }
</script>

<button class="overlay" type="button" aria-label={m.issues_cancel()} onclick={onCancel}></button>
<!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
<div class="picker" role="dialog" tabindex="-1" onkeydown={handleKeydown}>
  <input
    class="picker-search"
    type="text"
    bind:value={query}
    placeholder={m.issues_label_picker_search()}
  />
  <div class="picker-list">
    {#if loading && labels.length === 0}
      <div class="picker-empty">{m.issues_loading()}</div>
    {:else}
      {#each filtered as label}
        <button
          class="picker-item"
          class:selected={selected.has(label.name)}
          type="button"
          onclick={() => toggle(label.name)}
        >
          <span
            class="color-swatch"
            style:background={label.color ? `#${label.color}` : "var(--border)"}
          ></span>
          <span class="picker-name">{label.name}</span>
          {#if label.description}
            <span class="picker-desc">{label.description}</span>
          {/if}
        </button>
      {/each}
      {#if filtered.length === 0}
        <div class="picker-empty">{m.issues_label_picker_empty()}</div>
      {/if}
    {/if}
  </div>
  <div class="picker-actions">
    <Button variant="neutral" onclick={onCancel}>{m.issues_cancel()}</Button>
    <Button variant="primary" onclick={apply}>{m.issues_label_picker_apply()}</Button>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4); /* beardgit:allow-hex: modal backdrop neutral */
    z-index: 99;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .picker {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 100;
    width: 360px;
    max-height: 480px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-modal);
  }
  .picker-search {
    margin: 12px;
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: var(--font-size-md);
  }
  .picker-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px;
  }
  .picker-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 8px;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    cursor: pointer;
    text-align: left;
  }
  .picker-item:hover {
    background: color-mix(in srgb, var(--text-primary) 5%, transparent);
  }
  .picker-item.selected {
    background: color-mix(in srgb, var(--accent-primary) 15%, transparent);
  }
  .color-swatch {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .picker-name {
    font-weight: 500;
  }
  .picker-desc {
    color: var(--text-secondary);
    font-size: var(--font-size-xs);
    margin-left: 4px;
  }
  .picker-empty {
    text-align: center;
    padding: 16px;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }
  .picker-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px;
    border-top: 1px solid var(--border);
  }
</style>
