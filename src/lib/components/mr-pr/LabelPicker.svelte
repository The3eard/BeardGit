<!--
  LabelPicker — searchable dropdown of repository labels with multi-select
  and an Apply button. Pre-selects labels currently assigned to the MR/PR
  and computes the added/removed diff on submit.

  Populates the `repoLabels` store on mount if empty.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { repoLabels, loadRepoLabels } from "../../stores/mr-pr";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    /** Labels currently assigned to the MR/PR. */
    current: string[];
    /** Called with the added and removed label names on Apply. */
    onApply: (added: string[], removed: string[]) => void;
    /** Called when the user dismisses the picker. */
    onCancel: () => void;
  }

  let { current, onApply, onCancel }: Props = $props();

  let query = $state("");
  // Snapshot `current` at mount time — the picker intentionally only
  // captures the initial value and then operates on its own selection set.
  // svelte-ignore state_referenced_locally
  let selected: Set<string> = $state(new Set<string>(current));

  onMount(() => {
    if ($repoLabels.length === 0) {
      loadRepoLabels();
    }
  });

  let filtered = $derived(
    $repoLabels.filter(
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

<button class="overlay" type="button" aria-label={m.mrpr_cancel()} onclick={onCancel}></button>
<!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
<div class="picker" role="dialog" tabindex="-1" onkeydown={handleKeydown}>
  <input
    class="picker-search"
    type="text"
    bind:value={query}
    placeholder={m.mrpr_label_picker_search()}
  />
  <div class="picker-list">
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
      <div class="picker-empty">{m.mrpr_label_picker_empty()}</div>
    {/if}
  </div>
  <div class="picker-actions">
    <button type="button" class="btn-secondary" onclick={onCancel}
      >{m.mrpr_cancel()}</button
    >
    <button type="button" class="btn-primary" onclick={apply}
      >{m.mrpr_label_picker_apply()}</button
    >
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
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
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  }
  .picker-search {
    margin: 12px;
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 13px;
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
    font-size: 12px;
    cursor: pointer;
    text-align: left;
  }
  .picker-item:hover {
    background: rgba(255, 255, 255, 0.05);
  }
  .picker-item.selected {
    background: rgba(88, 166, 255, 0.15);
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
    font-size: 11px;
    margin-left: 4px;
  }
  .picker-empty {
    text-align: center;
    padding: 16px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .picker-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px;
    border-top: 1px solid var(--border);
  }
  .btn-primary {
    padding: 5px 12px;
    background: var(--accent-blue);
    color: #fff;
    border: none;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  .btn-secondary {
    padding: 5px 12px;
    background: none;
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
</style>
