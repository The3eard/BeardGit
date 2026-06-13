<!--
  CreateConfigDialog.svelte — modal for creating new AI config files.

  Lets the user pick a type (agent/skill/prompt), enter a name, choose
  a scope (project/user), and previews the resulting file path before
  creating.
-->
<script lang="ts">
  import { createConfigFile } from "../../stores/aiConfig";
  import * as m from "$lib/paraglide/messages";
  import Button from "$lib/components/ui/Button.svelte";

  interface Props {
    defaultScope: string;
    onClose: () => void;
  }

  const { defaultScope, onClose }: Props = $props();

  let kind = $state<"agent" | "skill" | "prompt">("agent");
  // svelte-ignore state_referenced_locally — intentional: scope is initialized from prop then mutated locally
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

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose} onkeydown={handleKeydown} role="presentation"></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={m.ai_config_create_title()}>
  <h3 class="dialog-title">{m.ai_config_create_title()}</h3>

  <!-- Type selector -->
  <div class="field">
    <div class="toggle-group">
      <Button variant="neutral" size="sm" active={kind === "agent"} onclick={() => (kind = "agent")}>{m.ai_config_type_agent()}</Button>
      <Button variant="neutral" size="sm" active={kind === "skill"} onclick={() => (kind = "skill")}>{m.ai_config_type_skill()}</Button>
      <Button variant="neutral" size="sm" active={kind === "prompt"} onclick={() => (kind = "prompt")}>{m.ai_config_type_prompt()}</Button>
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
      <Button variant="neutral" size="sm" active={scope === "project"} onclick={() => (scope = "project")}>{m.ai_config_scope_project()}</Button>
      <Button variant="neutral" size="sm" active={scope === "user"} onclick={() => (scope = "user")}>{m.ai_config_scope_user()}</Button>
    </div>
  </div>

  <!-- Error display -->
  {#if error}
    <div class="error-box">{error}</div>
  {/if}

  <!-- Actions -->
  <div class="dialog-actions">
    <Button variant="neutral" onclick={onClose}>{m.ai_config_cancel()}</Button>
    <Button
      variant="primary"
      disabled={!name.trim() || creating}
      onclick={handleCreate}
    >
      {creating ? "..." : m.ai_config_create_btn()}
    </Button>
  </div>
</div>

<style>
  /* dialog.css provides: .backdrop, .dialog, .dialog-title, .dialog-actions */

  .dialog {
    min-width: 360px;
    max-width: 440px;
  }

  .field {
    margin-bottom: 12px;
  }

  .field-label {
    display: block;
    font-size: var(--font-size-xs);
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
    font-size: var(--font-size-md);
    font-family: var(--font-mono);
    outline: none;
    box-sizing: border-box;
  }

  .field-input:focus {
    border-color: var(--accent-primary);
  }

  .toggle-group {
    display: flex;
    gap: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }

  .path-preview {
    padding: 6px 10px;
    background: color-mix(in srgb, var(--text-primary) 3%, transparent);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: var(--font-size-xs);
    font-family: var(--font-mono);
    color: var(--text-secondary);
    margin-bottom: 12px;
    word-break: break-all;
  }

  .error-box {
    padding: 8px 10px;
    background: var(--overlay-accent-red);
    border: 1px solid color-mix(in srgb, var(--accent-red) 30%, transparent);
    border-radius: 6px;
    font-size: var(--font-size-sm);
    color: var(--accent-red);
    margin-bottom: 12px;
    word-break: break-word;
  }

  .dialog-actions {
    margin-top: 4px;
  }
</style>
