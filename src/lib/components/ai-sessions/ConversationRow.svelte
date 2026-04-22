<!--
  ConversationRow — single row in the "Conversations" section of the AI
  Sessions view.

  Renders an `AiConversation` (an on-disk transcript, not a live process)
  with a consistent visual footprint alongside `ActiveRow` so both
  sections read as one list. The row is clickable (selects the
  conversation for the detail pane) and exposes a Resume button that
  spawns a fresh PTY via `resumeConversation`.

  Resume always forks the conversation — Claude's `--resume` writes a new
  UUID transcript that shares history with the parent, so the tooltip
  copy calls this out per the Phase-5 spec.
-->
<script lang="ts">
  import type { AiConversation } from "$lib/types";
  import { resumeConversation } from "$lib/stores/aiConversationActions";
  import { selectedConversationId } from "$lib/stores/aiConversations";
  import { selectedBackgroundSessionId } from "$lib/stores/aiBackground";
  import { providerName } from "$lib/data/ai-providers";
  import { formatRelativeTimeMs } from "$lib/utils/time";
  import { addToast } from "$lib/stores/toast";
  import ProviderIcon from "./ProviderIcon.svelte";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    conversation: AiConversation;
  }

  let { conversation }: Props = $props();

  /** Extract the last path segment for compact display. */
  function shortCwd(fullPath: string): string {
    const parts = fullPath.replace(/\\/g, "/").split("/").filter(Boolean);
    return parts[parts.length - 1] ?? fullPath;
  }

  let title = $derived(
    conversation.title && conversation.title.trim().length > 0
      ? conversation.title
      : m.ai_sessions_no_title(),
  );

  let cwdLabel = $derived(shortCwd(conversation.cwd));
  let lastActivity = $derived(formatRelativeTimeMs(conversation.last_activity_at));

  /**
   * Row click → select this conversation and clear any active bg-run
   * selection so the detail pane shows one source of truth.
   */
  function onRowClick() {
    selectedConversationId.set(conversation.id);
    selectedBackgroundSessionId.set(null);
  }

  /** Stop propagation so clicking Resume doesn't also select the row. */
  async function onResumeClick(e: MouseEvent) {
    e.stopPropagation();
    try {
      await resumeConversation(conversation);
    } catch (err) {
      addToast({
        message: m.ai_sessions_resume_error({ error: String(err) }),
        type: "error",
      });
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="session-item"
  data-testid="ai-conversation-row"
  data-conversation-id={conversation.id}
  onclick={onRowClick}
>
  <ProviderIcon provider={conversation.provider} size={20} />
  <div class="session-info">
    <div class="session-row-top">
      <span class="session-provider">{providerName(conversation.provider)}</span>
      <span class="session-title" title={title}>{title}</span>
      {#if conversation.parent_id}
        <span class="forked-badge" data-testid="ai-conversation-row-forked">
          {m.ai_sessions_forked_from({ prefix: conversation.parent_id })}
        </span>
      {/if}
    </div>
    <div class="session-row-bottom">
      <span class="session-cwd">{cwdLabel}</span>
      <span class="session-time">{lastActivity}</span>
    </div>
  </div>
  <div class="session-actions">
    <button
      class="session-action-btn resume-btn"
      onclick={onResumeClick}
      title={m.ai_sessions_resume_warning({
        provider: providerName(conversation.provider),
      })}
      data-testid="ai-conversation-row-resume"
    >
      <span class="action-label">{m.ai_sessions_resume_in_new_terminal()}</span>
    </button>
  </div>
</div>

<style>
  .session-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px;
    transition: background 0.1s;
    min-height: 48px;
    box-sizing: border-box;
    cursor: pointer;
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

  .session-title {
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    flex: 1;
  }

  .forked-badge {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    padding: 1px 5px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--accent-purple, #a371f7) 18%, transparent);
    color: var(--accent-purple, #a371f7);
    white-space: nowrap;
    flex-shrink: 0;
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
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 10px;
    padding: 2px 8px;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .session-action-btn:hover {
    color: var(--text-primary);
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
