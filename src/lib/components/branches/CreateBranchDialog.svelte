<!--
  CreateBranchDialog — single "create branch" entry point for every
  call site (Branches header, context menu, graph, reflog, ⌘⇧B).

  The dialog is controlled via `open`. On submit it decides which
  Tauri command to dispatch based on the source `kind` and optionally
  chains a `checkoutBranch` call.
-->
<script lang="ts">
  import { get } from "svelte/store";
  import { createBranch, createBranchAt, checkoutBranch } from "../../api/tauri";
  import Button from "$lib/components/ui/Button.svelte";
  import { Checkbox } from "$lib/components/ui";
  import { runMutation } from "../../api/runMutation";
  import { remoteNames } from "../../stores/remotes";
  import { branches, localBranches, remoteBranches } from "../../stores/branches";
  import { suggestLocalName, type InitialSource } from "./suggest-local-name";
  import { shortOid } from "../../utils/git";

  let {
    open,
    initialSource,
    onClose,
  }: {
    open: boolean;
    initialSource: InitialSource;
    onClose: () => void;
  } = $props();

  /** Internal working copy of the source — user can switch it to any branch tip. */
  let source = $state<InitialSource>(initialSource);
  let name = $state("");
  let checkout = $state(true);
  let submitting = $state(false);
  let showSourcePicker = $state(false);
  /** Track which `open` transitions we've already initialised so toggling
   * the prop doesn't wipe a half-filled name. */
  let primedForOpenCycle = false;

  $effect(() => {
    if (open && !primedForOpenCycle) {
      // Access initialSource inside a closure-accessed expression to avoid
      // the Svelte 5 "captures initial value" lint warning.
      const src = (() => initialSource)();
      source = src;
      name = suggestLocalName(src, get(remoteNames));
      checkout = true;
      submitting = false;
      showSourcePicker = false;
      primedForOpenCycle = true;
    }
    if (!open) {
      primedForOpenCycle = false;
    }
  });

  function sourceLabel(s: InitialSource): string {
    if (s.kind === "head") return "HEAD (current branch)";
    if (s.kind === "commit") return `at commit ${shortOid(s.oid)}`;
    return s.name;
  }

  function pickSource(next: InitialSource) {
    source = next;
    showSourcePicker = false;
  }

  async function handleCreate() {
    const trimmed = name.trim();
    if (!trimmed || submitting) return;
    submitting = true;
    const srcSnapshot = source;
    try {
      await runMutation({
        kind: "branch_create",
        invoke: async () => {
          if (srcSnapshot.kind === "head") {
            await createBranch(trimmed);
          } else if (srcSnapshot.kind === "ref") {
            // Even when the ref IS the current HEAD, createBranchAt with
            // the HEAD oid is correct and keeps one codepath.
            await createBranchAt(trimmed, srcSnapshot.oid);
          } else {
            await createBranchAt(trimmed, srcSnapshot.oid);
          }
          if (checkout) {
            await checkoutBranch(trimmed);
          }
        },
        successToast: () =>
          checkout ? `Created branch ${trimmed} · checked out` : `Created branch ${trimmed}`,
        failureToastPrefix: "Branch create failed",
      });
      onClose();
    } catch {
      // runMutation already surfaced the toast.
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
    else if (e.key === "Enter" && !submitting && name.trim().length > 0) {
      e.preventDefault();
      void handleCreate();
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div class="backdrop" onclick={onClose} onkeydown={handleKeydown} role="button" tabindex="-1"></div>
  <div
    class="dialog"
    data-testid="dialog-create-branch"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label="New branch"
    onkeydown={handleKeydown}
  >
    <h3 class="dialog-title">New branch</h3>

    <label class="field">
      <span class="label">Name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        data-testid="create-branch-name"
        type="text"
        class="input"
        bind:value={name}
        autofocus
        spellcheck="false"
        autocomplete="off"
      />
    </label>

    <div class="field">
      <span class="label">From</span>
      <div class="source-row">
        <button
          class="source-current"
          data-testid="create-branch-source"
          onclick={() => (showSourcePicker = !showSourcePicker)}
          type="button"
        >
          <span>{sourceLabel(source)}</span>
          <span class="chevron nf">{''}</span>
        </button>
        {#if showSourcePicker}
          <div class="source-popover">
            <div class="source-group-label">Local</div>
            {#each $localBranches as b}
              <button
                class="source-option"
                type="button"
                onclick={() => pickSource({ kind: "ref", name: b.name, oid: b.oid })}
              >
                {b.name}
              </button>
            {/each}
            <div class="source-group-label">Remote</div>
            {#each $remoteBranches as b}
              <button
                class="source-option"
                type="button"
                onclick={() => pickSource({ kind: "ref", name: b.name, oid: b.oid })}
              >
                {b.name}
              </button>
            {/each}
            {#if $branches.length === 0}
              <div class="source-empty">No branches yet</div>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <span class="checkbox-row">
      <Checkbox
        id="create-branch-checkout"
        testid="create-branch-checkout"
        checked={checkout}
        onchange={(e) => { checkout = (e.target as HTMLInputElement).checked; }}
      />
      <label for="create-branch-checkout">Check out new branch</label>
    </span>

    <div class="dialog-actions">
      <Button variant="neutral" onclick={onClose}>Cancel</Button>
      <Button
        variant="primary"
        testid="create-branch-submit"
        disabled={submitting || name.trim().length === 0}
        onclick={handleCreate}
      >
        Create
      </Button>
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
    background: rgba(0, 0, 0, 0.5); /* beardgit:allow-hex: modal backdrop neutral */
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
    box-shadow: var(--shadow-modal);
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
    margin-bottom: 12px;
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
    border-color: var(--accent-primary);
  }

  .source-row {
    position: relative;
  }

  .source-current {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    cursor: pointer;
    font-size: 13px;
  }

  .chevron {
    font-size: 9px;
    color: var(--text-secondary);
  }

  .source-popover {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 2px;
    max-height: 220px;
    overflow-y: auto;
    background: var(--bg-toolbar);
    border: 1px solid var(--border);
    border-radius: 4px;
    z-index: 10;
  }

  .source-group-label {
    padding: 6px 10px 2px;
    font-size: 10px;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .source-option {
    display: block;
    width: 100%;
    text-align: left;
    padding: 4px 10px;
    font-size: 12px;
    background: none;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
  }

  .source-option:hover {
    background: color-mix(in srgb, var(--accent-primary) 15%, transparent);
  }

  .source-empty {
    padding: 8px 10px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 16px;
    font-size: 13px;
  }

  .checkbox-row label {
    cursor: pointer;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
