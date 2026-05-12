<!--
  VisibilitySection.svelte — public/private/internal radio group +
  archive toggle.

  Visibility changes are applied at Save time, not on click — the user
  can freely flip between radios without committing. The *archive*
  toggle is destructive (archiving locks the repo, unarchiving
  restores it), so this component guards it behind an inline
  `<Dialog>`-based confirmation *before* the store is mutated. Rolling
  the change back from the Save button is not enough: users expect a
  deliberate confirmation step before flipping something that visibly
  changes the remote repo's identity.

  Internal is offered as a selectable option even on personal accounts
  — the forge will reject the apply call with a clear error, which the
  Save flow surfaces as a per-field failure.
-->
<script lang="ts">
  import { Button, Dialog, Field, FormRow } from "$lib/components/ui";
  import { repoConfigStore, updateCurrent } from "$lib/stores/repoConfig";
  import type { Visibility } from "$lib/types/repoConfig";

  let current = $derived($repoConfigStore.current);
  let archiveConfirmOpen = $state(false);

  const visibilityOptions: { value: Visibility; label: string; description: string }[] = [
    { value: "public", label: "Public", description: "Anyone on the internet can see this repository." },
    { value: "private", label: "Private", description: "Only people you give explicit access can view." },
    { value: "internal", label: "Internal", description: "Visible to everyone in the organisation (org accounts only)." },
  ];

  function selectVisibility(next: Visibility) {
    updateCurrent((draft) => {
      draft.visibility = next;
    });
  }

  function requestArchiveToggle() {
    // Always confirm — both archive *and* unarchive are destructive
    // in the sense that they visibly change the repo state.
    archiveConfirmOpen = true;
  }

  function confirmArchive() {
    updateCurrent((draft) => {
      draft.archived = !draft.archived;
    });
    archiveConfirmOpen = false;
  }

  function cancelArchive() {
    archiveConfirmOpen = false;
  }
</script>

<div class="repo-config-visibility" data-testid="repo-config-visibility">
  {#if current}
    <Field label="Visibility" description="Who can see this repository.">
      <div class="visibility-list" role="radiogroup" aria-label="Visibility">
        {#each visibilityOptions as option (option.value)}
          <label
            class="visibility-option"
            class:selected={current.visibility === option.value}
          >
            <input
              type="radio"
              name="repo-config-visibility"
              value={option.value}
              checked={current.visibility === option.value}
              onchange={() => selectVisibility(option.value)}
              data-testid={`repo-config-visibility-${option.value}`}
            />
            <span class="visibility-text">
              <span class="visibility-label">{option.label}</span>
              <span class="visibility-description">{option.description}</span>
            </span>
          </label>
        {/each}
      </div>
    </Field>

    <FormRow
      label={current.archived ? "Archived" : "Active"}
      helperText={current.archived
        ? "This repository is archived. Unarchiving restores write access."
        : "Archiving marks the repository read-only. You can unarchive later."}
    >
      <Button
        variant={current.archived ? "neutral" : "danger"}
        onclick={requestArchiveToggle}
      >
        {current.archived ? "Unarchive" : "Archive"}
      </Button>
    </FormRow>
  {/if}
</div>

<Dialog
  bind:open={archiveConfirmOpen}
  title={current?.archived ? "Unarchive repository?" : "Archive repository?"}
  size="sm"
>
  <p data-testid="repo-config-archive-confirm-message">
    {#if current?.archived}
      Unarchiving restores write access on the forge. Collaborators will
      be able to push and open issues again.
    {:else}
      Archiving marks the repository read-only on the forge. Nobody will
      be able to push, open issues, or merge pull requests until it is
      unarchived.
    {/if}
  </p>
  {#snippet footer()}
    <Button onclick={cancelArchive}>Cancel</Button>
    <Button
      variant={current?.archived ? "primary" : "danger"}
      onclick={confirmArchive}
    >
      {current?.archived ? "Unarchive" : "Archive"}
    </Button>
  {/snippet}
</Dialog>

<style>
  .repo-config-visibility {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .visibility-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .visibility-option {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    transition: border-color 0.12s ease, background 0.12s ease;
  }

  .visibility-option:hover {
    background: var(--overlay-hover);
  }

  .visibility-option.selected {
    border-color: var(--accent-primary);
    background: var(--overlay-accent-blue);
  }

  .visibility-option input[type="radio"] {
    margin-top: 3px;
  }

  .visibility-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .visibility-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .visibility-description {
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.45;
  }
</style>
