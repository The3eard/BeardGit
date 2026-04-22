<!--
  ActiveRow — single row in the "Active terminals" section of the AI
  Sessions view.

  Renders one `ActiveTerminal` (tab, composite segment, or background run)
  with a consistent visual footprint so the section reads as one list
  rather than three. Background-run rows additionally carry the live
  status badge + relative start-time; tab/segment rows carry a short cwd.
  All three branches share the `.session-item` class tree copied from
  the legacy `AiSessionList.svelte` so the look matches across sections.

  The Focus button delegates to `focusTerminal(active)` which covers all
  three kinds (see `aiConversationActions.ts` for the branch table).
-->
<script lang="ts">
  import type { ActiveTerminal } from "$lib/stores/aiActiveTerminals";
  import { focusTerminal } from "$lib/stores/aiConversationActions";
  import { providerName } from "$lib/data/ai-providers";
  import { formatRelativeTimeUnix } from "$lib/utils/time";
  import ProviderIcon from "./ProviderIcon.svelte";
  import BackgroundRunStatusBadge from "../ai/BackgroundRunStatusBadge.svelte";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    active: ActiveTerminal;
  }

  let { active }: Props = $props();

  /** Extract the last path segment for compact display. */
  function shortCwd(fullPath: string): string {
    const parts = fullPath.replace(/\\/g, "/").split("/").filter(Boolean);
    return parts[parts.length - 1] ?? fullPath;
  }

  /** Stop propagation so clicking the Focus button doesn't also trigger the row click. */
  function onFocusClick(e: MouseEvent) {
    e.stopPropagation();
    focusTerminal(active);
  }

  // Derive display fields per kind. Kept as `$derived` so Svelte 5's
  // reactivity tracks the discriminated union cleanly without `if`s in
  // the template.
  let provider = $derived(
    active.kind === "bg" ? active.session.provider : active.info.provider!,
  );

  let title = $derived.by(() => {
    if (active.kind === "tab") return `Terminal ${active.tabIndex + 1}`;
    if (active.kind === "segment") return `Terminal in ${shortCwd(active.info.cwd)}`;
    return providerName(active.session.provider);
  });

  let cwdLabel = $derived(
    active.kind === "bg" ? shortCwd(active.session.cwd) : shortCwd(active.info.cwd),
  );

  let startedAt = $derived(
    active.kind === "bg" ? active.session.started_at : null,
  );
</script>

<div
  class="session-item"
  data-testid="ai-active-row"
  data-kind={active.kind}
>
  <ProviderIcon {provider} size={20} />
  <div class="session-info">
    <div class="session-row-top">
      <span class="session-provider">{providerName(provider)}</span>
      {#if active.kind === "bg"}
        <BackgroundRunStatusBadge
          status={active.session.background_status!}
          compact
        />
      {/if}
      <span class="session-title">{title}</span>
    </div>
    <div class="session-row-bottom">
      <span class="session-cwd">{cwdLabel}</span>
      {#if startedAt}
        <span class="session-time">{formatRelativeTimeUnix(startedAt)}</span>
      {/if}
    </div>
  </div>
  <div class="session-actions">
    <button
      class="session-action-btn focus-btn"
      onclick={onFocusClick}
      title={m.ai_sessions_focus()}
      data-testid="ai-active-row-focus"
    >
      <span class="action-label">{m.ai_sessions_focus()}</span>
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

  .session-action-btn.focus-btn {
    color: var(--accent-blue);
    border-color: rgba(88, 166, 255, 0.3);
  }

  .session-action-btn.focus-btn:hover {
    background: rgba(88, 166, 255, 0.1);
  }

  .action-label {
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
</style>
