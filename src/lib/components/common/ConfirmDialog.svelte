<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import Button from "$lib/components/ui/Button.svelte";

  let {
    title,
    detail,
    message,
    confirmLabel = m.confirm_confirm(),
    cancelLabel = m.confirm_cancel(),
    destructive = false,
    /**
     * Optional inline checkbox rendered between the message and the
     * action row. When set, the dialog binds the checked state to
     * `checkboxChecked` (writable from the caller via `bind:`). Used
     * by destructive flows that need an upfront escalation toggle —
     * e.g. "Force remove (discard uncommitted changes)" on a
     * worktree.
     *
     * Leave undefined to keep the dialog's classic two-button shape.
     */
    checkboxLabel,
    checkboxChecked = $bindable(false),
    onConfirm,
    onCancel,
  }: {
    title: string;
    detail?: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
    destructive?: boolean;
    checkboxLabel?: string;
    checkboxChecked?: boolean;
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
<div class="dialog" data-testid="dialog-confirm" role="dialog" aria-modal="true" aria-label={title}>
  <h3 class="dialog-title" data-testid="dialog-title">{title}</h3>
  {#if detail}
    <p class="dialog-detail">{detail}</p>
  {/if}
  <p class="dialog-message">{message}</p>
  {#if checkboxLabel}
    <label class="dialog-checkbox">
      <input
        type="checkbox"
        bind:checked={checkboxChecked}
        data-testid="dialog-checkbox"
      />
      <span>{checkboxLabel}</span>
    </label>
  {/if}
  <div class="dialog-actions">
    <Button variant="neutral" testid="dialog-cancel-btn" onclick={onCancel}>{cancelLabel}</Button>
    <Button
      variant="primary"
      testid="dialog-confirm-btn"
      onclick={onConfirm}
    >
      {confirmLabel}
    </Button>
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
    background: color-mix(in srgb, var(--text-primary) 4%, transparent);
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

  .dialog-checkbox {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: -8px 0 16px;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
    user-select: none;
  }

  .dialog-checkbox input {
    margin: 0;
    cursor: pointer;
  }
</style>
