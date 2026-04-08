<!--
  GitignoreEditor.svelte — Full .gitignore editor for the Settings page.

  Uses CodeMirror via the CodeEditor component for editing with syntax
  awareness. Tracks dirty state and provides Save/Revert controls.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import { readGitignore, writeGitignore } from "$lib/api/tauri";
  import CodeEditor from "../editor/CodeEditor.svelte";
  import { activeTheme } from "$lib/stores/theme";

  let originalContent = $state("");
  let currentContent = $state("");
  let loading = $state(true);
  let saved = $state(false);

  let isDirty = $derived(currentContent !== originalContent);

  async function loadContent() {
    loading = true;
    try {
      const content = await readGitignore();
      originalContent = content;
      currentContent = content;
    } catch {
      originalContent = "";
      currentContent = "";
    }
    loading = false;
  }

  onMount(() => {
    loadContent();
  });

  async function handleSave() {
    await writeGitignore(currentContent);
    originalContent = currentContent;
    saved = true;
    setTimeout(() => { saved = false; }, 2000);
  }

  function handleRevert() {
    currentContent = originalContent;
  }

  function handleChange(newContent: string) {
    currentContent = newContent;
    saved = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      if (isDirty) handleSave();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="gitignore-editor" onkeydown={handleKeydown}>
  <div class="editor-toolbar">
    <span class="editor-title">.gitignore</span>
    <span class="editor-status">
      {#if saved}
        <span class="status-saved">{m.gitignore_editor_saved()}</span>
      {:else if isDirty}
        <span class="status-dirty">{m.gitignore_editor_unsaved()}</span>
      {/if}
    </span>
    <div class="editor-actions">
      <button
        class="btn btn-secondary"
        disabled={!isDirty}
        onclick={handleRevert}
      >
        {m.gitignore_editor_revert()}
      </button>
      <button
        class="btn btn-primary"
        disabled={!isDirty}
        onclick={handleSave}
      >
        {m.gitignore_editor_save()}
      </button>
    </div>
  </div>

  <div class="editor-container">
    {#if loading}
      <div class="empty-state">Loading...</div>
    {:else}
      {#if currentContent === "" && originalContent === ""}
        <p class="hint">{m.gitignore_editor_empty()}</p>
      {/if}
      <CodeEditor
        content={currentContent}
        filename=".gitignore"
        editorTheme={$activeTheme?.editor}
        isDark={$activeTheme?.meta.mode !== 'light'}
        readonly={false}
        onChange={handleChange}
      />
    {/if}
  </div>
</div>

<style>
  .gitignore-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
  }

  .editor-title {
    font-size: 13px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--text-primary);
  }

  .editor-status {
    flex: 1;
    font-size: 12px;
  }

  .status-saved {
    color: var(--accent-green);
  }

  .status-dirty {
    color: var(--accent-orange);
  }

  .editor-actions {
    display: flex;
    gap: 8px;
  }

  .btn {
    padding: 5px 14px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    border: 1px solid var(--border);
    transition: background 0.15s;
  }

  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .btn-secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
  }

  .btn-primary {
    background: var(--accent-blue);
    color: #fff;
    border-color: var(--accent-blue);
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .editor-container {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .hint {
    position: absolute;
    top: 12px;
    left: 16px;
    font-size: 12px;
    color: var(--text-secondary);
    pointer-events: none;
    z-index: 1;
    margin: 0;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    font-size: 13px;
  }
</style>
