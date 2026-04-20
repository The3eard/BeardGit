<!--
  CreateIssueDialog — modal for creating a new issue. Title + description
  are required; labels, assignees, and milestone are optional.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    createIssue,
    labelsCache,
    labelsCacheLoading,
    refreshLabelsCache,
  } from "../../stores/issues";
  import * as m from "$lib/paraglide/messages";
  import LabelPicker from "../common/LabelPicker.svelte";
  import AssigneePicker from "./AssigneePicker.svelte";
  import MilestonePicker from "./MilestonePicker.svelte";

  let { onClose }: { onClose: () => void } = $props();

  let titleInput = $state("");
  let bodyInput = $state("");
  let labels = $state<string[]>([]);
  let assignees = $state<string[]>([]);
  let milestoneId = $state<number | null>(null);
  let submitting = $state(false);
  let errorMsg = $state("");

  let showLabelPicker = $state(false);
  let showAssigneePicker = $state(false);
  let showMilestonePicker = $state(false);

  onMount(() => {
    if ($labelsCache.length === 0) void refreshLabelsCache();
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  function applyLabels(added: string[], removed: string[]) {
    const set = new Set(labels);
    for (const a of added) set.add(a);
    for (const r of removed) set.delete(r);
    labels = [...set];
    showLabelPicker = false;
  }

  function applyAssignees(added: string[], removed: string[]) {
    const set = new Set(assignees);
    for (const a of added) set.add(a);
    for (const r of removed) set.delete(r);
    assignees = [...set];
    showAssigneePicker = false;
  }

  function applyMilestone(id: number | null) {
    milestoneId = id;
    showMilestonePicker = false;
  }

  async function handleSubmit() {
    if (!titleInput.trim()) return;
    submitting = true;
    errorMsg = "";
    try {
      await createIssue(titleInput.trim(), bodyInput, labels, assignees, milestoneId);
      onClose();
    } catch (e) {
      errorMsg = m.issues_create_failed({ error: String(e) });
    } finally {
      submitting = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<button class="backdrop" type="button" onclick={onClose} aria-label={m.issues_cancel()}></button>
<div class="dialog" role="dialog" aria-modal="true" aria-label={m.issues_create_title()}>
  <h3 class="dialog-title">{m.issues_create_title()}</h3>

  <div class="form-field">
    <label for="issue-title">{m.issues_title_label()}</label>
    <!-- svelte-ignore a11y_autofocus -->
    <input id="issue-title" type="text" bind:value={titleInput} autofocus />
  </div>

  <div class="form-field">
    <label for="issue-body">{m.issues_description_label()}</label>
    <textarea id="issue-body" rows="5" bind:value={bodyInput}></textarea>
  </div>

  <div class="form-field">
    <label for="issue-labels-row">{m.issues_labels_label()}</label>
    <div class="chip-row" id="issue-labels-row">
      {#each labels as label}
        <span class="chip">{label}
          <button type="button" class="chip-x" onclick={() => labels = labels.filter(l => l !== label)}>×</button>
        </span>
      {/each}
      <button type="button" class="chip-add" onclick={() => showLabelPicker = true}>+ {m.issues_add_label()}</button>
    </div>
  </div>

  <div class="form-field">
    <label for="issue-assignees-row">{m.issues_assignees_label()}</label>
    <div class="chip-row" id="issue-assignees-row">
      {#each assignees as a}
        <span class="chip">{a}
          <button type="button" class="chip-x" onclick={() => assignees = assignees.filter(x => x !== a)}>×</button>
        </span>
      {/each}
      <button type="button" class="chip-add" onclick={() => showAssigneePicker = true}>+ {m.issues_add_assignee()}</button>
    </div>
  </div>

  <div class="form-field">
    <label for="issue-milestone-btn">{m.issues_milestone_label()}</label>
    <button id="issue-milestone-btn" type="button" class="milestone-btn" onclick={() => showMilestonePicker = true}>
      {milestoneId === null ? m.issues_no_milestone() : `#${milestoneId}`}
    </button>
  </div>

  {#if errorMsg}<p class="error-msg">{errorMsg}</p>{/if}

  <div class="dialog-actions">
    <button type="button" class="btn btn-cancel" onclick={onClose}>{m.issues_cancel()}</button>
    <button
      type="button"
      class="btn btn-primary"
      disabled={submitting || !titleInput.trim()}
      onclick={handleSubmit}
    >
      {submitting ? m.issues_loading() : m.issues_create_button()}
    </button>
  </div>
</div>

{#if showLabelPicker}
  <LabelPicker
    labels={$labelsCache}
    loading={$labelsCacheLoading}
    current={labels}
    onApply={applyLabels}
    onCancel={() => showLabelPicker = false}
  />
{/if}

{#if showAssigneePicker}
  <AssigneePicker
    current={assignees}
    onApply={applyAssignees}
    onCancel={() => showAssigneePicker = false}
  />
{/if}

{#if showMilestonePicker}
  <MilestonePicker
    current={milestoneId}
    onConfirm={applyMilestone}
    onCancel={() => showMilestonePicker = false}
  />
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 99;
    border: none;
    cursor: pointer;
  }
  .dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 100;
    min-width: 440px;
    max-width: 520px;
    max-height: 80vh;
    overflow-y: auto;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 18px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .dialog-title {
    margin: 0 0 4px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .form-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .form-field > label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .form-field input[type="text"],
  .form-field textarea {
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    box-sizing: border-box;
  }
  .form-field textarea {
    resize: vertical;
    min-height: 80px;
  }
  .chip-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip {
    padding: 2px 6px 2px 8px;
    border-radius: 10px;
    background: rgba(88, 166, 255, 0.15);
    color: var(--accent-blue);
    font-size: 11px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .chip-x {
    border: none;
    background: none;
    color: var(--accent-blue);
    cursor: pointer;
    font-size: 12px;
    padding: 0;
  }
  .chip-add {
    padding: 2px 8px;
    background: none;
    border: 1px dashed var(--border);
    border-radius: 10px;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
  }
  .milestone-btn {
    padding: 4px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
    align-self: flex-start;
  }
  .error-msg {
    margin: 0;
    padding: 6px 10px;
    background: rgba(248, 81, 73, 0.1);
    border: 1px solid rgba(248, 81, 73, 0.3);
    border-radius: 4px;
    color: var(--accent-red);
    font-size: 12px;
  }
  .dialog-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    padding-top: 6px;
    border-top: 1px solid var(--border);
  }
</style>
