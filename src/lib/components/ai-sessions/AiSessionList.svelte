<!--
  AiSessionList — list of AI coding assistant sessions for the current
  project (interactive + headless). Swaps the outer shell for <List>
  while keeping session-specific row markup, styles, and lifecycle.
-->
<script lang="ts">
  import {
    mergedSessions,
    sessionsLoading,
    refreshSessions,
    dismissSession,
  } from "../../stores/aiSessions";
  import { activeBackgroundRunCount, selectedBackgroundSessionId, requestOpenCreateBackgroundRunDialog } from "../../stores/aiBackground";
  import {
    getSessionTier,
    focusSessionTab,
    resumeSession,
  } from "../../stores/aiSessionActions";
  import { repoInfo } from "../../stores/repo";
  import { formatRelativeTimeUnix } from "../../utils/time";
  import type { AiSession } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import List from "../common/List.svelte";
  import BackgroundRunStatusBadge from "../ai/BackgroundRunStatusBadge.svelte";
  import ProviderIcon from "./ProviderIcon.svelte";
  import { providerName } from "$lib/data/ai-providers";

  interface Props {
    onSelectSession?: (session: AiSession) => void;
  }

  let { onSelectSession }: Props = $props();

  /** Extract last path segment for compact display. */
  function shortCwd(fullPath: string): string {
    const parts = fullPath.replace(/\\/g, "/").split("/").filter(Boolean);
    return parts[parts.length - 1] ?? fullPath;
  }

  // No `onMount` on purpose — refresh + listener startup live on the
  // `SplitView` refreshFn + app-shell init (see `AiSessionsView.svelte`
  // and `+layout.svelte`). Keeping this component a dumb render of
  // `$mergedSessions` matches `TagList` / `BranchList` / etc. so the
  // view swap paints in the same frame as the rest of the sections.

  function handleRefresh() {
    const path = $repoInfo?.path;
    refreshSessions(path);
  }

  function getKey(session: AiSession): string {
    return session.id;
  }

  function openNewRunDialog() {
    requestOpenCreateBackgroundRunDialog();
  }

  function handleSelect(session: AiSession) {
    // Every row populates the detail pane — interactive/headless sessions
    // without a background_status still need to drive the selection store
    // so the right-hand pane reflects what the user clicked. The detail
    // pane reads the id through `selectedSession` (merged list) rather
    // than `selectedBackgroundSession` (bg-runs only) — see aiSessions.ts.
    selectedBackgroundSessionId.set(session.id);
    onSelectSession?.(session);
  }

  async function handleResumeSession(session: AiSession) {
    try {
      const attached = await resumeSession(session);
      if (!attached) {
        console.warn("Resume not supported for provider:", session.provider);
      }
    } catch (err) {
      console.error("Failed to resume session:", err);
    }
  }
</script>

<List
  items={$mergedSessions}
  loading={$sessionsLoading}
  title={m.sidebar_ai_sessions()}
  selectedKey={$selectedBackgroundSessionId}
  {getKey}
  onSelect={handleSelect}
  onRefresh={handleRefresh}
>
  {#snippet headerActions()}
    {#if $activeBackgroundRunCount > 0}
      <span class="count-badge">{$activeBackgroundRunCount}</span>
    {:else if $mergedSessions.length > 0}
      <span class="count-badge">{$mergedSessions.length}</span>
    {/if}
    <button
      class="new-run-btn"
      onclick={openNewRunDialog}
      title={m.ai_background_tab_button_tooltip()}
    >
      + {m.ai_background_new_run_button()}
    </button>
    <button
      class="refresh-btn nf"
      onclick={handleRefresh}
      disabled={$sessionsLoading}
      title="Refresh"
    >
      {$sessionsLoading ? "\uF110" : "\uF021"}
    </button>
  {/snippet}

  {#snippet emptyState()}
    <div class="empty-state">
      <span class="empty-icon nf">{"\uF489"}</span>
      <span class="empty-text">{m.ai_sessions_empty()}</span>
      <span class="empty-hint">{m.ai_sessions_empty_hint()}</span>
    </div>
  {/snippet}

  {#snippet row({ item }: { item: AiSession; selected: boolean })}
    <div
      class="session-item"
      class:active={item.is_active}
      class:ended={!item.is_active}
      data-testid="ai-session-row"
      data-session-id={item.id}
    >
      <ProviderIcon provider={item.provider} size={20} />
      <div class="session-info">
        <div class="session-row-top">
          <span class="session-provider">{providerName(item.provider)}</span>
          {#if item.background_status}
            <BackgroundRunStatusBadge status={item.background_status} compact />
          {:else if item.is_active}
            <span class="session-badge active">ACTIVE</span>
          {:else}
            <span class="session-badge ended">ENDED</span>
          {/if}
          <span class="session-badge kind" class:headless={item.kind === "headless"}>
            {item.kind}
          </span>
        </div>
        <div class="session-row-bottom">
          <span class="session-cwd">{shortCwd(item.cwd)}</span>
          {#if item.started_at}
            <span class="session-time">{formatRelativeTimeUnix(item.started_at)}</span>
          {/if}
        </div>
      </div>
      <div class="session-meta">
        {#if !item.worktree_path}
          <span class="external-badge" data-testid="external-badge">
            {m.ai_sessions_external()}
          </span>
        {/if}
      </div>
      <div class="session-actions">
        {#if item.is_active && item.kind === "interactive"}
          {@const tier = getSessionTier(item)}
          {#if tier === "focus"}
            <button
              class="session-action-btn focus-btn"
              onclick={() => focusSessionTab(item)}
              title={m.ai_sessions_focus()}
            >
              <span class="action-label">{m.ai_sessions_focus()}</span>
            </button>
          {:else}
            <button
              class="session-action-btn resume-btn"
              onclick={() => handleResumeSession(item)}
              title={m.ai_sessions_open_terminal()}
            >
              <span class="action-label">{m.ai_sessions_open_terminal()}</span>
            </button>
          {/if}
        {:else if item.is_active && item.kind === "headless"}
          <button class="session-action-btn" title="Output">
            <span class="nf">{"\uF15C"}</span>
          </button>
        {:else}
          <button
            class="session-action-btn dismiss"
            onclick={() => dismissSession(item.id)}
            title="Dismiss"
          >
            <span class="nf">{"\uF00D"}</span>
          </button>
        {/if}
      </div>
    </div>
  {/snippet}
</List>

<style>
  .count-badge {
    font-size: 10px;
    background: var(--accent-blue);
    color: #ffffff;
    border-radius: 8px;
    padding: 0 5px;
    min-width: 16px;
    text-align: center;
    line-height: 16px;
  }

  .new-run-btn {
    background: transparent;
    border: 1px solid var(--accent-blue);
    color: var(--accent-blue);
    border-radius: 5px;
    padding: 2px 8px;
    font-size: 10px;
    font-weight: 600;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .new-run-btn:hover {
    background: color-mix(in srgb, var(--accent-blue) 14%, transparent);
  }

  /* ─── Empty state ─── */

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    gap: 8px;
  }

  .empty-icon {
    font-size: 32px;
    color: var(--text-secondary);
    opacity: 0.4;
    font-family: var(--font-icons);
  }

  .empty-text {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .empty-hint {
    font-size: 11px;
    color: var(--text-secondary);
    opacity: 0.6;
    text-align: center;
    max-width: 280px;
  }

  /* ─── Session items ─── */

  .session-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px;
    transition: background 0.1s;
  }

  .session-item.ended {
    opacity: 0.5;
  }

  .session-info {
    flex: 1;
    min-width: 0;
  }

  .session-row-top {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .session-provider {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .session-badge {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    padding: 1px 5px;
    border-radius: 8px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .session-badge.active {
    background: rgba(63, 185, 80, 0.15);
    color: var(--accent-green);
  }

  .session-badge.ended {
    background: rgba(128, 128, 128, 0.15);
    color: var(--text-secondary);
  }

  .session-badge.kind {
    background: rgba(88, 166, 255, 0.15);
    color: var(--accent-blue);
  }

  .session-badge.kind.headless {
    background: rgba(210, 153, 34, 0.15);
    color: var(--accent-orange);
  }

  .session-row-bottom {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 2px;
  }

  .session-cwd {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--accent-blue);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .session-time {
    font-size: 10px;
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .session-meta {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .external-badge {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    padding: 1px 5px;
    border-radius: 8px;
    background: rgba(128, 128, 128, 0.15);
    color: var(--text-secondary);
  }

  .session-actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .session-item:hover .session-actions {
    opacity: 1;
  }

  .session-action-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 14px;
    padding: 2px 4px;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .session-action-btn:hover {
    color: var(--text-primary);
  }

  .session-action-btn.dismiss:hover {
    color: var(--accent-red);
  }

  .session-action-btn.focus-btn,
  .session-action-btn.resume-btn {
    font-size: 10px;
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.06);
  }

  .session-action-btn.focus-btn {
    color: var(--accent-blue);
    border-color: rgba(88, 166, 255, 0.3);
  }

  .session-action-btn.focus-btn:hover {
    background: rgba(88, 166, 255, 0.1);
  }

  .session-action-btn.resume-btn {
    color: var(--accent-green);
    border-color: rgba(63, 185, 80, 0.3);
  }

  .session-action-btn.resume-btn:hover {
    background: rgba(63, 185, 80, 0.1);
  }

  .action-label {
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
</style>
