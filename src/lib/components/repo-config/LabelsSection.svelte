<!--
  LabelsSection.svelte — CRUD over the repo's label catalogue.

  Labels live *outside* the `RemoteRepoConfig` patch flow because they
  are a separate CLI surface on both `gh` (`gh label create/edit/delete`)
  and `glab` (`glab label create/delete`). Each row's action goes
  through its own Tauri command and refreshes the local store on
  success — there is no "Save" button for labels.

  Every user-supplied string (name, description, color) is forwarded
  to the backend per-argument; `arg()` on the Rust side keeps the
  invocation shell-injection safe even when a label name contains
  `; echo PWNED`.
-->
<script lang="ts">
  import { Button, Card, Dialog, Field, FormRow } from "$lib/components/ui";
  import { repoConfigStore, updateCurrent } from "$lib/stores/repoConfig";
  import type { RepoConfigLabel } from "$lib/types/repoConfig";
  import {
    createRepoLabel,
    updateRepoLabel,
    deleteRepoLabel,
  } from "$lib/api/tauri";
  import { addToast } from "$lib/stores/toast";

  let current = $derived($repoConfigStore.current);
  let repoPath = $derived($repoConfigStore.repoPath);

  // Editor state — one dialog handles both Add + Edit via the `mode`
  // discriminator. `originalName` tracks the pre-edit name so rename
  // round-trips work on both backends.
  let editorOpen = $state(false);
  let editorMode = $state<"add" | "edit">("add");
  let editorOriginalName = $state<string>("");
  let draft = $state<RepoConfigLabel>(emptyLabel());
  let saving = $state(false);
  let editorError = $state<string | null>(null);

  // Delete confirmation state.
  let deleteConfirmOpen = $state(false);
  let deleteTarget = $state<string | null>(null);

  function emptyLabel(): RepoConfigLabel {
    return { name: "", color: "cccccc", description: "" };
  }

  function openAdd() {
    editorMode = "add";
    editorOriginalName = "";
    draft = emptyLabel();
    editorError = null;
    editorOpen = true;
  }

  function openEdit(label: RepoConfigLabel) {
    editorMode = "edit";
    editorOriginalName = label.name;
    draft = {
      name: label.name,
      color: label.color ?? "cccccc",
      description: label.description ?? "",
    };
    editorError = null;
    editorOpen = true;
  }

  function closeEditor() {
    editorOpen = false;
  }

  async function saveLabel() {
    if (!repoPath) return;
    if (draft.name.trim().length === 0) {
      editorError = "Label name cannot be empty.";
      return;
    }
    saving = true;
    editorError = null;
    try {
      if (editorMode === "add") {
        await createRepoLabel(repoPath, draft);
        updateCurrent((d) => {
          d.labels = [...d.labels, { ...draft }];
        });
        addToast({ message: `Label ${draft.name} created`, type: "success" });
      } else {
        await updateRepoLabel(repoPath, editorOriginalName, draft);
        updateCurrent((d) => {
          d.labels = d.labels.map((l) =>
            l.name === editorOriginalName ? { ...draft } : l,
          );
        });
        addToast({ message: `Label ${draft.name} updated`, type: "success" });
      }
      editorOpen = false;
    } catch (e) {
      editorError = String(e);
      addToast({ message: String(e), type: "error" });
    } finally {
      saving = false;
    }
  }

  function requestDelete(name: string) {
    deleteTarget = name;
    deleteConfirmOpen = true;
  }

  async function confirmDelete() {
    if (!repoPath || !deleteTarget) return;
    const name = deleteTarget;
    try {
      await deleteRepoLabel(repoPath, name);
      updateCurrent((d) => {
        d.labels = d.labels.filter((l) => l.name !== name);
      });
      addToast({ message: `Label ${name} deleted`, type: "success" });
    } catch (e) {
      addToast({ message: String(e), type: "error" });
    } finally {
      deleteConfirmOpen = false;
      deleteTarget = null;
    }
  }

  function cancelDelete() {
    deleteConfirmOpen = false;
    deleteTarget = null;
  }

  /** Normalise a hex color input — strip `#`, lowercase, trim. */
  function normaliseColor(raw: string): string {
    return raw.replace(/^#/, "").trim().toLowerCase();
  }
</script>

<div class="repo-config-labels" data-testid="repo-config-labels">
  {#if current}
    <Card
      title="Labels"
      description="Used to categorise issues and pull/merge requests."
    >
      {#snippet actions()}
        <span data-testid="repo-config-label-add-wrap">
          <Button
            variant="primary"
            icon={"\uF067"}
            onclick={openAdd}
          >
            Add label
          </Button>
        </span>
      {/snippet}

      <div class="label-list">
        {#if current.labels.length === 0}
          <p class="empty">No labels yet. Add one to start categorising.</p>
        {:else}
          {#each current.labels as label (label.name)}
            <div
              class="label-row"
              data-testid={`repo-config-label-row-${label.name}`}
            >
              <span
                class="swatch"
                style={`background: #${label.color ?? "cccccc"}`}
                aria-hidden="true"
              ></span>
              <span class="label-name">{label.name}</span>
              <span class="label-desc">{label.description ?? ""}</span>
              <div class="label-actions">
                <Button
                  variant="ghost"
                  size="sm"
                  icon={"\uF044"}
                  onclick={() => openEdit(label)}
                >
                  Edit
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  icon={"\uF2ED"}
                  onclick={() => requestDelete(label.name)}
                >
                  Delete
                </Button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </Card>
  {/if}
</div>

<Dialog
  bind:open={editorOpen}
  title={editorMode === "add" ? "Add label" : "Edit label"}
  size="sm"
>
  <div class="editor">
    <Field label="Name" for="label-name">
      <input
        id="label-name"
        type="text"
        class="bg-input"
        bind:value={draft.name}
        data-testid="repo-config-label-name"
      />
    </Field>
    <Field label="Description" for="label-description">
      <input
        id="label-description"
        type="text"
        class="bg-input"
        value={draft.description ?? ""}
        oninput={(e) =>
          (draft = { ...draft, description: (e.target as HTMLInputElement).value })}
        data-testid="repo-config-label-description"
      />
    </Field>
    <Field label="Color (hex)" for="label-color">
      <div class="color-row">
        <span
          class="swatch large"
          style={`background: #${draft.color ?? "cccccc"}`}
          aria-hidden="true"
        ></span>
        <input
          id="label-color"
          type="text"
          class="bg-input"
          placeholder="ff0000"
          value={draft.color ?? ""}
          oninput={(e) =>
            (draft = {
              ...draft,
              color: normaliseColor((e.target as HTMLInputElement).value),
            })}
          data-testid="repo-config-label-color"
        />
      </div>
    </Field>
    {#if editorError}
      <p class="error" role="alert" data-testid="repo-config-label-error">
        {editorError}
      </p>
    {/if}
  </div>
  {#snippet footer()}
    <Button onclick={closeEditor}>Cancel</Button>
    <span data-testid="repo-config-label-save-wrap">
      <Button variant="primary" loading={saving} onclick={saveLabel}>
        Save
      </Button>
    </span>
  {/snippet}
</Dialog>

<Dialog
  bind:open={deleteConfirmOpen}
  title="Delete label?"
  size="sm"
>
  <p data-testid="repo-config-label-delete-message">
    Deleting the label <strong>{deleteTarget}</strong> removes it from every
    issue and pull/merge request on the forge. This cannot be undone.
  </p>
  {#snippet footer()}
    <Button onclick={cancelDelete}>Cancel</Button>
    <Button variant="danger" onclick={confirmDelete}>Delete</Button>
  {/snippet}
</Dialog>

<style>
  .repo-config-labels {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .label-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .label-row {
    display: grid;
    grid-template-columns: 20px 160px 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .swatch {
    display: inline-block;
    width: 14px;
    height: 14px;
    border-radius: 7px;
    border: 1px solid rgba(0, 0, 0, 0.2);
  }

  .swatch.large {
    width: 24px;
    height: 24px;
    border-radius: 4px;
  }

  .label-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .label-desc {
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .label-actions {
    display: flex;
    gap: 4px;
  }

  .empty {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0;
    padding: 8px 0;
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .color-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .bg-input {
    flex: 1;
    padding: 6px 10px;
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: inherit;
    font-size: 12px;
  }

  .bg-input:focus {
    outline: none;
    border-color: var(--accent-blue);
  }

  .error {
    font-size: 11px;
    color: var(--accent-red);
    margin: 0;
  }
</style>
