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
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 999;
  }

  .dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 20px 24px;
    min-width: 320px;
    max-width: 420px;
    z-index: 1000;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .dialog-title {
    margin: 0 0 8px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
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

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn {
    padding: 6px 16px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    border: 1px solid var(--border);
    transition: background 0.15s;
  }

  .btn-cancel {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .btn-cancel:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .btn-confirm {
    background: var(--accent-blue);
    color: #fff;
    border-color: var(--accent-blue);
  }

  .btn-confirm:hover {
    opacity: 0.9;
  }

  .btn-confirm.destructive {
    background: #f85149;
    border-color: #f85149;
  }

  .btn-confirm.destructive:hover {
    opacity: 0.9;
  }
</style>
