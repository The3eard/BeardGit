<!--
  ReflogView — Resizable split view for reflog list + detail.
-->
<script lang="ts">
  import SplitView from "../common/SplitView.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import ReflogList from "./ReflogList.svelte";
  import ReflogDetail from "./ReflogDetail.svelte";
  import { reflogEntries, selectedReflogEntry, loadReflog } from "../../stores/reflog";
  import * as m from "$lib/paraglide/messages";
  import type { ReflogEntry } from "../../types";

  let {
    onContextMenu,
    onNavigateToGraph,
    onNavigate,
    onFileClick,
  }: {
    onContextMenu: (e: MouseEvent, entry: ReflogEntry, index: number) => void;
    onNavigateToGraph: (oid: string) => void;
    onNavigate: (view: string) => void;
    onFileClick?: (path: string) => void;
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
        {onFileClick}
      />
    {:else}
      <EmptyState fill icon={"\uF1DA"} title={m.reflog_select_entry()} />
    {/if}
  {/snippet}
</SplitView>
