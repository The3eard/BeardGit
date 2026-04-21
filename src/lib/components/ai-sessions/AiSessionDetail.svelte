<script lang="ts">
  /**
   * Detail pane for an AI Sessions row.
   *
   * For background runs: shows the status badge, elapsed time, token-usage
   * readout, transcript panel, and the four action buttons from the Phase
   * 10 spec (Switch to worktree, Open terminal here, View changes, Discard
   * worktree). For ordinary provider sessions, shows a lightweight info
   * card.
   */
  import {
    selectedBackgroundSession,
    aiBackgroundTranscripts,
    cancelAiBackgroundRun,
    discardAiBackgroundRunWorktree,
    openTerminalForAiBackgroundSession,
  } from "$lib/stores/aiBackground";
  import { openProjectTab } from "$lib/stores/projects";
  import * as m from "$lib/paraglide/messages";
  import BackgroundRunStatusBadge from "../ai/BackgroundRunStatusBadge.svelte";
  import BackgroundRunTranscript from "../ai/BackgroundRunTranscript.svelte";
  import ProviderIcon from "./ProviderIcon.svelte";
  import type { AiSession } from "$lib/types";

  let session = $derived($selectedBackgroundSession);
  let transcript = $derived.by(() => {
    if (!session) return [] as string[];
    return $aiBackgroundTranscripts.get(session.id) ?? [];
  });

  let status = $derived(session?.background_status ?? null);

  let tokenLine = $derived.by(() => {
    if (!status || status.state !== "completed") return null;
    const tu = status.token_usage;
    if (!tu) return null;
    return m.ai_background_token_usage({
      input: tu.input,
      output: tu.output,
    });
  });

  let costLine = $derived.by(() => {
    if (!status || status.state !== "completed") return null;
    const cost = status.token_usage?.total_cost_usd;
    if (cost == null) return null;
    return m.ai_background_token_cost({ cost: cost.toFixed(4) });
  });

  let isRunning = $derived(status?.state === "running" || status?.state === "queued");
  let isTerminal = $derived(
    status?.state === "completed" ||
      status?.state === "failed" ||
      status?.state === "cancelled",
  );

  async function handleOpenTerminal() {
    if (!session || isRunning) return;
    try {
      await openTerminalForAiBackgroundSession(session.id);
    } catch (e) {
      console.error("failed to open terminal for AI session", e);
    }
  }

  async function handleDiscard() {
    if (!session || !isTerminal) return;
    // eslint-disable-next-line no-alert
    if (!window.confirm(m.ai_background_discard_confirm())) return;
    try {
      await discardAiBackgroundRunWorktree(session.id);
    } catch (e) {
      console.error("failed to discard worktree", e);
    }
  }

  async function handleSwitchToWorktree() {
    if (!session?.worktree_path) return;
    try {
      await openProjectTab(session.worktree_path);
    } catch (e) {
      console.error("failed to open worktree tab", e);
    }
  }

  async function handleCancel() {
    if (!session || !isRunning) return;
    try {
      await cancelAiBackgroundRun(session.id);
    } catch (e) {
      console.error("failed to cancel run", e);
    }
  }
</script>

{#if !session}
  <div class="empty">{m.ai_sessions_empty()}</div>
{:else if session.background_status}
  <div class="detail">
    <header class="header">
      <div class="title-row">
        <ProviderIcon provider={session.provider} size={20} />
        <span class="provider">{session.provider.replace("_", " ")}</span>
        <BackgroundRunStatusBadge status={session.background_status} />
      </div>
      <div class="wt-row">
        <code class="wt-path">{session.worktree_path ?? session.cwd}</code>
      </div>
    </header>

    {#if tokenLine}
      <div class="meta">
        <span>{tokenLine}</span>
        {#if costLine}<span class="dot">•</span><span>{costLine}</span>{/if}
      </div>
    {/if}

    <div class="actions">
      {#if isRunning}
        <button class="btn danger" onclick={handleCancel}>
          {m.ai_background_cancel_run()}
        </button>
        <button
          class="btn"
          disabled
          title={m.ai_background_tooltip_terminal_running()}
        >
          {m.ai_background_open_terminal()}
        </button>
      {:else}
        <button class="btn" onclick={handleOpenTerminal}>
          {m.ai_background_open_terminal()}
        </button>
      {/if}
      <button class="btn" onclick={handleSwitchToWorktree} disabled={!session.worktree_path}>
        {m.ai_background_switch_to_worktree()}
      </button>
      {#if isTerminal}
        <button class="btn danger" onclick={handleDiscard}>
          {m.ai_background_discard_worktree()}
        </button>
      {/if}
    </div>

    <BackgroundRunTranscript lines={transcript} />
  </div>
{:else}
  <!-- Provider-reported session (not a background run). -->
  <div class="detail">
    <header class="header">
      <div class="title-row">
        <ProviderIcon provider={session.provider} size={20} />
        <span class="provider">{session.provider.replace("_", " ")}</span>
      </div>
      <div class="wt-row">
        <code class="wt-path">{session.cwd}</code>
      </div>
    </header>
    <div class="info">
      {m.ai_sessions_external_terminal()}
    </div>
  </div>
{/if}

<style>
  .detail {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    flex: 1;
    min-height: 0;
  }

  .header {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .provider {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    text-transform: capitalize;
  }

  .wt-row {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .wt-path {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent-blue);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .dot {
    opacity: 0.4;
  }

  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .btn {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 4px;
    padding: 4px 10px;
    font-size: 11px;
    cursor: pointer;
  }

  .btn:hover:not(:disabled) {
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn.danger {
    color: #f85149;
    border-color: color-mix(in srgb, #f85149 40%, transparent);
  }

  .btn.danger:hover {
    background: color-mix(in srgb, #f85149 10%, transparent);
  }

  .empty,
  .info {
    padding: 24px;
    color: var(--text-secondary);
    text-align: center;
    font-size: 12px;
  }
</style>
