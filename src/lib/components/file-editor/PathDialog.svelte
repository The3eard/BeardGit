<!--
  PathDialog.svelte — combined dialog for new-file / new-folder / rename
  flows in the file-editor panel.

  Validation is intentionally strict: empty names, names containing `..`
  or `/`, and Windows-illegal characters all surface inline before the
  dialog will fire its `onConfirm` callback. This keeps the dialog usable
  cross-platform without falling through to the backend's filesystem
  errors for trivial typos.
-->
<script lang="ts">
  import { Button, Dialog, Field } from "$lib/components/ui";
  import * as m from "$lib/paraglide/messages";

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
    /** Parent directory ("" for repo root). Used by the new-* modes. */
    parentDir?: string;
    /** Existing repo-relative path. Used by the rename mode. */
    targetPath?: string;
    /** Confirm callback; called only after client-side validation passes. */
    onConfirm: (name: string) => void | Promise<void>;
    /** Cancel callback; the parent is expected to flip `open` back to false. */
    onClose: () => void;
  }

  let {
    open,
    mode,
    parentDir = "",
    targetPath = "",
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

  /** Bound to the `<input>`. Initialised whenever the dialog re-opens. */
  let nameValue = $state("");
  let touched = $state(false);

  // Reset the field on every open transition so prior typos don't carry
  // across.  `targetPath`/`mode` change synchronously when the parent
  // hands us a different action, so this also re-prefills rename inputs.
  $effect(() => {
    if (open) {
      nameValue = mode === "rename" ? leaf(targetPath) : "";
      touched = false;
    }
  });

  /** Disallowed character set — Windows reserved + path separator. */
  const INVALID_CHARS = /[<>:"|?*\\/]/;

  /** Returns a localized error string when the input is invalid. */
  function validate(name: string): string | null {
    const trimmed = name.trim();
    if (trimmed === "") return m.editor_path_invalid();
    if (trimmed.startsWith("/") || trimmed.startsWith("..")) {
      return m.editor_path_invalid();
    }
    if (trimmed.split("/").some((part) => part === "..")) {
      return m.editor_path_invalid();
    }
    if (INVALID_CHARS.test(trimmed)) {
      return m.editor_path_invalid();
    }
    return null;
  }

  let validationError = $derived(touched ? validate(nameValue) : null);

  /** Final string passed to the parent — for rename this is the new leaf. */
  function payloadName(): string {
    return nameValue.trim();
  }

  async function submit() {
    touched = true;
    if (validate(nameValue) !== null) return;
    await onConfirm(payloadName());
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      void submit();
    }
  }

  /** Display label for the parent input — empty parent reads as "/". */
  let parentLabel = $derived(parentDir === "" ? "/" : `${parentDir}/`);
</script>

<Dialog
  bind:open
  {title}
  size="sm"
  onClose={onClose}
>
  <div class="form" onkeydown={onKeyDown} role="presentation">
    {#if mode !== "rename"}
      <Field label={m.editor_dialog_parent_label()}>
        <input
          class="input readonly"
          type="text"
          value={parentLabel}
          readonly
          aria-readonly="true"
        />
      </Field>
    {/if}
    <Field label={m.editor_dialog_name_label()} error={validationError ?? undefined}>
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
      disabled={!!validationError || nameValue.trim() === ""}
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
    font-size: 12px;
    font-family: var(--font-mono);
    outline: none;
  }
  .input:focus {
    border-color: var(--accent-primary);
  }
  .input.readonly {
    background: var(--bg-toolbar);
    color: var(--text-secondary);
    cursor: not-allowed;
  }
</style>
