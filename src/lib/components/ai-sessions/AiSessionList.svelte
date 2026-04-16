<!--
  AiSessionList — list of AI coding assistant sessions for the current
  project (interactive + headless). Swaps the outer shell for <List>
  while keeping session-specific row markup, styles, and lifecycle.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    sessions,
    sessionsLoading,
    refreshSessions,
    dismissSession,
    startSessionListeners,
    stopSessionListeners,
  } from "../../stores/aiSessions";
  import { repoInfo } from "../../stores/repo";
  import { formatRelativeTimeUnix } from "../../utils/time";
  import type { AiSession, AiProviderKind } from "$lib/types";
  import * as m from "$lib/paraglide/messages";
  import List from "../common/List.svelte";

  /** Display name per AI provider. */
  const PROVIDER_NAME: Record<AiProviderKind, string> = {
    claude_code: "Claude Code",
    codex: "Codex",
    open_code: "OpenCode",
  };

  /** Provider-colored icon backgrounds. */
  const PROVIDER_COLORS: Record<string, string> = {
    claude_code: "#d97757",
    codex: "#ffffff",
    open_code: "#8b8b8b",
  };

  /** Provider icon initials for the rounded square. */
  const PROVIDER_INITIALS: Record<string, string> = {
    claude_code: "C",
    codex: "X",
    open_code: "O",
  };

  /** Extract last path segment for compact display. */
  function shortCwd(fullPath: string): string {
    const parts = fullPath.replace(/\\/g, "/").split("/").filter(Boolean);
    return parts[parts.length - 1] ?? fullPath;
  }

  onMount(() => {
    const path = $repoInfo?.path;
    if (path) {
      refreshSessions(path);
      startSessionListeners(path);
    }
  });

  onDestroy(() => {
    stopSessionListeners();
  });

  function handleRefresh() {
    const path = $repoInfo?.path;
    refreshSessions(path);
  }

  function getKey(session: AiSession): string {
    return session.id;
  }
</script>

<List
  items={$sessions}
  loading={$sessionsLoading}
  title={m.sidebar_ai_sessions()}
  selectedKey={null}
  {getKey}
  onRefresh={handleRefresh}
>
  {#snippet headerActions()}
    {#if $sessions.length > 0}
      <span class="count-badge">{$sessions.length}</span>
    {/if}
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
    {@const providerColor = PROVIDER_COLORS[item.provider] ?? "#888"}
    <div
      class="session-item"
      class:active={item.is_active}
      class:ended={!item.is_active}
      style="--provider-color: {providerColor}"
    >
      <div class="session-icon" style="background: {providerColor}">
        <span class="icon-initial">{PROVIDER_INITIALS[item.provider] ?? "?"}</span>
      </div>
      <div class="session-info">
        <div class="session-row-top">
          <span class="session-provider">{PROVIDER_NAME[item.provider] ?? item.provider}</span>
          {#if item.is_active}
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
      <div class="session-actions">
        {#if item.is_active && item.kind === "interactive"}
          <button
            class="session-action-btn external"
            disabled
            title={m.ai_sessions_external_terminal()}
          >
            <span class="external-label">{m.ai_sessions_external()}</span>
          </button>
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
    padding: 8px 0;
    transition: background 0.1s;
  }

  .session-item.active {
    background: color-mix(in srgb, var(--provider-color) 5%, transparent);
  }

  .session-item.ended {
    opacity: 0.5;
  }

  .session-icon {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .icon-initial {
    font-size: 14px;
    font-weight: 700;
    color: #000000;
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

  .session-action-btn.external {
    cursor: default;
    opacity: 0.5;
    font-size: 10px;
    padding: 2px 6px;
  }

  .external-label {
    font-size: 10px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--text-secondary);
  }
</style>
