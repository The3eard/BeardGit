<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { stashes, selectedStashIndex, doStashPush, doStashApply, doStashPop, doStashDrop, selectStash } from "../../stores/stashes";
  import ConfirmDialog from "../common/ConfirmDialog.svelte";
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
</script>

<div class="stash-list">
  <div class="stash-list-header">
    {#if showStashInput}
      <div class="stash-input-row">
        <input
          type="text"
          class="stash-input"
          placeholder={m.stash_message_placeholder()}
          bind:value={stashMessage}
          onkeydown={handleStashKeydown}
        />
        <button class="btn btn-small btn-confirm" onclick={handleStashPush}>✓</button>
        <button class="btn btn-small btn-cancel" onclick={handleCancelStash}>✕</button>
      </div>
    {:else}
      <button class="btn btn-stash" onclick={() => (showStashInput = true)}>
        {m.stash_button()}
      </button>
    {/if}
  </div>

  <div class="stash-items">
    {#if $stashes.length === 0}
      <div class="stash-empty">
        <p>{m.stash_empty()}</p>
      </div>
    {:else}
      {#each $stashes as entry (entry.index)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="stash-row"
          class:selected={$selectedStashIndex === entry.index}
          onclick={() => selectStash(entry.index)}
          onkeydown={(e) => { if (e.key === 'Enter') selectStash(entry.index); }}
          role="button"
          tabindex="0"
        >
          <div class="stash-top">
            <span class="stash-message">{entry.message || `stash@{${entry.index}}`}</span>
            <span class="stash-time">{formatRelativeTimeUnix(entry.timestamp)}</span>
          </div>
          <div class="stash-bottom-container">
            <div class="stash-bottom">
              <span class="stash-branch">{m.stash_on_branch({ branch: entry.branch })}</span>
              <span class="stash-oid">{shortOid(entry.oid)}</span>
            </div>
            <div class="stash-bottom-hover">
              <div class="stash-actions">
                <button
                  class="action-btn action-btn-apply"
                  title={m.stash_apply()}
                  onclick={(e: MouseEvent) => { e.stopPropagation(); doStashApply(entry.index); }}
                >{m.stash_apply()}</button>
                <button
                  class="action-btn action-btn-pop"
                  title={m.stash_pop()}
                  onclick={(e: MouseEvent) => { e.stopPropagation(); doStashPop(entry.index); }}
                >{m.stash_pop()}</button>
                <button
                  class="action-btn action-btn-danger"
                  title={m.stash_drop()}
                  onclick={(e: MouseEvent) => { e.stopPropagation(); confirmDrop = entry.index; }}
                >{m.stash_drop()}</button>
              </div>
              <span class="stash-oid">{shortOid(entry.oid)}</span>
            </div>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

{#if confirmDrop !== null}
  {@const dropEntry = $stashes.find((e) => e.index === confirmDrop)}
  <ConfirmDialog
    title={m.stash_confirm_drop_title()}
    detail={dropEntry ? `${dropEntry.message || `stash@{${dropEntry.index}}`}\n${shortOid(dropEntry.oid)}` : `stash@{${confirmDrop}}`}
    message={m.stash_confirm_drop_message()}
    confirmLabel={m.stash_drop()}
    destructive={true}
    onConfirm={() => {
      doStashDrop(confirmDrop!);
      confirmDrop = null;
    }}
    onCancel={() => (confirmDrop = null)}
  />
{/if}

<style>
  .stash-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .stash-list-header {
    padding: 8px;
    border-bottom: 1px solid var(--border);
  }

  .btn-stash {
    width: 100%;
    padding: 6px 12px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-stash:hover {
    background: rgba(255, 255, 255, 0.1);
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
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
  }

  .btn-small:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .stash-items {
    flex: 1;
    overflow-y: auto;
  }

  .stash-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .stash-row {
    display: flex;
    flex-direction: column;
    gap: 3px;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
  }

  .stash-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .stash-row.selected {
    background: rgba(88, 166, 255, 0.08);
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

  .stash-row:hover .stash-bottom {
    visibility: hidden;
  }

  .stash-row:hover .stash-bottom-hover {
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
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 4px;
    font-size: 10px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .action-btn-apply:hover {
    background: rgba(63, 185, 80, 0.15);
    border-color: var(--accent-green);
    color: var(--accent-green);
  }

  .action-btn-pop:hover {
    background: rgba(88, 166, 255, 0.15);
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }

  .action-btn-danger:hover {
    background: rgba(248, 81, 73, 0.2);
    border-color: #f85149;
    color: #f85149;
  }
</style>
