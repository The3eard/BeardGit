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
        return { icon: "\uE725", color: "var(--accent-blue)" };
      case "rebase":
        return { icon: "\uF021", color: "var(--accent-purple)" };
      case "reset":
        return { icon: "\uF0E2", color: "var(--accent-orange)" };
      case "merge":
        return { icon: "\uE727", color: "var(--accent-blue)" };
      case "pull":
        return { icon: "\uF063", color: "var(--accent-blue)" };
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
    <span
      class="action-icon nf"
      style="color: {style.color}"
    >{style.icon}</span>
    <div class="entry-info">
      <div class="entry-line-1">
        <span class="entry-action">{item.action}</span>
        <span class="entry-summary">{item.summary}</span>
      </div>
      <div class="entry-line-2">
        <span class="entry-oids">
          <span class="oid">{shortOid(item.prev_oid)}</span>
          <span class="oid-arrow">{"\u2192"}</span>
          <span class="oid">{shortOid(item.oid)}</span>
        </span>
        <span class="entry-time">{formatRelativeTimeUnix(item.timestamp)}</span>
      </div>
    </div>
  {/snippet}
</List>

<style>
  .action-icon {
    font-size: 14px;
    width: 18px;
    text-align: center;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .entry-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .entry-line-1 {
    display: flex;
    align-items: center;
    gap: 6px;
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
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .entry-line-2 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .entry-oids {
    display: flex;
    align-items: center;
    gap: 3px;
  }

  .oid {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
  }

  .oid-arrow {
    font-size: 10px;
    color: var(--text-secondary);
    opacity: 0.6;
  }

  .entry-time {
    font-size: 10px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }
</style>
