<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";

  let {
    title,
    detail,
    message,
    confirmLabel = m.confirm_confirm(),
    cancelLabel = m.confirm_cancel(),
    destructive = false,
    onConfirm,
    onCancel,
  }: {
    title: string;
    detail?: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
    destructive?: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onCancel();
    } else if (e.key === "Enter") {
      onConfirm();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    return () => window.removeEventListener("keydown", handleKeydown);
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div class="backdrop" onclick={onCancel} onkeydown={(e) => { if (e.key === "Escape") onCancel(); }} role="button" tabindex="-1"></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={title}>
  <h3 class="dialog-title">{title}</h3>
  {#if detail}
    <p class="dialog-detail">{detail}</p>
  {/if}
  <p class="dialog-message">{message}</p>
  <div class="dialog-actions">
    <button class="btn btn-cancel" onclick={onCancel}>{cancelLabel}</button>
    <button
      class="btn btn-confirm"
      class:destructive
      onclick={onConfirm}
    >
      {confirmLabel}
    </button>
  </div>
</div>

<style>
  /* dialog.css provides: .backdrop, .dialog, .dialog-title, .dialog-actions, .btn, .btn-cancel, .btn-confirm */

  .dialog {
    min-width: 320px;
    max-width: 420px;
  }

  .dialog-title {
    margin-bottom: 8px;
  }

  .dialog-detail {
    margin: 0 0 12px;
    padding: 8px 10px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--text-primary);
    line-height: 1.5;
    white-space: pre-line;
  }

  .dialog-message {
    margin: 0 0 20px;
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .btn-confirm.destructive {
    background: var(--accent-red, #f85149);
    border-color: var(--accent-red, #f85149);
  }
</style>
