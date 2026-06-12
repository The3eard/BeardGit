<!--
  ErrorDialog.svelte — Modal popup shown when an operation fails.

  Displays the error message, a collapsible details section for stack
  traces or raw errors, and action buttons to copy the error info to
  clipboard or open the log directory.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import { getDebugInfo, openLogDirectory } from "$lib/api/tauri";
  import Button from "$lib/components/ui/Button.svelte";
  import type { DebugInfo } from "$lib/types";

  interface Props {
    title?: string;
    message: string;
    detail?: string;
    onClose: () => void;
  }

  let {
    title = m.error_dialog_title(),
    message,
    detail,
    onClose,
  }: Props = $props();

  let detailsExpanded = $state(false);
  let copied = $state(false);
  let debugInfo = $state<DebugInfo | null>(null);

  onMount(() => {
    getDebugInfo().then((info) => { debugInfo = info; }).catch(() => {});
    window.addEventListener("keydown", handleKeydown);
    return () => window.removeEventListener("keydown", handleKeydown);
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
    }
  }

  async function copyError() {
    let text = `${title}\n${message}`;
    if (detail) {
      text += `\n\n${detail}`;
    }
    if (debugInfo) {
      text += `\n\n--- Debug Info ---`;
      text += `\nApp: ${debugInfo.app_version}`;
      text += `\nOS: ${debugInfo.os}`;
      text += `\nGit: ${debugInfo.git_version ?? "unknown"}`;
      text += `\nLogs: ${debugInfo.log_path}`;
    }
    await navigator.clipboard.writeText(text);
    copied = true;
    setTimeout(() => { copied = false; }, 2000);
  }

  async function handleOpenLog() {
    try {
      await openLogDirectory();
    } catch {
      // Best-effort — ignore if it fails
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose} onkeydown={(e) => { if (e.key === 'Escape') onClose(); }} role="presentation"></div>
<div class="dialog" data-testid="dialog-error" role="alertdialog" aria-modal="true" aria-label={title}>
  <div class="dialog-header">
    <span class="error-icon">{"\uF0028"}</span>
    <h3 class="dialog-title" data-testid="dialog-title">{title}</h3>
  </div>

  <p class="dialog-message">{message}</p>

  {#if detail}
    <button
      class="details-toggle"
      onclick={() => { detailsExpanded = !detailsExpanded; }}
    >
      <span class="chevron" class:expanded={detailsExpanded}>{"\uEAB6"}</span>
      {m.error_dialog_details()}
    </button>
    {#if detailsExpanded}
      <pre class="details-content">{detail}</pre>
    {/if}
  {/if}

  <div class="dialog-actions">
    <Button variant="neutral" icon={"\uF0219"} onclick={handleOpenLog}>
      {m.error_dialog_open_log()}
    </Button>
    <Button variant="neutral" icon={"\uF0C5"} onclick={copyError}>
      {copied ? m.error_dialog_copied() : m.error_dialog_copy()}
    </Button>
    <Button variant="primary" testid="dialog-dismiss-btn" onclick={onClose}>
      {m.error_dialog_close()}
    </Button>
  </div>
</div>

<style>
  /* dialog.css provides: .backdrop, .dialog, .dialog-title, .dialog-actions */

  .dialog {
    min-width: 360px;
    max-width: 480px;
  }

  .dialog-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }

  .error-icon {
    font-family: var(--font-icons);
    font-size: 18px;
    color: var(--accent-red);
  }

  .dialog-title {
    margin: 0;
  }

  .dialog-message {
    margin: 0 0 16px;
    font-size: var(--font-size-md);
    color: var(--text-secondary);
    line-height: 1.5;
    word-break: break-word;
  }

  .details-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    padding: 0;
    margin-bottom: 8px;
    font-size: var(--font-size-sm);
    color: var(--text-tertiary);
    cursor: pointer;
  }

  .details-toggle:hover {
    color: var(--text-primary);
  }

  .chevron {
    font-family: var(--font-icons);
    font-size: var(--font-size-sm);
    transition: transform 0.15s;
    transform: rotate(0deg);
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  .details-content {
    margin: 0 0 16px;
    padding: 8px 10px;
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: var(--font-size-xs);
    font-family: var(--font-mono);
    color: var(--text-secondary);
    line-height: 1.5;
    max-height: 200px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }

</style>
