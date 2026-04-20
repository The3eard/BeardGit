<script lang="ts">
  import { onMount } from "svelte";
  import { listCiWorkflows, triggerWorkflow, activeProvider } from "../../stores/provider";
  import { repoInfo } from "../../stores/repo";
  import type { Workflow } from "../../types";
  import * as m from "$lib/paraglide/messages";

  interface Props {
    open: boolean;
    onClose: () => void;
  }
  let { open, onClose }: Props = $props();

  let workflows = $state<Workflow[]>([]);
  let selectedWorkflowId = $state<string>("");
  let gitRef = $state<string>("");
  let pairs = $state<{ key: string; value: string }[]>([{ key: "", value: "" }]);
  let loading = $state(false);
  let submitting = $state(false);
  let error = $state<string | null>(null);

  // For GitLab the workflow dropdown hides (single placeholder).
  // For GitHub the dropdown is the primary input.
  let isGitHub = $derived($activeProvider?.kind === "github");

  onMount(() => {
    if (open) init();
  });

  $effect(() => {
    if (open && workflows.length === 0) {
      init();
    }
  });

  async function init() {
    loading = true;
    error = null;
    try {
      workflows = await listCiWorkflows();
      if (workflows.length > 0 && !selectedWorkflowId) {
        selectedWorkflowId = workflows[0].id;
      }
      gitRef = $repoInfo?.head_branch ?? "";
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function addPair() {
    pairs = [...pairs, { key: "", value: "" }];
  }

  function removePair(idx: number) {
    pairs = pairs.filter((_, i) => i !== idx);
    if (pairs.length === 0) pairs = [{ key: "", value: "" }];
  }

  async function submit() {
    if (submitting) return;
    submitting = true;
    error = null;
    try {
      const inputs: Record<string, string> = {};
      for (const { key, value } of pairs) {
        const k = key.trim();
        if (k.length > 0) inputs[k] = value;
      }
      await triggerWorkflow(selectedWorkflowId, gitRef, inputs);
      onClose();
    } catch (e) {
      error = m.pipeline_trigger_error({ error: String(e) });
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

{#if open}
  <div
    class="dialog-overlay"
    role="presentation"
    onclick={onClose}
    onkeydown={handleKeydown}
  >
    <div
      class="dialog"
      role="dialog"
      tabindex="-1"
      aria-labelledby="trigger-wf-title"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="dialog-header">
        <h2 id="trigger-wf-title">{m.pipeline_trigger_dialog_title()}</h2>
        <button class="close-btn nf" onclick={onClose} aria-label="Close">{"\uF00D"}</button>
      </div>

      <div class="dialog-body">
        {#if loading}
          <div class="spinner"></div>
        {:else if workflows.length === 0}
          <div class="empty">{m.pipeline_trigger_no_workflows()}</div>
        {:else}
          {#if isGitHub}
            <label>
              {m.pipeline_trigger_workflow_label()}
              <select bind:value={selectedWorkflowId}>
                {#each workflows as wf (wf.id)}
                  <option value={wf.id} disabled={wf.state === "disabled"}>
                    {wf.name} ({wf.path})
                  </option>
                {/each}
              </select>
            </label>
          {/if}

          <label>
            {m.pipeline_trigger_ref_label()}
            <input type="text" bind:value={gitRef} placeholder="main" />
          </label>

          <fieldset>
            <legend>
              {isGitHub
                ? m.pipeline_trigger_inputs_label()
                : m.pipeline_trigger_variables_label()}
            </legend>
            {#each pairs as pair, i (i)}
              <div class="pair-row">
                <input
                  type="text"
                  bind:value={pair.key}
                  placeholder={m.pipeline_variable_key_placeholder()}
                />
                <input
                  type="text"
                  bind:value={pair.value}
                  placeholder={m.pipeline_variable_value_placeholder()}
                />
                <button
                  class="pair-remove nf"
                  onclick={() => removePair(i)}
                  aria-label="Remove"
                >{"\uF00D"}</button>
              </div>
            {/each}
            <button class="pair-add" onclick={addPair}>
              {m.pipeline_trigger_add_variable()}
            </button>
          </fieldset>
        {/if}

        {#if error}
          <div class="dialog-error">{error}</div>
        {/if}
      </div>

      <div class="dialog-footer">
        <button class="btn btn-secondary" onclick={onClose} disabled={submitting}>
          {m.pipeline_trigger_cancel()}
        </button>
        <button
          class="btn btn-primary"
          onclick={submit}
          disabled={submitting || loading || workflows.length === 0 || !gitRef.trim()}
        >
          {#if submitting}<div class="spinner spinner--sm"></div>{/if}
          {m.pipeline_trigger_submit()}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .dialog-overlay {
    position: fixed; inset: 0; background: rgba(0, 0, 0, 0.5);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .dialog {
    background: var(--bg-primary); border: 1px solid var(--border);
    border-radius: 8px; width: 500px; max-width: 90vw; max-height: 80vh;
    display: flex; flex-direction: column;
  }
  .dialog-header {
    padding: 12px 16px; display: flex; align-items: center; justify-content: space-between;
    border-bottom: 1px solid var(--border);
  }
  .dialog-header h2 { margin: 0; font-size: 14px; color: var(--text-primary); }
  .dialog-body { padding: 16px; display: flex; flex-direction: column; gap: 12px; overflow-y: auto; }
  .dialog-body label { display: flex; flex-direction: column; font-size: 12px; color: var(--text-secondary); gap: 4px; }
  .dialog-body input, .dialog-body select {
    background: var(--bg-secondary); color: var(--text-primary);
    border: 1px solid var(--border); border-radius: 4px; padding: 6px 8px; font-size: 12px;
  }
  fieldset {
    border: 1px solid var(--border); border-radius: 4px; padding: 8px; margin: 0;
    display: flex; flex-direction: column; gap: 6px;
  }
  legend { font-size: 11px; color: var(--text-secondary); padding: 0 4px; }
  .pair-row { display: grid; grid-template-columns: 1fr 1fr auto; gap: 4px; }
  .pair-remove { background: none; border: none; color: var(--text-secondary); cursor: pointer; font-family: var(--font-icons); }
  .pair-add {
    background: none; border: 1px dashed var(--border); color: var(--accent-blue);
    border-radius: 4px; padding: 6px; font-size: 11px; cursor: pointer;
  }
  .dialog-footer {
    padding: 12px 16px; border-top: 1px solid var(--border);
    display: flex; justify-content: flex-end; gap: 8px;
  }
  .dialog-error { color: var(--accent-red); font-size: 12px; background: rgba(248, 81, 73, 0.1); padding: 6px 8px; border-radius: 4px; }
  .empty { color: var(--text-secondary); font-size: 12px; }
  .close-btn { background: none; border: none; color: var(--text-secondary); cursor: pointer; font-family: var(--font-icons); }
  .spinner { border: 2px solid var(--border); border-top-color: var(--accent-blue); border-radius: 50%; width: 20px; height: 20px; animation: spin 0.8s linear infinite; align-self: center; }
  .spinner--sm { width: 10px; height: 10px; border-width: 1.5px; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
