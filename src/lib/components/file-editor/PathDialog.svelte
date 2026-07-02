<!--
  PathDialog.svelte — combined dialog for new-file / new-folder / rename
  flows in the file-editor panel.

  For the new-* modes the parent-directory field is editable (with a
  `<datalist>` of existing directories): the user can type a nested
  relative directory and the backend creates intermediate dirs on demand.
  Validation is intentionally strict — empty/`..`/absolute/Windows-illegal
  leaf names and parent dirs surface inline before the dialog fires its
  `onConfirm` callback — but the backend (`validate_repo_relative_path`)
  remains the authority. Rename keeps its original leaf-only behavior.
-->
<script lang="ts">
  import { Button, Dialog, Field } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";
  import { joinRepoPath, validateDir, validateLeaf } from "./path-validation";

  /**
   * Dialog mode. `new-file` and `new-folder` both prompt for a leaf
   * name placed under `parentDir`. `rename` prefills the leaf name of
   * `targetPath` and produces a new sibling path.
   */
  type Mode = "new-file" | "new-folder" | "rename";

  interface Props {
    /** Whether the dialog is currently visible. */
    open: boolean;
    /** Mode determines title, validation, and the confirm payload. */
    mode: Mode;
    /** Seed parent directory ("" for repo root). Used by the new-* modes. */
    parentDir?: string;
    /** Existing repo-relative path. Used by the rename mode. */
    targetPath?: string;
    /**
     * Existing directory paths for the parent-field autocomplete
     * (new-* modes only). Rendered as a native `<datalist>`.
     */
    existingDirs?: string[];
    /**
     * Confirm callback. For `rename` the argument is the new leaf name;
     * for the new-* modes it is the full repo-relative path (edited
     * parent joined with the leaf). Called only after client-side
     * validation passes.
     */
    onConfirm: (value: string) => void | Promise<void>;
    /** Cancel callback; the parent is expected to flip `open` back to false. */
    onClose: () => void;
  }

  let {
    open,
    mode,
    parentDir = "",
    targetPath = "",
    existingDirs = [],
    onConfirm,
    onClose,
  }: Props = $props();

  /** Pull the leaf name out of a forward-slashed path. */
  function leaf(path: string): string {
    const idx = path.lastIndexOf("/");
    return idx >= 0 ? path.slice(idx + 1) : path;
  }

  /** Localized title — derived from mode + (rename's) target. */
  let title = $derived.by(() => {
    if (mode === "new-file") return m.editor_dialog_new_file_title();
    if (mode === "new-folder") return m.editor_dialog_new_folder_title();
    return m.editor_dialog_rename_title({ name: leaf(targetPath) });
  });

  /** Bound to the leaf `<input>`. Initialised whenever the dialog re-opens. */
  let nameValue = $state("");
  /** Bound to the editable parent-directory `<input>` (new-* modes). */
  let parentValue = $state("");
  let touched = $state(false);
  let parentTouched = $state(false);

  // Reset the fields on every open transition so prior typos don't carry
  // across.  `targetPath`/`mode` change synchronously when the parent
  // hands us a different action, so this also re-prefills rename inputs
  // and re-seeds the editable parent from the tree selection.
  $effect(() => {
    if (open) {
      nameValue = mode === "rename" ? leaf(targetPath) : "";
      parentValue = parentDir;
      touched = false;
      parentTouched = false;
    }
  });

  /** Map a validation code to the (single, friendly) localized message. */
  let nameError = $derived(
    touched && validateLeaf(nameValue) !== null
      ? m.editor_path_invalid()
      : null,
  );
  let parentError = $derived(
    mode !== "rename" && parentTouched && validateDir(parentValue) !== null
      ? m.editor_path_invalid()
      : null,
  );

  /** True when both fields pass validation (ungated by `touched`). */
  let canSubmit = $derived(
    validateLeaf(nameValue) === null &&
      (mode === "rename" || validateDir(parentValue) === null),
  );

  async function submit() {
    touched = true;
    parentTouched = true;
    if (!canSubmit) return;
    const value =
      mode === "rename"
        ? nameValue.trim()
        : joinRepoPath(parentValue, nameValue);
    await onConfirm(value);
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      void submit();
    }
  }
</script>

<Dialog
  bind:open
  {title}
  size="sm"
  onClose={onClose}
>
  <div class="form" onkeydown={onKeyDown} role="presentation">
    {#if mode !== "rename"}
      <Field
        label={m.editor_dialog_parent_label()}
        error={parentError ?? undefined}
      >
        <input
          class="input"
          type="text"
          list="path-dialog-dirs"
          placeholder={m.editor_dialog_parent_placeholder()}
          bind:value={parentValue}
          oninput={() => (parentTouched = true)}
        />
        {#if existingDirs.length > 0}
          <datalist id="path-dialog-dirs">
            {#each existingDirs as dir (dir)}
              <option value={dir}></option>
            {/each}
          </datalist>
        {/if}
      </Field>
    {/if}
    <Field label={m.editor_dialog_name_label()} error={nameError ?? undefined}>
      <input
        class="input"
        type="text"
        bind:value={nameValue}
        oninput={() => (touched = true)}
      />
    </Field>
  </div>
  {#snippet footer()}
    <Button variant="neutral" onclick={onClose}>
      {m.editor_dialog_cancel()}
    </Button>
    <Button
      variant="primary"
      disabled={!canSubmit}
      onclick={() => void submit()}
    >
      {m.editor_dialog_create()}
    </Button>
  {/snippet}
</Dialog>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .input {
    width: 100%;
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-family: var(--font-mono);
    outline: none;
  }
  .input:focus {
    border-color: var(--accent-primary);
  }
</style>
