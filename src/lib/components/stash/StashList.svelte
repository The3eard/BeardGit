<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { stashes, selectedStashIndex, doStashPush, doStashApply, doStashPop, doStashDrop, selectStash } from "../../stores/stashes";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
  import List from "../common/List.svelte";
  import { formatRelativeTimeUnix } from "../../utils/time";
  import { shortOid } from "../../utils/git";

  let showStashInput = $state(false);
  let stashMessage = $state("");
  let confirmDrop = $state<number | null>(null);

  async function handleStashPush() {
    const msg = stashMessage.trim() || null;
    await doStashPush(msg);
    stashMessage = "";
    showStashInput = false;
  }

  function handleCancelStash() {
    stashMessage = "";
    showStashInput = false;
  }

  function handleStashKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      handleStashPush();
    } else if (e.key === "Escape") {
      handleCancelStash();
    }
  }

  function getKey(entry: { index: number }): string {
    return String(entry.index);
  }

  let selectedKey = $derived(
    $selectedStashIndex !== null && $stashes.some((s) => s.index === $selectedStashIndex)
      ? String($selectedStashIndex)
      : null,
  );

  function handleSelect(entry: { index: number }) {
    selectStash(entry.index);
  }
</script>

<List
  items={$stashes}
  loading={false}
  title="STASHES"
  {selectedKey}
  {getKey}
  emptyMessage={m.stash_empty()}
  onSelect={handleSelect}
>
  {#snippet headerActions()}
    {#if showStashInput}
      <div class="stash-input-row">
        <input
          type="text"
          class="stash-input"
          placeholder={m.stash_message_placeholder()}
          bind:value={stashMessage}
          onkeydown={handleStashKeydown}
        />
        <button class="btn btn-small btn-confirm" onclick={handleStashPush}>&#x2713;</button>
        <button class="btn btn-small btn-cancel" onclick={handleCancelStash}>&#x2715;</button>
      </div>
    {:else}
      <button class="btn btn-stash" onclick={() => (showStashInput = true)}>
        {m.stash_button()}
      </button>
    {/if}
  {/snippet}

  {#snippet row({ item })}
    <div class="stash-content">
      <div class="stash-top">
        <span class="stash-message">{item.message || `stash@{${item.index}}`}</span>
        <span class="stash-time">{formatRelativeTimeUnix(item.timestamp)}</span>
      </div>
      <div class="stash-bottom-container">
        <div class="stash-bottom">
          <span class="stash-branch">{m.stash_on_branch({ branch: item.branch })}</span>
          <span class="stash-oid">{shortOid(item.oid)}</span>
        </div>
        <div class="stash-bottom-hover">
          <div class="stash-actions">
            <button
              class="action-btn action-btn-apply"
              title={m.stash_apply()}
              onclick={(e: MouseEvent) => { e.stopPropagation(); doStashApply(item.index); }}
            >{m.stash_apply()}</button>
            <button
              class="action-btn action-btn-pop"
              title={m.stash_pop()}
              onclick={(e: MouseEvent) => { e.stopPropagation(); doStashPop(item.index); }}
            >{m.stash_pop()}</button>
            <button
              class="action-btn action-btn-danger"
              title={m.stash_drop()}
              onclick={(e: MouseEvent) => { e.stopPropagation(); confirmDrop = item.index; }}
            >{m.stash_drop()}</button>
          </div>
          <span class="stash-oid">{shortOid(item.oid)}</span>
        </div>
      </div>
    </div>
  {/snippet}
</List>

{#if confirmDrop !== null}
  {@const dropEntry = $stashes.find((e) => e.index === confirmDrop)}
  <ConfirmDialog
    title={m.stash_confirm_drop_title()}
    detail={dropEntry ? `${dropEntry.message || `stash@{${dropEntry.index}}`}\n${shortOid(dropEntry.oid)}` : `stash@{${confirmDrop}}`}
    message={m.stash_confirm_drop_message()}
    confirmLabel={m.stash_drop()}
    destructive={true}
    onConfirm={() => {
      // svelte-ignore state_referenced_locally
      doStashDrop(confirmDrop!);
      confirmDrop = null;
    }}
    onCancel={() => (confirmDrop = null)}
  />
{/if}

<style>
  .stash-content {
    display: flex;
    flex-direction: column;
    gap: 3px;
    width: 100%;
  }

  .stash-input-row {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .stash-input {
    flex: 1;
    padding: 5px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
  }

  .stash-input:focus {
    border-color: var(--accent-blue);
  }

  .btn-small {
    padding: 4px 8px;
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
  }

  .btn-small:hover {
    background: color-mix(in srgb, var(--text-primary) 10%, transparent);
  }

  .btn-stash {
    padding: 6px 12px;
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-stash:hover {
    background: color-mix(in srgb, var(--text-primary) 10%, transparent);
  }

  .stash-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .stash-message {
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .stash-time {
    font-size: 11px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .stash-bottom-container {
    display: grid;
  }

  .stash-bottom,
  .stash-bottom-hover {
    grid-area: 1 / 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .stash-bottom-hover {
    visibility: hidden;
  }

  :global(.list-row:hover) .stash-bottom {
    visibility: hidden;
  }

  :global(.list-row:hover) .stash-bottom-hover {
    visibility: visible;
  }

  .stash-branch {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .stash-oid {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .stash-actions {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .action-btn {
    padding: 2px 8px;
    background: color-mix(in srgb, var(--text-primary) 8%, transparent);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 4px;
    font-size: 10px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .action-btn:hover {
    background: color-mix(in srgb, var(--text-primary) 15%, transparent);
  }

  .action-btn-apply:hover {
    background: color-mix(in srgb, var(--accent-green) 15%, transparent);
    border-color: var(--accent-green);
    color: var(--accent-green);
  }

  .action-btn-pop:hover {
    background: color-mix(in srgb, var(--accent-blue) 15%, transparent);
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }

  .action-btn-danger:hover {
    background: color-mix(in srgb, var(--accent-red) 20%, transparent);
    border-color: var(--accent-red);
    color: var(--accent-red);
  }
</style>
