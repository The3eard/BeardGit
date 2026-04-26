<script lang="ts">
  import { onMount } from "svelte";
  import { addWorktree } from "../../stores/worktrees";
  import * as m from "$lib/paraglide/messages";
  import Button from "$lib/components/ui/Button.svelte";

  interface Props {
    onClose: () => void;
    repoPath: string;
  }

  let { onClose, repoPath }: Props = $props();

  let branch = $state("");
  let path = $state("");
  let createNewBranch = $state(true);
  let submitting = $state(false);
  let error = $state<string | null>(null);

  /** Auto-suggest worktree path from parent dir + branch name. */
  let autoPath = $derived.by(() => {
    if (!branch) return "";
    const parent = repoPath.replace(/\\/g, "/").replace(/\/[^/]+$/, "");
    const safeBranch = branch.replace(/[/\\]/g, "-");
    return `${parent}/${safeBranch}`;
  });

  /** Use user-entered path if they typed one, otherwise the auto-suggestion. */
  let pathEdited = $state(false);
  let effectivePath = $derived(pathEdited ? path : autoPath);

  function handlePathInput(value: string) {
    path = value;
    pathEdited = true;
  }

  async function handleCreate() {
    if (!branch.trim() || !effectivePath.trim()) return;
    submitting = true;
    error = null;
    try {
      await addWorktree(effectivePath.trim(), branch.trim(), createNewBranch);
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
    } else if (e.key === "Enter" && !submitting) {
      handleCreate();
    }
  }

  let branchInput: HTMLInputElement | undefined = $state();

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    branchInput?.focus();
    return () => window.removeEventListener("keydown", handleKeydown);
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div class="backdrop" onclick={onClose} onkeydown={(e) => { if (e.key === "Escape") onClose(); }} role="button" tabindex="-1"></div>
<div class="dialog" role="dialog" aria-modal="true" aria-label={m.worktree_create_title()}>
  <h3 class="dialog-title">{m.worktree_create_title()}</h3>

  <div class="form-field">
    <label class="field-label" for="wt-branch">{m.worktree_branch_label()}</label>
    <input
      id="wt-branch"
      type="text"
      class="field-input"
      bind:this={branchInput}
      bind:value={branch}
      placeholder="feature/my-branch"
    />
  </div>

  <div class="form-field">
    <label class="field-label" for="wt-path">{m.worktree_path_label()}</label>
    <input
      id="wt-path"
      type="text"
      class="field-input"
      value={effectivePath}
      oninput={(e) => handlePathInput(e.currentTarget.value)}
      placeholder={autoPath || "/path/to/worktree"}
    />
  </div>

  <label class="checkbox-label">
    <input type="checkbox" bind:checked={createNewBranch} />
    <span>{m.worktree_create_new_branch()}</span>
  </label>

  {#if error}
    <p class="error-text">{error}</p>
  {/if}

  <div class="dialog-actions">
    <Button variant="neutral" onclick={onClose} disabled={submitting}>
      {m.confirm_cancel()}
    </Button>
    <Button
      variant="primary"
      onclick={handleCreate}
      disabled={submitting || !branch.trim() || !effectivePath.trim()}
    >
      {submitting ? "..." : m.worktree_create_button()}
    </Button>
  </div>
</div>

<style>
  /* dialog.css provides: .backdrop, .dialog, .dialog-title, .dialog-actions */

  .dialog {
    min-width: 360px;
    max-width: 480px;
  }

  .form-field {
    margin-bottom: 12px;
  }

  .field-label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    margin-bottom: 4px;
  }

  .field-input {
    width: 100%;
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-mono);
    outline: none;
    box-sizing: border-box;
  }

  .field-input:focus {
    border-color: var(--accent-blue);
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-primary);
    margin-bottom: 16px;
    cursor: pointer;
  }

  .checkbox-label input[type="checkbox"] {
    accent-color: var(--accent-blue);
  }

  .error-text {
    font-size: 12px;
    color: var(--accent-red);
    margin: 0 0 12px;
    padding: 6px 10px;
    background: var(--overlay-accent-red);
    border-radius: 6px;
    word-break: break-word;
  }

</style>
