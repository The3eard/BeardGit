<!--
  EditorPane.svelte — wraps `CodeEditor` for the active editor tab.

  Composes the toggleable CodeMirror extensions out of `editorPrefs`,
  wires Mod+S / Mod+Shift+S → save / save-and-stage shortcuts, and
  renders friendly placeholders for binary / too-large files.

  The editor remounts (via `{#key tab.path}`) whenever the active tab
  changes so CodeMirror reloads the correct language pack on file
  switch.
-->
<script lang="ts">
  import {
    autocompletion,
    closeBrackets,
    closeBracketsKeymap,
    completeAnyWord,
    completionKeymap,
  } from "@codemirror/autocomplete";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { bracketMatching, foldGutter, indentOnInput } from "@codemirror/language";
  import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
  import { EditorState, type Extension } from "@codemirror/state";
  import {
    crosshairCursor,
    drawSelection,
    highlightActiveLine,
    keymap,
    rectangularSelection,
  } from "@codemirror/view";
  import CodeEditor from "$lib/components/editor/CodeEditor.svelte";
  import { editorPrefs } from "$lib/stores/editorPrefs";
  import {
    activeTabPath,
    saveActive,
    tabs,
    updateBuffer,
  } from "$lib/stores/fileEditor";
  import { activeTheme } from "$lib/stores/theme";
  import * as m from "$lib/paraglide/messages";

  /** Currently active tab — derived from the store. */
  let active = $derived(
    $tabs.find((t) => t.path === $activeTabPath) ?? null,
  );

  /**
   * Build the CodeMirror extension array from the user's editor
   * preferences. Always-on bits (history, default keymap, save
   * shortcut) live at the top of the array; toggleable features are
   * appended individually so the array key reflects the exact set.
   */
  let extensions = $derived.by<Extension[]>(() => {
    const prefs = $editorPrefs;
    const exts: Extension[] = [
      history(),
      drawSelection(),
      EditorState.allowMultipleSelections.of(true),
      EditorState.tabSize.of(prefs?.tab_size ?? 2),
      keymap.of([
        ...defaultKeymap,
        ...historyKeymap,
        ...searchKeymap,
        ...completionKeymap,
        ...closeBracketsKeymap,
        indentWithTab,
      ]),
      keymap.of([
        {
          key: "Mod-s",
          preventDefault: true,
          run: () => {
            void saveActive();
            return true;
          },
        },
        {
          key: "Mod-Shift-s",
          preventDefault: true,
          run: () => {
            void saveActive({ stage: true });
            return true;
          },
        },
      ]),
    ];
    if (!prefs) return exts;
    if (prefs.autocomplete) {
      // The `autocompletion()` extension is just the popup machinery —
      // suggestions still need a source. Most language packs we ship
      // (`lang-rust`, `lang-python`, `lang-go`, …) don't contribute
      // language data for completions, so without an explicit source
      // the popup never opens. Register `completeAnyWord` as a global
      // language-data entry: it scans the active buffer for words and
      // suggests them, working for *every* language without an LSP.
      // Language-aware sources (HTML tags, CSS properties, etc.) keep
      // working because language data merges — we add to the set, we
      // don't override it.
      exts.push(autocompletion());
      exts.push(EditorState.languageData.of(() => [{ autocomplete: completeAnyWord }]));
    }
    if (prefs.close_brackets) exts.push(closeBrackets());
    if (prefs.bracket_matching) exts.push(bracketMatching());
    if (prefs.highlight_active_line) exts.push(highlightActiveLine());
    if (prefs.highlight_selection_matches) exts.push(highlightSelectionMatches());
    if (prefs.fold_gutter) exts.push(foldGutter());
    if (prefs.indent_on_input) exts.push(indentOnInput());
    if (prefs.rectangular_selection) exts.push(rectangularSelection());
    if (prefs.crosshair_cursor) exts.push(crosshairCursor());
    return exts;
  });

  /** Format a byte count for the too-large placeholder copy. */
  function formatSize(bytes?: number): string {
    if (bytes === undefined) return "";
    if (bytes >= 1024 * 1024) {
      return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }
    if (bytes >= 1024) {
      return `${Math.round(bytes / 1024)} KB`;
    }
    return `${bytes} B`;
  }

  function onChange(content: string) {
    if (active) updateBuffer(active.path, content);
  }
</script>

<div class="editor-pane">
  {#if !active}
    <div class="placeholder">{m.editor_no_tab_open()}</div>
  {:else if active.status === "loading"}
    <div class="placeholder">{m.editor_loading_file()}</div>
  {:else if active.status === "binary"}
    <div class="placeholder">{m.editor_binary_file()}</div>
  {:else if active.status === "too_large"}
    <div class="placeholder">
      {m.editor_too_large({ size: formatSize(active.size) })}
    </div>
  {:else if active.status === "error"}
    <div class="placeholder error">
      {m.editor_load_error({ message: active.error ?? "" })}
    </div>
  {:else}
    <CodeEditor
      content={active.bufferContent}
      filename={active.path}
      editorTheme={$activeTheme?.editor}
      isDark={$activeTheme?.meta.mode !== "light"}
      readonly={false}
      lineWrapping={$editorPrefs?.line_wrapping ?? true}
      revisionId={active.loadVersion}
      {extensions}
      {onChange}
    />
  {/if}
</div>

<style>
  .editor-pane {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
    overflow: hidden;
  }
  .placeholder {
    margin: auto;
    padding: 16px;
    color: var(--text-secondary);
    font-size: 13px;
    text-align: center;
  }
  .placeholder.error {
    color: var(--accent-red);
  }
</style>
