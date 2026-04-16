<!--
  CreateMrPrDialog — modal dialog for creating a new merge request or pull request.

  Provides fields for source/target branch (text inputs), title, description,
  draft toggle, labels, and reviewers. Calls createMrPr from the store on submit.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { activeProvider } from "../../stores/provider";
  import { createMrPr } from "../../stores/mr-pr";
  import { getBranches } from "../../api/tauri";
  import * as m from "$lib/paraglide/messages";
  import type { BranchInfo } from "../../types";

  let { onClose }: { onClose: () => void } = $props();

  let isGitHub = $derived($activeProvider?.kind === "github");
  let dialogTitle = $derived(isGitHub ? m.mrpr_create_title_github() : m.mrpr_create_title());

  let branches = $state<BranchInfo[]>([]);
  let sourceBranch = $state("");
  let targetBranch = $state("");
  let titleInput = $state("");
  let bodyInput = $state("");
  let isDraft = $state(false);
  let labelsInput = $state("");
  let reviewersInput = $state("");
  let submitting = $state(false);
  let errorMsg = $state("");

  onMount(async () => {
    try {
      branches = await getBranches();
      // Default source to current branch (HEAD)
      const head = branches.find(b => b.is_head);
      if (head) sourceBranch = head.name;
      // Default target to main/master/develop
      const defaultTarget = branches.find(b =>
        b.name === "main" || b.name === "master" || b.name === "develop"
      );
      if (defaultTarget) targetBranch = defaultTarget.name;
    } catch {
      // branches not available
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  async function handleSubmit() {
    if (!sourceBranch || !targetBranch || !titleInput.trim()) return;
    submitting = true;
    errorMsg = "";
    try {
      const labels = labelsInput.trim()
        ? labelsInput.split(",").map(l => l.trim()).filter(Boolean)
        : [];
      const reviewers = reviewersInput.trim()
        ? reviewersInput.split(",").map(r => r.trim()).filter(Boolean)
        : [];
      await createMrPr(
        sourceBranch, targetBranch, titleInput.trim(), bodyInput,
        isDraft, labels, reviewers
      );
      onClose();
    } catch (e) {
      errorMsg = m.mrpr_create_failed({ error: String(e) });
    } finally {
      submitting = false;
    }
  }

  // Local branch names for dropdown
  let localBranches = $derived(branches.filter(b => !b.is_remote).map(b => b.name));
</script>

<svelte:window onkeydown={handleKeydown} />

<button class="backdrop" onclick={onClose} tabindex="-1" aria-label="Close dialog"></button>
<div class="dialog" role="dialog" aria-modal="true" aria-label={dialogTitle}>
  <h3 class="dialog-title">{dialogTitle}</h3>

  <div class="form-field">
    <label for="source-branch">{m.mrpr_source_branch()}</label>
    <select id="source-branch" bind:value={sourceBranch}>
      {#each localBranches as branch}
        <option value={branch}>{branch}</option>
      {/each}
    </select>
  </div>

  <div class="form-field">
    <label for="target-branch">{m.mrpr_target_branch()}</label>
    <select id="target-branch" bind:value={targetBranch}>
      {#each localBranches as branch}
        <option value={branch}>{branch}</option>
      {/each}
    </select>
  </div>

  <div class="form-field">
    <label for="mrpr-title">{m.mrpr_title_label()}</label>
    <input id="mrpr-title" type="text" bind:value={titleInput} />
  </div>

  <div class="form-field">
    <label for="mrpr-body">{m.mrpr_description_label()}</label>
    <textarea id="mrpr-body" rows="4" bind:value={bodyInput}></textarea>
  </div>

  <div class="form-field inline">
    <label>
      <input type="checkbox" bind:checked={isDraft} />
      {m.mrpr_draft_label()}
    </label>
  </div>

  <div class="form-field">
    <label for="mrpr-labels">{m.mrpr_labels_label()}</label>
    <input id="mrpr-labels" type="text" bind:value={labelsInput} />
  </div>

  <div class="form-field">
    <label for="mrpr-reviewers">{m.mrpr_reviewers_label()}</label>
    <input id="mrpr-reviewers" type="text" bind:value={reviewersInput} />
  </div>

  {#if errorMsg}
    <p class="error-msg">{errorMsg}</p>
  {/if}

  <div class="dialog-actions">
    <button class="btn btn-cancel" onclick={onClose}>{m.mrpr_cancel()}</button>
    <button
      class="btn btn-confirm"
      disabled={submitting || !sourceBranch || !targetBranch || !titleInput.trim()}
      onclick={handleSubmit}
    >
      {submitting ? m.mrpr_loading() : m.mrpr_create_button()}
    </button>
  </div>
</div>

<style>
  /* dialog.css provides: .backdrop, .dialog, .dialog-title, .dialog-actions, .btn, .btn-cancel, .btn-confirm */

  .dialog {
    min-width: 440px;
    max-width: 520px;
    max-height: 80vh;
    overflow-y: auto;
  }

  .form-field {
    margin-bottom: 12px;
  }

  .form-field label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .form-field.inline label {
    display: flex;
    align-items: center;
    gap: 6px;
    text-transform: none;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
  }

  .form-field input[type="text"],
  .form-field select,
  .form-field textarea {
    width: 100%;
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
    min-height: 60px;
  }

  .form-field select {
    appearance: none;
    cursor: pointer;
  }

  .error-msg {
    margin: 0 0 12px;
    padding: 6px 10px;
    background: rgba(248, 81, 73, 0.1);
    border: 1px solid rgba(248, 81, 73, 0.3);
    border-radius: 4px;
    color: var(--accent-red);
    font-size: 12px;
  }

  .dialog-actions {
    padding-top: 8px;
  }
</style>
