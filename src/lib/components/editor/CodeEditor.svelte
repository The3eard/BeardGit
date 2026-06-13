<!--
  CodeEditor.svelte — Base CodeMirror 6 wrapper.

  Wraps an `EditorView` with language auto-detection, theme bridging, and
  optional change/selection callbacks.  All heavy setup is async so the UI
  never blocks while language grammars are loaded.
-->
<script lang="ts">
  import { EditorView, lineNumbers, type ViewUpdate } from '@codemirror/view';
  import { EditorState, type Extension } from '@codemirror/state';
  import { onMount, onDestroy, untrack } from 'svelte';
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
    /**
     * Soft-wrap long lines. Default `true` to match every existing
     * caller (diff views, conflict resolver, blame). Pass `false` to
     * gate wrapping behind a user preference (the file-editor panel
     * routes the `line_wrapping` editor pref through this prop so the
     * setting can be respected without forking the wrapper).
     */
    lineWrapping?: boolean;
    /**
     * Bump this number to force the editor to swallow the latest
     * `content` value into a fresh `EditorState`. Used by the
     * file-editor panel on file load + reload — the parent owns
     * exactly when "external" content swaps happen, so the editor
     * never has to guess from prop changes whether a new `content`
     * value is the user's typing round-tripping or a real overwrite.
     * Leave unset (or constant) when callers never need explicit
     * resets — diff / merge views fall in that bucket.
     */
    revisionId?: number;
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
    lineWrapping = true,
    revisionId = 0,
    onChange,
    onSelection,
  }: Props = $props();

  let containerEl = $state<HTMLDivElement | undefined>();
  let view: EditorView | undefined;

  /**
   * Snapshot of the props used by the most recent init. We diff against
   * these on every reactive run to decide whether a remount is genuinely
   * required, instead of letting Svelte's effect dependency-tracking call
   * the shots — empirically it kept re-firing on prop *identity* changes
   * (e.g. the `extensions` array reference rebuilds), which tore the view
   * down mid-keystroke and dropped focus.
   */
  let mountedFilename: string | undefined;
  let mountedRevisionId: number | undefined;
  let mountedTheme: ThemeEditorData | null | undefined;
  let mountedIsDark: boolean | undefined;

  /** Assemble all extensions for the editor state. */
  function buildExtensions(langExt: Extension | null): Extension[] {
    const exts: Extension[] = [
      createCodemirrorTheme(editorTheme, isDark),
      lineNumbers(),
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
    if (lineWrapping) {
      exts.push(EditorView.lineWrapping);
    }
    if (readonly) {
      exts.push(EditorState.readOnly.of(true), EditorView.editable.of(false));
    }
    if (langExt) exts.push(langExt);
    exts.push(...extensions);
    return exts;
  }

  /**
   * Destroy any existing view and create a fresh one. Reads of every
   * prop are wrapped in `untrack` so the surrounding `$effect` only
   * sees the keys it explicitly samples (`filename`, `revisionId`).
   */
  async function initEditor() {
    if (view) {
      view.destroy();
      view = undefined;
    }
    const target = containerEl;
    if (!target) return;
    const fname = untrack(() => filename);
    const langName = getLanguageExtensionName(fname);
    const langExt = langName ? await loadLanguageExtension(langName) : null;
    if (containerEl !== target) {
      // The container was swapped out (or removed) while we awaited the
      // language pack. Bail out — the next mount cycle will rebuild.
      return;
    }
    const state = EditorState.create({
      doc: untrack(() => content),
      extensions: untrack(() => buildExtensions(langExt)),
    });
    view = new EditorView({ state, parent: target });
    mountedFilename = fname;
    mountedRevisionId = untrack(() => revisionId);
    mountedTheme = untrack(() => editorTheme);
    mountedIsDark = untrack(() => isDark);
  }

  // Initial mount + tear-down. Strict lifecycle, no reactivity tracking
  // — `onMount` runs exactly once after the DOM is ready.
  onMount(() => {
    void initEditor();
  });

  onDestroy(() => {
    view?.destroy();
    view = undefined;
  });

  /**
   * Re-init the view when the user-controlled keys actually change.
   * Tracked deps: `filename`, `revisionId`, `editorTheme`, `isDark`.
   * Everything else (extensions, content, callbacks) is read via
   * `untrack` inside `initEditor`. The diff-against-snapshot guard
   * skips the no-op runs — Svelte's `$effect` will fire any time the
   * parent re-renders us with the same props, so we compare values
   * ourselves before destroying the live view. Theme + dark-mode are
   * in the dep set so that an OS-driven auto theme flip (or a manual
   * theme change) actually reskins the editor; the surrounding panel
   * passes `\$activeTheme?.editor` and `\$activeTheme?.meta.mode !==
   * "light"`, both stable references across keystrokes.
   */
  $effect(() => {
    const f = filename;
    const r = revisionId;
    const t = editorTheme;
    const d = isDark;
    if (!view) return; // first mount handled by onMount
    if (
      f === mountedFilename &&
      r === mountedRevisionId &&
      t === mountedTheme &&
      d === mountedIsDark
    ) {
      return;
    }
    void untrack(() => initEditor());
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
    font-size: var(--font-size-sm);
    line-height: 1.5;
  }
</style>
