<!--
  CodeEditor.svelte — Base CodeMirror 6 wrapper.

  Wraps an `EditorView` with language auto-detection, theme bridging, and
  optional change/selection callbacks.  All heavy setup is async so the UI
  never blocks while language grammars are loaded.
-->
<script lang="ts">
  import { EditorView, lineNumbers, type ViewUpdate } from '@codemirror/view';
  import { EditorState, type Extension } from '@codemirror/state';
  import { createCodemirrorTheme } from './codemirror-theme';
  import { getLanguageExtensionName, loadLanguageExtension } from './language-support';
  import type { ThemeEditorData } from '$lib/types';

  interface Props {
    content: string;
    filename?: string;
    editorTheme?: ThemeEditorData | null;
    isDark?: boolean;
    readonly?: boolean;
    extensions?: Extension[];
    onChange?: (content: string) => void;
    onSelection?: (from: number, to: number) => void;
  }

  let {
    content,
    filename = '',
    editorTheme = null,
    isDark = true,
    readonly = true,
    extensions = [],
    onChange,
    onSelection,
  }: Props = $props();

  let containerEl: HTMLDivElement;
  let view: EditorView | undefined;

  /** Assemble all extensions for the editor state. */
  function buildExtensions(langExt: Extension | null): Extension[] {
    const exts: Extension[] = [
      createCodemirrorTheme(editorTheme, isDark),
      lineNumbers(),
      EditorView.lineWrapping,
      EditorView.updateListener.of((update: ViewUpdate) => {
        if (update.docChanged && onChange) {
          onChange(update.state.doc.toString());
        }
        if (update.selectionSet && onSelection) {
          const sel = update.state.selection.main;
          onSelection(sel.from, sel.to);
        }
      }),
    ];
    if (readonly) {
      exts.push(EditorState.readOnly.of(true), EditorView.editable.of(false));
    }
    if (langExt) exts.push(langExt);
    exts.push(...extensions);
    return exts;
  }

  /** Destroy any existing view and create a fresh one. */
  async function initEditor() {
    if (view) {
      view.destroy();
      view = undefined;
    }
    const langName = getLanguageExtensionName(filename);
    const langExt = langName ? await loadLanguageExtension(langName) : null;

    const state = EditorState.create({
      doc: content,
      extensions: buildExtensions(langExt),
    });

    view = new EditorView({ state, parent: containerEl });
  }

  /** Mount/unmount the editor when the container element is available. */
  $effect(() => {
    if (containerEl) {
      initEditor();
    }
    return () => {
      view?.destroy();
      view = undefined;
    };
  });

  /** Reflect external content changes without recreating the editor. */
  $effect(() => {
    if (view && content !== view.state.doc.toString()) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: content },
      });
    }
  });
</script>

<div class="code-editor" bind:this={containerEl}></div>

<style>
  .code-editor {
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .code-editor :global(.cm-editor) {
    height: 100%;
  }

  .code-editor :global(.cm-scroller) {
    overflow: auto;
    font-family: 'Fira Code', var(--font-mono), monospace;
    font-size: 12px;
    line-height: 1.5;
  }
</style>
