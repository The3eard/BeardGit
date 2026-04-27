<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import { doCreateTag } from "../../stores/tags";
  import Button from "$lib/components/ui/Button.svelte";

  let { onClose }: { onClose: () => void } = $props();

  let name = $state("");
  let target = $state("");
  let annotated = $state(false);
  let message = $state("");
  let error = $state<string | null>(null);
  let creating = $state(false);

  const INVALID_CHARS = /[\s~^:?*\[\\]/;

  function validate(): string | null {
    if (!name.trim()) return "Tag name is required";
    if (INVALID_CHARS.test(name)) return "Tag name contains invalid characters";
    if (annotated && !message.trim()) return "Message is required for annotated tags";
    return null;
  }

  async function handleCreate() {
    const err = validate();
    if (err) {
      error = err;
      return;
    }
    error = null;
    creating = true;
    try {
      await doCreateTag(name.trim(), target.trim() || "HEAD", annotated ? message.trim() : null);
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      creating = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
    if (e.key === "Enter" && !annotated) handleCreate();
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    return () => window.removeEventListener("keydown", handleKeydown);
  });
</script>

<div class="dialog-backdrop" onclick={onClose} onkeydown={(e) => { if (e.key === "Escape") onClose(); }} role="button" tabindex="-1">
  <div class="dialog-card" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1">
    <div class="dialog-header">
      <h3 class="dialog-title">{m.tags_create_dialog_title()}</h3>
    </div>

    <div class="dialog-body">
      <div class="field">
        <label class="field-label" for="tag-name">{m.tags_create_name_label()}</label>
        <input
          id="tag-name"
          type="text"
          class="field-input"
          placeholder={m.tags_create_name_placeholder()}
          bind:value={name}
          />
      </div>

      <div class="field">
        <label class="field-label" for="tag-target">{m.tags_create_target_label()}</label>
        <input
          id="tag-target"
          type="text"
          class="field-input mono"
          placeholder={m.tags_create_target_placeholder()}
          bind:value={target}
        />
        <span class="field-hint">{m.tags_create_commit_hint()}</span>
      </div>

      <div class="field-row">
        <label class="toggle-label">
          <input type="checkbox" bind:checked={annotated} />
          <span>{m.tags_create_annotated_label()}</span>
        </label>
      </div>

      {#if annotated}
        <div class="field">
          <label class="field-label" for="tag-message">{m.tags_create_message_label()}</label>
          <textarea
            id="tag-message"
            class="field-textarea"
            placeholder={m.tags_create_message_placeholder()}
            bind:value={message}
            rows={3}
          ></textarea>
        </div>
      {/if}

      {#if error}
        <div class="error-msg">{error}</div>
      {/if}
    </div>

    <div class="dialog-footer">
      <Button variant="neutral" onclick={onClose}>{m.tags_create_cancel()}</Button>
      <Button variant="primary" onclick={handleCreate} disabled={creating}>
        {#if creating}
          <div class="spinner spinner--small"></div>
        {:else}
          {m.tags_create_confirm()}
        {/if}
      </Button>
    </div>
  </div>
</div>

<style>
  /* dialog.css provides: .dialog-title */
  /* This dialog uses .dialog-backdrop + .dialog-card (flex-centered), not the standard .backdrop + .dialog */

  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6); /* beardgit:allow-hex: modal backdrop neutral */
    z-index: 999;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .dialog-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 100%;
    max-width: 420px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5); /* beardgit:allow-hex: shadow neutral always-dark */
    z-index: 1000;
    overflow: hidden;
  }

  .dialog-header {
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--border);
  }

  .dialog-title {
    margin: 0;
  }

  .dialog-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .field-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .field-input {
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
    transition: border-color 0.15s;
  }

  .field-input:focus {
    border-color: var(--accent-blue);
  }

  .field-input.mono {
    font-family: var(--font-mono);
  }

  .field-hint {
    font-size: 11px;
    color: var(--text-secondary);
    opacity: 0.7;
  }

  .field-textarea {
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
    resize: vertical;
    font-family: inherit;
    line-height: 1.5;
    transition: border-color 0.15s;
  }

  .field-textarea:focus {
    border-color: var(--accent-blue);
  }

  .field-row {
    display: flex;
    align-items: center;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
    user-select: none;
  }

  .toggle-label input[type="checkbox"] {
    accent-color: var(--accent-blue);
    width: 14px;
    height: 14px;
    cursor: pointer;
  }

  .error-msg {
    font-size: 12px;
    color: var(--accent-red);
    padding: 6px 10px;
    background: var(--overlay-accent-red);
    border: 1px solid color-mix(in srgb, var(--accent-red) 25%, transparent);
    border-radius: 5px;
  }

  .dialog-footer {
    padding: 12px 20px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .spinner--small {
    width: 12px;
    height: 12px;
    border-width: 1.5px;
  }
</style>
