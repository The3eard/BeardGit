<script lang="ts">
  import type { ReflogEntry } from "../../types";
  import { selectedReflogOid, selectReflogEntry } from "../../stores/reflog";
  import { formatRelativeTimeUnix } from "../../utils/time";
  import * as m from "$lib/paraglide/messages";

  let {
    entries,
    onContextMenu,
  }: {
    entries: ReflogEntry[];
    onContextMenu?: (e: MouseEvent, entry: ReflogEntry) => void;
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

  function shortOid(oid: string): string {
    return oid.substring(0, 7);
  }
</script>

<div class="reflog-list">
  <div class="list-header">
    <h3>{m.reflog_title()}</h3>
  </div>
  <div class="list-content">
    {#each entries as entry (entry.oid + entry.timestamp)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="reflog-row"
        class:selected={$selectedReflogOid === entry.oid}
        onclick={() => selectReflogEntry(entry.oid)}
        oncontextmenu={(e) => { e.preventDefault(); onContextMenu?.(e, entry); }}
      >
        <span
          class="action-icon nf"
          style="color: {actionStyle(entry.action).color}"
        >{actionStyle(entry.action).icon}</span>
        <div class="entry-info">
          <div class="entry-line-1">
            <span class="entry-action">{entry.action}</span>
            <span class="entry-summary">{entry.summary}</span>
          </div>
          <div class="entry-line-2">
            <span class="entry-oids">
              <span class="oid">{shortOid(entry.prev_oid)}</span>
              <span class="oid-arrow">{"\u2192"}</span>
              <span class="oid">{shortOid(entry.oid)}</span>
            </span>
            <span class="entry-time">{formatRelativeTimeUnix(entry.timestamp)}</span>
          </div>
        </div>
      </div>
    {:else}
      <div class="empty">{m.reflog_empty()}</div>
    {/each}
  </div>
</div>

<style>
  .reflog-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .list-header {
    padding: 12px 16px 8px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .list-header h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .list-content {
    flex: 1;
    overflow-y: auto;
  }

  .reflog-row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 6px 16px;
    cursor: pointer;
    transition: background 0.1s;
  }

  .reflog-row:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .reflog-row.selected {
    background: rgba(88, 166, 255, 0.1);
  }

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

  .empty {
    padding: 24px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
  }
</style>
