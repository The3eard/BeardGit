<!--
  MergeEditor.svelte — 3-way merge conflict resolution using CodeMirror's
  unifiedMergeView.

  Shows the "ours" content as the editable document with the "base" version
  as the original reference.  Inline accept/reject controls let the user
  resolve each changed chunk.  The "Mark Resolved" button writes the final
  editor content back via the onResolve callback.
-->
<script lang="ts">
  import { EditorView, lineNumbers } from '@codemirror/view';
  import { EditorState } from '@codemirror/state';
  import { unifiedMergeView, goToNextChunk, goToPreviousChunk } from '@codemirror/merge';
  import { createCodemirrorTheme } from './codemirror-theme';
  import { getLanguageExtensionName, loadLanguageExtension } from './language-support';
  import type { ThemeEditorData } from '$lib/types';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    /** Content from the current branch ("ours"). */
    ours: string;
    /** Content from the incoming branch ("theirs"). */
    theirs: string;
    /** Content from the common ancestor ("base"). */
    base: string;
    /** Filename used for language detection and display. */
    filename: string;
    /** CodeMirror theme data from the TOML theme system. */
    editorTheme?: ThemeEditorData | null;
    /** Whether the UI is in dark mode. */
    isDark?: boolean;
    /** Called with the resolved file content when the user clicks "Mark Resolved". */
    onResolve?: (content: string) => void;
    /** Called when the user cancels conflict resolution. */
    onCancel?: () => void;
  }

  let {
    ours,
    theirs,
    base,
    filename,
    editorTheme = null,
    isDark = true,
    onResolve,
    onCancel,
  }: Props = $props();

  let containerEl: HTMLDivElement;
  let view: EditorView | undefined;

  /** Destroy any existing editor and create a fresh one with unified merge view. */
  async function initEditor() {
    if (view) {
      view.destroy();
      view = undefined;
    }

    const langName = getLanguageExtensionName(filename);
    const langExt = langName ? await loadLanguageExtension(langName) : null;
    const theme = createCodemirrorTheme(editorTheme, isDark);

    const extensions = [
      theme,
      lineNumbers(),
      unifiedMergeView({
        original: base,
        mergeControls: true,
        highlightChanges: true,
        gutter: true,
        collapseUnchanged: { margin: 3, minSize: 4 },
      }),
      EditorView.lineWrapping,
    ];
    if (langExt) extensions.push(langExt);

    // Start with "ours" content — the user edits to resolve conflicts.
    const state = EditorState.create({ doc: ours, extensions });
    view = new EditorView({ state, parent: containerEl });
  }

  /** Save the current editor content via the onResolve callback. */
  function handleResolve() {
    if (view && onResolve) {
      onResolve(view.state.doc.toString());
    }
  }

  /** Move the cursor to the next changed chunk. */
  function handleNextChunk() {
    if (view) goToNextChunk(view);
  }

  /** Move the cursor to the previous changed chunk. */
  function handlePrevChunk() {
    if (view) goToPreviousChunk(view);
  }

  /**
   * Mount/unmount the editor when the container element or any
   * content / theme props change.
   */
  $effect(() => {
    // Read reactive deps so the effect re-runs on change.
    const _ours = ours;
    const _theirs = theirs;
    const _base = base;
    const _file = filename;
    const _theme = editorTheme;
    const _dark = isDark;

    if (!containerEl) return;

    initEditor();

    return () => {
      view?.destroy();
      view = undefined;
    };
  });
</script>

<div class="merge-editor-wrapper">
  <div class="merge-toolbar">
    <span class="merge-filename">{filename}</span>
    <div class="merge-actions">
      <button class="merge-btn nav" onclick={handlePrevChunk} title="Previous conflict">{"\uF062"}</button>
      <button class="merge-btn nav" onclick={handleNextChunk} title="Next conflict">{"\uF063"}</button>
      <button class="merge-btn resolve" onclick={handleResolve}>{m.merge_mark_resolved()}</button>
      {#if onCancel}
        <button class="merge-btn cancel" onclick={onCancel}>{m.merge_cancel()}</button>
      {/if}
    </div>
  </div>
  <div class="merge-content" bind:this={containerEl}></div>
</div>

<style>
  .merge-editor-wrapper {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .merge-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    gap: 8px;
  }

  .merge-filename {
    font-family: var(--font-mono);
    color: var(--accent-blue);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .merge-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .merge-btn {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 11px;
    padding: 3px 10px;
    border-radius: 4px;
    white-space: nowrap;
  }

  .merge-btn:hover {
    background: var(--selection);
  }

  .merge-btn.nav {
    font-family: var(--font-icons);
    font-size: 11px;
    padding: 3px 6px;
  }

  .merge-btn.resolve {
    background: var(--accent-green);
    color: var(--bg-primary);
    border-color: var(--accent-green);
    font-weight: 600;
  }

  .merge-btn.resolve:hover {
    opacity: 0.9;
  }

  .merge-btn.cancel {
    color: var(--text-secondary);
  }

  .merge-content {
    flex: 1;
    overflow: hidden;
  }

  .merge-content :global(.cm-editor) {
    height: 100%;
  }

  .merge-content :global(.cm-scroller) {
    overflow: auto;
    font-family: 'Fira Code', var(--font-mono), monospace;
    font-size: 12px;
    line-height: 1.5;
  }
</style>
