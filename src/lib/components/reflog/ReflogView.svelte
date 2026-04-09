<!--
  ReflogView — Resizable split view for reflog list + detail.
-->
<script lang="ts">
  import SplitView from "../common/SplitView.svelte";
  import ReflogList from "./ReflogList.svelte";
  import ReflogDetail from "./ReflogDetail.svelte";
  import { reflogEntries, selectedReflogEntry, loadReflog } from "../../stores/reflog";
  import * as m from "$lib/paraglide/messages";
  import type { ReflogEntry } from "../../types";

  let {
    onContextMenu,
    onNavigateToGraph,
    onNavigate,
  }: {
    onContextMenu: (e: MouseEvent, entry: ReflogEntry) => void;
    onNavigateToGraph: (oid: string) => void;
    onNavigate: (view: string) => void;
  } = $props();
</script>

<SplitView refreshFn={loadReflog}>
  {#snippet left()}
    <ReflogList entries={$reflogEntries} {onContextMenu} />
  {/snippet}
  {#snippet right()}
    {#if $selectedReflogEntry}
      <ReflogDetail
        entry={$selectedReflogEntry}
        {onNavigateToGraph}
        {onNavigate}
      />
    {:else}
      <div class="no-diff">
        <p>{m.reflog_select_entry()}</p>
      </div>
    {/if}
  {/snippet}
</SplitView>

<style>
  .no-diff {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    font-size: 13px;
  }
</style>
