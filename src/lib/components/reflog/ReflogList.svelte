<script lang="ts">
  import type { ReflogEntry } from "../../types";
  import { selectedReflogIndex, selectReflogEntry, loadReflog, reflogLoading } from "../../stores/reflog";
  import { formatRelativeTimeUnix } from "../../utils/time";
  import { shortOid } from "../../utils/git";
  import List from "../common/List.svelte";
  import * as m from "$lib/paraglide/messages";

  let {
    entries,
    onContextMenu,
  }: {
    entries: ReflogEntry[];
    onContextMenu?: (e: MouseEvent, entry: ReflogEntry, index: number) => void;
  } = $props();

  /** Map reflog action to a display icon and color. */
  function actionStyle(action: string): { icon: string; color: string } {
    switch (action) {
      case "commit":
        return { icon: "\uF444", color: "var(--accent-green)" };
      case "checkout":
        return { icon: "\uE725", color: "var(--accent-primary)" };
      case "rebase":
        return { icon: "\uF021", color: "var(--accent-purple)" };
      case "reset":
        return { icon: "\uF0E2", color: "var(--accent-orange)" };
      case "merge":
        return { icon: "\uE727", color: "var(--accent-primary)" };
      case "pull":
        return { icon: "\uF063", color: "var(--accent-primary)" };
      case "cherry-pick":
        return { icon: "\uF41E", color: "var(--accent-purple)" };
      default:
        return { icon: "\uF444", color: "var(--text-secondary)" };
    }
  }

  /** Derive a stable key from entry (oid + timestamp + index in entries array). */
  function getKey(entry: ReflogEntry): string {
    const idx = entries.indexOf(entry);
    return `${entry.oid}-${entry.timestamp}-${idx}`;
  }

  let selectedKey = $derived(
    $selectedReflogIndex !== null && entries[$selectedReflogIndex]
      ? getKey(entries[$selectedReflogIndex])
      : null,
  );

  function handleSelect(entry: ReflogEntry) {
    const idx = entries.indexOf(entry);
    if (idx >= 0) selectReflogEntry(idx);
  }

  function handleContextMenu(e: MouseEvent, entry: ReflogEntry) {
    const idx = entries.indexOf(entry);
    if (idx >= 0) onContextMenu?.(e, entry, idx);
  }

  function handleRefresh() {
    loadReflog();
  }
</script>

<List
  items={entries}
  loading={$reflogLoading}
  title={m.reflog_title()}
  {selectedKey}
  {getKey}
  emptyMessage={m.reflog_empty()}
  onSelect={handleSelect}
  onRefresh={handleRefresh}
  onContextMenu={handleContextMenu}
>
  {#snippet row({ item }: { item: ReflogEntry; selected: boolean })}
    {@const style = actionStyle(item.action)}
    <div class="row-status">
      <span class="state-icon nf" style="color: {style.color}">{style.icon}</span>
    </div>
    <div class="row-center">
      <div class="row-title">
        <span class="entry-action">{item.action}</span>
        <span class="entry-summary">{item.summary}</span>
      </div>
      <div class="row-meta">
        <span class="oid">{shortOid(item.prev_oid)}</span>
        <span class="oid-arrow">{"\u2192"}</span>
        <span class="oid">{shortOid(item.oid)}</span>
      </div>
    </div>
    <div class="row-time">
      {formatRelativeTimeUnix(item.timestamp)}
    </div>
  {/snippet}
</List>

<style>
  .row-status {
    display: flex;
    align-items: flex-start;
    min-width: 28px;
    flex-shrink: 0;
    padding-top: 1px;
  }

  .state-icon {
    font-size: 14px;
    font-family: var(--font-icons);
  }

  .row-center {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .row-title {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
  }

  .entry-action {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .entry-summary {
    font-size: 12px;
    font-weight: 500;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-meta {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 10px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    overflow: hidden;
  }

  .oid-arrow {
    opacity: 0.6;
  }

  .row-time {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
    margin-top: 2px;
  }
</style>
