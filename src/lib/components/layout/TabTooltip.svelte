<script lang="ts">
  import type { ProjectSnapshot } from "$lib/types";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    snapshot: ProjectSnapshot;
    x: number;
    y: number;
  }

  let { snapshot, x, y }: Props = $props();

  let isClean = $derived(
    snapshot.ahead === 0 &&
    snapshot.behind === 0 &&
    snapshot.staged === 0 &&
    snapshot.unstaged === 0 &&
    snapshot.untracked === 0 &&
    snapshot.stash_count === 0
  );
</script>

<div class="tab-tooltip" style="left: {x}px; top: {y}px;">
  <div class="branch-name">
    <span class="branch-icon">{"\uE725"}</span>
    {snapshot.head_branch ?? "detached"}
  </div>

  {#if isClean}
    <div class="clean-status">{m.tab_tooltip_clean()}</div>
  {:else}
    <div class="summary-rows">
      <div class="row">
        <span class="label">{m.tab_tooltip_ahead()}</span>
        <span class="badge" class:green={snapshot.ahead > 0} class:dimmed={snapshot.ahead === 0}>{snapshot.ahead}</span>
      </div>
      <div class="row">
        <span class="label">{m.tab_tooltip_behind()}</span>
        <span class="badge" class:dimmed={snapshot.behind === 0}>{snapshot.behind}</span>
      </div>
      <div class="separator"></div>
      <div class="row">
        <span class="label">{m.tab_tooltip_modified()}</span>
        <span class="badge" class:orange={snapshot.unstaged > 0} class:dimmed={snapshot.unstaged === 0}>{snapshot.unstaged}</span>
      </div>
      <div class="row">
        <span class="label">{m.tab_tooltip_staged()}</span>
        <span class="badge" class:green={snapshot.staged > 0} class:dimmed={snapshot.staged === 0}>{snapshot.staged}</span>
      </div>
      <div class="row">
        <span class="label">{m.tab_tooltip_untracked()}</span>
        <span class="badge" class:dimmed={true}>{snapshot.untracked}</span>
      </div>
      <div class="row">
        <span class="label">{m.tab_tooltip_stashes()}</span>
        <span class="badge" class:purple={snapshot.stash_count > 0} class:dimmed={snapshot.stash_count === 0}>{snapshot.stash_count}</span>
      </div>
    </div>
  {/if}
</div>

<style>
  .tab-tooltip {
    position: fixed;
    background: var(--bg-toolbar);
    border: 1px solid color-mix(in srgb, var(--text-primary) 15%, transparent);
    border-radius: 8px;
    padding: 10px 14px;
    min-width: 210px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4); /* beardgit:allow-hex: shadow neutral always-dark */
    z-index: 9999;
    pointer-events: none;
  }

  .branch-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 8px;
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .branch-icon {
    font-family: var(--font-icons);
    color: var(--accent-primary);
    font-size: 12px;
    flex-shrink: 0;
  }

  .clean-status {
    font-size: 11px;
    color: var(--accent-green);
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .summary-rows {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 11px;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .label {
    color: var(--text-secondary);
  }

  .separator {
    height: 1px;
    background: color-mix(in srgb, var(--text-primary) 6%, transparent);
    margin: 2px 0;
  }

  .badge {
    padding: 1px 8px;
    border-radius: 10px;
    font-weight: 500;
    font-size: 11px;
    background: var(--overlay-accent-muted);
    color: var(--text-secondary);
  }

  .badge.green {
    background: color-mix(in srgb, var(--accent-green) 15%, transparent);
    color: var(--accent-green);
  }

  .badge.orange {
    background: color-mix(in srgb, var(--accent-orange) 15%, transparent);
    color: var(--accent-orange);
  }

  .badge.purple {
    background: color-mix(in srgb, var(--accent-purple) 15%, transparent);
    color: var(--accent-purple);
  }

  .badge.dimmed {
    background: var(--overlay-accent-muted);
    color: var(--text-secondary);
    opacity: 0.5;
  }
</style>
