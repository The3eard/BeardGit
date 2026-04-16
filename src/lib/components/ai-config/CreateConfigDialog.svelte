<!--
  CreateConfigDialog.svelte — modal for creating new AI config files.

  Lets the user pick a type (agent/skill/prompt), enter a name, choose
  a scope (project/user), and previews the resulting file path before
  creating.
-->
<script lang="ts">
  import { createConfigFile } from "../../stores/aiConfig";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    defaultScope: string;
    onClose: () => void;
  }

  const { defaultScope, onClose }: Props = $props();

  let kind = $state<"agent" | "skill" | "prompt">("agent");
  let scope = $state(defaultScope);
  let name = $state("");
  let error = $state("");
  let creating = $state(false);

  let pathPreview = $derived.by(() => {
    const safeName = name.trim() || "my-file";
    const base = scope === "user" ? "~/.claude" : ".claude";
    switch (kind) {
      case "agent":
        return `${base}/agents/${safeName}.md`;
      case "skill":
        return `${base}/skills/${safeName}/SKILL.md`;
      case "prompt":
        return `${base}/prompts/${safeName}.md`;
    }
  });

  async function handleCreate() {
    const trimmed = name.trim();
    if (!trimmed) return;
    creating = true;
    error = "";
    try {
      await createConfigFile(kind, scope, trimmed);
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      creating = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
    } else if (e.key === "Enter" && name.trim()) {
      handleCreate();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div class="backdrop" onclick={onClose} role="button" tabindex="-1"></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={m.ai_config_create_title()}>
  <h3 class="dialog-title">{m.ai_config_create_title()}</h3>

  <!-- Type selector -->
  <div class="field">
    <div class="toggle-group">
      <button class="toggle-btn" class:active={kind === "agent"} onclick={() => (kind = "agent")}>
        {m.ai_config_type_agent()}
      </button>
      <button class="toggle-btn" class:active={kind === "skill"} onclick={() => (kind = "skill")}>
        {m.ai_config_type_skill()}
      </button>
      <button class="toggle-btn" class:active={kind === "prompt"} onclick={() => (kind = "prompt")}>
        {m.ai_config_type_prompt()}
      </button>
    </div>
  </div>

  <!-- Name input -->
  <div class="field">
    <label class="field-label" for="config-name">{m.ai_config_name()}</label>
    <input
      id="config-name"
      type="text"
      class="field-input"
      placeholder="my-config"
      bind:value={name}
    />
  </div>

  <!-- Path preview -->
  <div class="path-preview">{pathPreview}</div>

  <!-- Scope selector -->
  <div class="field">
    <div class="toggle-group">
      <button class="toggle-btn" class:active={scope === "project"} onclick={() => (scope = "project")}>
        {m.ai_config_scope_project()}
      </button>
      <button class="toggle-btn" class:active={scope === "user"} onclick={() => (scope = "user")}>
        {m.ai_config_scope_user()}
      </button>
    </div>
  </div>

  <!-- Error display -->
  {#if error}
    <div class="error-box">{error}</div>
  {/if}

  <!-- Actions -->
  <div class="dialog-actions">
    <button class="btn btn-cancel" onclick={onClose}>{m.ai_config_cancel()}</button>
    <button
      class="btn btn-create"
      disabled={!name.trim() || creating}
      onclick={handleCreate}
    >
      {creating ? "..." : m.ai_config_create_btn()}
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
    min-width: 360px;
    max-width: 440px;
    z-index: 1000;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .dialog-title {
    margin: 0 0 16px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .field {
    margin-bottom: 12px;
  }

  .field-label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
  }

  .field-input {
    width: 100%;
    padding: 7px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-mono);
    outline: none;
    box-sizing: border-box;
  }

  .field-input:focus {
    border-color: var(--accent-blue);
  }

  .toggle-group {
    display: flex;
    gap: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }

  .toggle-btn {
    flex: 1;
    padding: 6px 12px;
    background: var(--bg-primary);
    border: none;
    border-right: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .toggle-btn:last-child {
    border-right: none;
  }

  .toggle-btn.active {
    background: rgba(88, 166, 255, 0.15);
    color: var(--accent-blue);
    font-weight: 600;
  }

  .toggle-btn:hover:not(.active) {
    background: rgba(255, 255, 255, 0.04);
  }

  .path-preview {
    padding: 6px 10px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    margin-bottom: 12px;
    word-break: break-all;
  }

  .error-box {
    padding: 8px 10px;
    background: rgba(248, 81, 73, 0.12);
    border: 1px solid rgba(248, 81, 73, 0.3);
    border-radius: 6px;
    font-size: 12px;
    color: #f85149;
    margin-bottom: 12px;
    word-break: break-word;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
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

  .btn-create {
    background: var(--accent-blue);
    color: #fff;
    border-color: var(--accent-blue);
  }

  .btn-create:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-create:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
