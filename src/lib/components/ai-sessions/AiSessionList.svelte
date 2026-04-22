<!--
  AiSessionList — two-section sidebar for the AI Sessions view.

  Section 1: "Active terminals" — every BeardGit-owned PTY currently
  running an AI provider (standalone tab, composite segment, background
  run). Rendered via `ActiveRow`.

  Section 2: "Conversations" — every on-disk AI transcript scoped to the
  current project. Rendered via `ConversationRow`.

  The two lists are structurally disjoint; a live bg-run may also have a
  transcript but each row is a different affordance (Focus vs Resume) so
  we intentionally let them coexist.

  Selection is mutually exclusive: clicking a conversation row clears the
  bg-run selection and vice versa. Focusing an Active row does NOT alter
  selection — it just switches tabs/segments. Detail-pane branching lives
  in `AiSessionDetail.svelte`.

  Refresh runs BOTH `refreshConversations` and `refreshAiBackgroundRuns`
  so the header refresh button covers both sections in a single click.
-->
<script lang="ts">
  import {
    conversations,
    conversationsLoading,
    refreshConversations,
  } from "$lib/stores/aiConversations";
  import { activeAiTerminals } from "$lib/stores/aiActiveTerminals";
  import {
    refreshAiBackgroundRuns,
    requestOpenCreateBackgroundRunDialog,
  } from "$lib/stores/aiBackground";
  import { repoInfo } from "$lib/stores/repo";
  import * as m from "$lib/paraglide/messages";
  import ActiveRow from "./ActiveRow.svelte";
  import ConversationRow from "./ConversationRow.svelte";

  /**
   * Refresh both lists. Called from the header button and by
   * `AiSessionsView`'s `refreshFn` on SplitView mount.
   */
  async function handleRefresh() {
    const path = $repoInfo?.path;
    await Promise.all([
      refreshConversations(path),
      refreshAiBackgroundRuns(),
    ]);
  }

  function openNewRunDialog() {
    requestOpenCreateBackgroundRunDialog();
  }
</script>

<div class="list-panel" data-testid="ai-session-list">
  <!-- Panel header: sits above both sections, hosts the global actions. -->
  <div class="panel-header">
    <span class="panel-title">{m.sidebar_ai_sessions()}</span>
    <div class="panel-actions">
      <button
        class="new-run-btn"
        onclick={openNewRunDialog}
        title={m.ai_background_tab_button_tooltip()}
        data-testid="ai-session-list-new-run"
      >
        + {m.ai_background_new_run_button()}
      </button>
      <button
        class="refresh-btn nf"
        onclick={handleRefresh}
        disabled={$conversationsLoading}
        title="Refresh"
        data-testid="ai-session-list-refresh"
      >
        {$conversationsLoading ? "" : ""}
      </button>
    </div>
  </div>

  <div class="sections">
    <!-- ─── Active terminals ─── -->
    <section
      class="section"
      data-testid="ai-session-list-section-active"
    >
      <header class="section-header">
        <span class="section-title">{m.ai_sessions_active_title()}</span>
        <span class="section-count" data-testid="ai-session-list-active-count">
          {$activeAiTerminals.length}
        </span>
      </header>
      {#if $activeAiTerminals.length === 0}
        <div class="section-empty" data-testid="ai-session-list-active-empty">
          {m.ai_sessions_empty_active()}
        </div>
      {:else}
        <ul class="row-list" role="list">
          {#each $activeAiTerminals as active (active.kind === "bg" ? `bg:${active.session.id}` : active.kind === "tab" ? `tab:${active.tabIndex}` : `seg:${active.tabIndex}:${active.segmentIndex}`)}
            <li class="row-item">
              <ActiveRow {active} />
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <!-- ─── Conversations ─── -->
    <section
      class="section"
      data-testid="ai-session-list-section-conversations"
    >
      <header class="section-header">
        <span class="section-title">{m.ai_sessions_conversations_title()}</span>
        <span class="section-count" data-testid="ai-session-list-conversations-count">
          {$conversations.length}
        </span>
      </header>
      {#if $conversations.length === 0 && !$conversationsLoading}
        <div
          class="section-empty"
          data-testid="ai-session-list-conversations-empty"
        >
          {m.ai_sessions_empty_conversations()}
        </div>
      {:else}
        <ul class="row-list" role="list">
          {#each $conversations as conversation (conversation.id)}
            <li class="row-item">
              <ConversationRow {conversation} />
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
</div>

<style>
  .list-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-primary);
    flex-shrink: 0;
    gap: 8px;
  }

  .panel-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .panel-actions {
    display: flex;
    align-items: center;
    gap: 6px;
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

  .refresh-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-family: var(--font-icons);
    font-size: 12px;
    padding: 2px 4px;
    border-radius: 4px;
    cursor: pointer;
  }

  .refresh-btn:hover:not(:disabled) {
    color: var(--text-primary);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .sections {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .section {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .section-title {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .section-count {
    font-size: 10px;
    background: var(--accent-blue);
    color: #ffffff;
    border-radius: 8px;
    padding: 0 6px;
    min-width: 18px;
    text-align: center;
    line-height: 16px;
    font-weight: 600;
  }

  .section-empty {
    padding: 16px 12px;
    font-size: 11px;
    color: var(--text-secondary);
    opacity: 0.6;
    text-align: center;
    font-style: italic;
  }

  .row-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .row-item {
    border-bottom: 1px solid color-mix(in srgb, var(--border) 40%, transparent);
  }

  .row-item:hover {
    background: var(--overlay-hover);
  }

  .row-item:last-child {
    border-bottom: none;
  }
</style>
