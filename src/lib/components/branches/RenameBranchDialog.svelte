<!--
  RenameBranchDialog — single-field dialog for renaming a local branch.
  After a successful rename, if the renamed branch was the selected
  branch in the Branches panel, the selection is moved to the new name.
-->
<script lang="ts">
  import { get } from "svelte/store";
  import { renameBranch } from "../../api/tauri";
  import { runMutation } from "../../api/runMutation";
  import { selectedBranchName } from "../../stores/branches";

  let {
    open,
    currentName,
    onClose,
  }: {
    open: boolean;
    currentName: string;
    onClose: () => void;
  } = $props();

  let newName = $state(currentName);
  let submitting = $state(false);
  let primed = false;

  $effect(() => {
    if (open && !primed) {
      // Read the prop inside the effect so Svelte tracks the dependency correctly.
      newName = currentName;
      submitting = false;
      primed = true;
    }
    if (!open) primed = false;
  });

  let trimmed = $derived(newName.trim());
  let disabled = $derived(submitting || trimmed.length === 0 || trimmed === currentName);

  async function handleRename() {
    if (disabled) return;
    submitting = true;
    const target = trimmed;
    const oldName = currentName;
    try {
      await runMutation({
        kind: "branch_rename",
        invoke: () => renameBranch(oldName, target),
        successToast: () => `Renamed ${oldName} → ${target}`,
        failureToastPrefix: "Rename failed",
      });
      if (get(selectedBranchName) === oldName) {
        selectedBranchName.set(target);
      }
      onClose();
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
    else if (e.key === "Enter" && !disabled) {
      e.preventDefault();
      void handleRename();
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div class="backdrop" onclick={onClose} onkeydown={handleKeydown} role="button" tabindex="-1"></div>
  <div
    class="dialog"
    data-testid="dialog-rename-branch"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label="Rename branch"
    onkeydown={handleKeydown}
  >
    <h3 class="dialog-title">Rename branch</h3>
    <label class="field">
      <span class="label">New name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        data-testid="rename-branch-input"
        type="text"
        class="input"
        bind:value={newName}
        autofocus
        spellcheck="false"
        autocomplete="off"
      />
    </label>
    <div class="dialog-actions">
      <button class="btn btn-cancel" type="button" onclick={onClose}>Cancel</button>
      <button
        class="btn btn-confirm"
        data-testid="rename-branch-submit"
        type="button"
        {disabled}
        onclick={handleRename}
      >
        Rename
      </button>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 1001;
    background: rgba(0, 0, 0, 0.5);
  }

  .dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 1002;
    min-width: 360px;
    max-width: 480px;
    background: var(--bg-toolbar);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 20px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .dialog-title {
    margin: 0 0 16px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 16px;
  }

  .label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--text-secondary);
  }

  .input {
    width: 100%;
    padding: 6px 10px;
    font-size: 13px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    box-sizing: border-box;
  }

  .input:focus {
    outline: none;
    border-color: var(--accent-blue);
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn {
    padding: 6px 16px;
    font-size: 12px;
    border-radius: 4px;
    cursor: pointer;
    border: 1px solid var(--border);
  }

  .btn-cancel {
    background: none;
    color: var(--text-secondary);
  }

  .btn-confirm {
    background: var(--accent-blue);
    color: #fff;
    border-color: var(--accent-blue);
  }

  .btn-confirm:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
