<!--
  DiffEditor.svelte — Side-by-side diff viewer using @codemirror/merge.

  Renders two read-only editors aligned by changed lines with unchanged
  regions collapsed.  Language detection and theme bridging are shared with
  `CodeEditor` via the same utility modules.
-->
<script lang="ts">
  import { MergeView } from '@codemirror/merge';
  import { EditorView, highlightWhitespace, lineNumbers } from '@codemirror/view';
  import { EditorState, type Extension } from '@codemirror/state';
  import { createCodemirrorTheme } from './codemirror-theme';
  import { getLanguageExtensionName, loadLanguageExtension } from './language-support';
  import type { ThemeEditorData } from '$lib/types';
  import { diffCommentsLayer, type DiffCommentsLayerProps } from './diff-comments-layer';
  import IconButton from '$lib/components/ui/IconButton.svelte';
  import { diffShowWhitespace, diffLineWrapping } from '$lib/stores/diffSettings';

  interface Props {
    oldContent: string;
    newContent: string;
    filename?: string;
    editorTheme?: ThemeEditorData | null;
    isDark?: boolean;
    extensions?: Extension[];
    onClose?: () => void;
    /** When set, render this text instead of the CodeMirror MergeView (e.g. "Binary file — no preview"). */
    placeholder?: string;
    toolbar?: import('svelte').Snippet;
    /** When set, injects the diff-comments-layer into the right-side (new) editor. */
    commentsLayer?: DiffCommentsLayerProps;
  }

  let {
    oldContent,
    newContent,
    filename = '',
    editorTheme = null,
    isDark = true,
    extensions = [],
    onClose,
    placeholder,
    toolbar,
    commentsLayer,
  }: Props = $props();

  let containerEl: HTMLDivElement;
  let mergeView: MergeView | undefined;

  /** Destroy any existing MergeView and create a fresh one. */
  async function initMergeView() {
    if (mergeView) {
      mergeView.destroy();
      mergeView = undefined;
    }

    const langName = getLanguageExtensionName(filename);
    const langExt = langName ? await loadLanguageExtension(langName) : null;

    const theme = createCodemirrorTheme(editorTheme, isDark);
    const sharedExtensions: Extension[] = [
      theme,
      lineNumbers(),
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
    ];
    // Soft-wrap is opt-in via Settings → General → "Wrap long lines in
    // diffs". When off, CodeMirror's default behaviour is horizontal
    // scroll inside the .cm-scroller so the user can still reach the
    // full line.
    if ($diffLineWrapping) {
      sharedExtensions.push(EditorView.lineWrapping);
    }
    // Whitespace glyphs (· / →) — toggled by Settings → General →
    // "Show whitespace in diffs". The view is rebuilt whenever the
    // store flips (see the $effect below) so the change is visible
    // without re-opening the diff.
    if ($diffShowWhitespace) {
      sharedExtensions.push(highlightWhitespace());
    }
    if (langExt) sharedExtensions.push(langExt);
    sharedExtensions.push(...extensions);

    const bExtensions: Extension[] = [...sharedExtensions];
    if (commentsLayer) bExtensions.push(diffCommentsLayer(commentsLayer));

    mergeView = new MergeView({
      a: { doc: oldContent, extensions: sharedExtensions },
      b: { doc: newContent, extensions: bExtensions },
      parent: containerEl,
      collapseUnchanged: { margin: 3, minSize: 4 },
      gutter: true,
    });
  }

  /**
   * Mount/unmount the MergeView when the container element or any
   * content / theme props change.  All reactive deps are read before
   * the early-return so Svelte tracks them correctly.
   */
  $effect(() => {
    // Read reactive deps so the effect re-runs on change.
    const _old = oldContent;
    const _new = newContent;
    const _file = filename;
    const _theme = editorTheme;
    const _dark = isDark;
    const _placeholder = placeholder;
    const _commentsLayer = commentsLayer;
    // Whitespace toggle — re-init the MergeView when it flips so the
    // highlightWhitespace extension is added or removed in place.
    const _whitespace = $diffShowWhitespace;
    // Line-wrapping toggle — same story; rebuild so EditorView.lineWrapping
    // gets added/removed when the user flips the Settings toggle.
    const _wrap = $diffLineWrapping;

    if (!containerEl || placeholder) return;

    initMergeView();

    return () => {
      mergeView?.destroy();
      mergeView = undefined;
    };
  });
</script>

<div class="diff-editor-wrapper">
  {#if toolbar}
    <div class="diff-header">{@render toolbar()}</div>
  {:else if onClose}
    <div class="diff-header">
      <span class="diff-filename">{filename}</span>
      <IconButton tone="default" icon={""} description="Close" onclick={onClose} />
    </div>
  {/if}
  {#if placeholder}
    <div class="diff-placeholder">{placeholder}</div>
  {:else}
    <div class="diff-editor" bind:this={containerEl}></div>
  {/if}
</div>

<style>
  .diff-editor-wrapper {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .diff-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    font-size: var(--font-size-sm);
  }

  .diff-filename {
    font-family: var(--font-mono);
    color: var(--accent-primary);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diff-editor {
    flex: 1;
    overflow: hidden;
  }

  .diff-editor :global(.cm-editor) {
    height: 100%;
  }

  .diff-editor :global(.cm-scroller) {
    overflow: auto;
    font-family: 'Fira Code', var(--font-mono), monospace;
    font-size: var(--font-size-sm);
    line-height: 1.5;
  }

  .diff-editor :global(.cm-mergeView) {
    height: 100%;
  }

  .diff-placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: var(--font-size-md);
    padding: 24px;
    text-align: center;
  }
</style>
