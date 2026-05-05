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
  import { lintGutter } from "@codemirror/lint";
  import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
  import { EditorState, type Extension } from "@codemirror/state";
  import {
    crosshairCursor,
    drawSelection,
    highlightActiveLine,
    highlightActiveLineGutter,
    keymap,
    rectangularSelection,
  } from "@codemirror/view";
  import { colorPicker } from "@replit/codemirror-css-color-picker";
  import { indentationMarkers } from "@replit/codemirror-indentation-markers";
  import CodeEditor from "$lib/components/editor/CodeEditor.svelte";
  import { getLanguageExtensionName } from "$lib/components/editor/language-support";
  import { jsonLinter } from "$lib/components/file-editor/json-lint";
  import { keywordCompletion, keywordsForLanguage } from "$lib/components/file-editor/keywords";
  import { snippetsForLanguage } from "$lib/components/file-editor/snippets";
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
    const filename = active?.path ?? "";
    const langName = getLanguageExtensionName(filename);
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

      // Per-language snippet pack — empty for unmapped languages.
      if (prefs.snippets && langName) {
        const snippets = snippetsForLanguage(langName);
        if (snippets.length > 0) {
          exts.push(EditorState.languageData.of(() => [{ autocomplete: snippets }]));
        }
      }

      // Per-language keyword completion — feeds the language's reserved
      // words into the popup, tagged so the icon renders correctly.
      if (prefs.keyword_completion && langName) {
        const words = keywordsForLanguage(langName);
        if (words.length > 0) {
          const source = keywordCompletion(words);
          exts.push(EditorState.languageData.of(() => [{ autocomplete: source }]));
        }
      }
    }
    if (prefs.close_brackets) exts.push(closeBrackets());
    if (prefs.bracket_matching) exts.push(bracketMatching());
    if (prefs.highlight_active_line) {
      // Companion to highlightActiveLine — also highlights the gutter
      // line number, which the user explicitly wanted bundled with the
      // existing pref rather than introducing a separate toggle.
      exts.push(highlightActiveLine());
      exts.push(highlightActiveLineGutter());
    }
    if (prefs.highlight_selection_matches) exts.push(highlightSelectionMatches());
    if (prefs.fold_gutter) exts.push(foldGutter());
    if (prefs.indent_on_input) exts.push(indentOnInput());
    if (prefs.rectangular_selection) exts.push(rectangularSelection());
    if (prefs.crosshair_cursor) exts.push(crosshairCursor());
    if (prefs.color_picker) exts.push(colorPicker);
    if (prefs.indent_guides) {
      // Replit's indentation markers extension reads its colours from a
      // CSS variable lookup at construction time — it doesn't accept
      // `var(--token)` strings, so we pass neutral grays that read well
      // in both light and dark themes. The escape comment below is for
      // ESLint's `no-hex-in-svelte` rule.
      exts.push(
        indentationMarkers({
          thickness: 1,
          // beardgit:allow-hex: indent-marker tokens — package config doesn't accept CSS vars
          colors: {
            light: "#e5e7eb",
            dark: "#3a3f46",
            activeLight: "#9ca3af",
            activeDark: "#6b7280",
          },
        }),
      );
    }
    if (prefs.json_lint && filename.toLowerCase().endsWith(".json")) {
      exts.push(lintGutter());
      exts.push(jsonLinter(filename));
    }
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
