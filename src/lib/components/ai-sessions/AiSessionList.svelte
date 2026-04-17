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
  import { aiResumeSession } from "../../api/tauri";
  import { openTabs, activeTabIndex } from "../../stores/tabs";
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

  /** Compare two paths ignoring trailing slashes. */
  function normalizedMatch(a: string, b: string): boolean {
    return a.replace(/\/+$/, "") === b.replace(/\/+$/, "");
  }

  /** Determine the action tier for a session.
   * - "focus": session has a linked terminal tab in the app
   * - "resume": session is active but no linked terminal tab
   * - "ended": session has ended
   */
  function getSessionTier(session: AiSession): "focus" | "resume" | "ended" {
    if (!session.is_active) return "ended";

    // Check if any open tab has a terminal matching this provider + cwd
    const currentTabs = $openTabs;
    for (const tab of currentTabs) {
      if (tab.kind === "terminal" && tab.terminal.provider === session.provider) {
        if (normalizedMatch(tab.terminal.cwd, session.cwd)) return "focus";
      }
      if (tab.kind === "composite") {
        for (const seg of tab.segments) {
          if (seg.type === "terminal" && seg.info.provider === session.provider) {
            if (normalizedMatch(seg.info.cwd, session.cwd)) return "focus";
          }
        }
      }
    }

    return "resume";
  }

  function handleFocusSession(session: AiSession) {
    // Find the matching tab and switch to it
    const currentTabs = $openTabs;
    for (let i = 0; i < currentTabs.length; i++) {
      const tab = currentTabs[i];
      if (tab.kind === "terminal" && tab.terminal.provider === session.provider) {
        if (normalizedMatch(tab.terminal.cwd, session.cwd)) {
          activeTabIndex.set(i);
          return;
        }
      }
      if (tab.kind === "composite") {
        for (let s = 0; s < tab.segments.length; s++) {
          const seg = tab.segments[s];
          if (seg.type === "terminal" && seg.info.provider === session.provider) {
            if (normalizedMatch(seg.info.cwd, session.cwd)) {
              activeTabIndex.set(i);
              // Also switch to the terminal segment within the composite
              return;
            }
          }
        }
      }
    }
  }

  async function handleResumeSession(session: AiSession) {
    try {
      const sessionId = await aiResumeSession(session.provider, session.id);
      if (sessionId === null) {
        // Provider does not support resume
        console.warn("Resume not supported for provider:", session.provider);
      }
    } catch (err) {
      console.error("Failed to resume session:", err);
    }
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
          {@const tier = getSessionTier(item)}
          {#if tier === "focus"}
            <button
              class="session-action-btn focus-btn"
              onclick={() => handleFocusSession(item)}
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
