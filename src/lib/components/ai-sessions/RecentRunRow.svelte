<!--
  RecentRunRow — single row in the "Recent runs" section of the AI
  Sessions view.

  Renders a terminal-state background AI run (`completed`, `failed`,
  `cancelled`). The "Active terminals" section drops these the
  moment they exit, so this row gives the user a way back to the
  captured transcript and metadata of a finished run. Clicking
  routes selection through `selectedBackgroundSessionId` (same as
  bg-kind ActiveRows), so the detail pane's bg-run branch renders
  with the run's status badge, transcript, switch-to-worktree, and
  discard-worktree affordances.

  Layout matches `SessionRow`: provider icon, title (worktree slug),
  trailing relative timestamp. The terminal-state status is conveyed
  by a tiny inline badge next to the slug so the user can scan
  completion outcomes at a glance without opening each row.
-->
<script lang="ts">
  import type { AiSession } from "$lib/types";
  import {
    selectAiSessionRow,
  } from "$lib/stores/aiActiveTerminals";
  import { selectedBackgroundSessionId } from "$lib/stores/aiBackground";
  import { formatRelativeTimeUnix } from "$lib/utils/time";
  import * as m from "$lib/paraglide/messages";
  import ProviderIcon from "./ProviderIcon.svelte";
  import SessionRow from "./SessionRow.svelte";

  interface Props {
    session: AiSession;
  }

  let { session }: Props = $props();

  /**
   * Last path segment of the worktree path — the AI background
   * coordinator names worktrees after a slug derived from the
   * prompt, so this is the most distinguishing token for the row.
   * Falls back to the session id when no worktree path is recorded
   * (external sessions surfaced from disk).
   */
  let title = $derived.by(() => {
    const wt = session.worktree_path;
    if (!wt) return session.id;
    const parts = wt.replace(/\\/g, "/").split("/").filter(Boolean);
    return parts[parts.length - 1] ?? session.id;
  });

  let date = $derived(
    session.started_at != null
      ? formatRelativeTimeUnix(session.started_at)
      : null,
  );

  /**
   * Compact terminal-state label rendered inline next to the title.
   * Mirrors `BackgroundRunStatusBadge`'s wording but stays tiny so
   * the row keeps its single-line shape.
   */
  let statusLabel = $derived.by(() => {
    const state = session.background_status?.state;
    if (state === "completed") return m.ai_background_status_completed();
    if (state === "failed") return m.ai_background_status_failed();
    if (state === "cancelled") return m.ai_background_status_cancelled();
    return null;
  });

  let statusKind = $derived(session.background_status?.state ?? null);

  let selected = $derived($selectedBackgroundSessionId === session.id);

  function onSelect() {
    selectAiSessionRow({ kind: "background", id: session.id });
  }
</script>

<div
  data-testid="ai-recent-run-row"
  data-session-id={session.id}
  data-status={statusKind}
  onclick={onSelect}
  role="presentation"
>
  <SessionRow {title} {date} {selected} {onSelect}>
    {#snippet icon()}
      <ProviderIcon provider={session.provider} size={20} />
    {/snippet}
  </SessionRow>
  {#if statusLabel}
    <span
      class="status-tag"
      class:completed={statusKind === "completed"}
      class:failed={statusKind === "failed"}
      class:cancelled={statusKind === "cancelled"}
      data-testid="ai-recent-run-status"
    >
      {statusLabel}
    </span>
  {/if}
</div>

<style>
  div[data-testid="ai-recent-run-row"] {
    position: relative;
  }

  /* The status tag floats over the trailing date column so the row
     keeps its `[icon] [title] [date]` skeleton from `SessionRow`
     intact while still showing outcome at-a-glance. */
  .status-tag {
    position: absolute;
    top: 50%;
    right: 64px;
    transform: translateY(-50%);
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    padding: 1px 6px;
    border-radius: 8px;
    white-space: nowrap;
    pointer-events: none;
  }

  .status-tag.completed {
    background: color-mix(in srgb, var(--accent-green) 18%, transparent);
    color: var(--accent-green);
  }

  .status-tag.failed {
    background: color-mix(in srgb, var(--accent-red) 18%, transparent);
    color: var(--accent-red);
  }

  .status-tag.cancelled {
    background: color-mix(in srgb, var(--text-secondary) 18%, transparent);
    color: var(--text-secondary);
  }
</style>
