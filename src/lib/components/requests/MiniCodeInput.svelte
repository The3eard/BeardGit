<!--
  MiniCodeInput.svelte — Single-line CodeMirror text input.

  Drop-in replacement for `<input class="bg-input">` when the field
  benefits from CodeMirror behaviour (autocomplete, theming, future
  syntax highlights for `{{var}}` mustaches, etc.). Designed to look
  identical to the canonical `.bg-input` so it can sit alongside
  native inputs in the same row without visual drift — the host
  `<div>` wears the same border, padding, height, and focus ring as
  `.bg-input`, and `:global(.cm-editor)` is flattened so CodeMirror's
  own chrome doesn't add a second frame.

  API:
   - `value` (two-way bound string) — current text.
   - `placeholder` (string) — shown via a tiny CodeMirror `Decoration`
     at offset 0 when the doc is empty.
   - `extraExtensions` (Extension[]) — extra CodeMirror extensions
     consumers can add (autocomplete sources, update listeners, etc.).
   - `class` — passthrough class list applied to the host div.

  Behaviour:
   - Single line: pressing Enter does nothing. Newlines pasted in are
     stripped via a transaction filter so the doc never grows past
     one line.
   - History (`Ctrl/Cmd+Z`) and the default keymap are wired so basic
     editing feels native.
   - Theme is the canonical app theme via `createCodemirrorTheme`;
     reconfigured live when the user switches light/dark.
   - Default font is `var(--font-mono)` so URLs and `{{vars}}` read
     in mono and align with the method dropdown next to it.

  Focus ring: `:focus-within` on the host div paints the same blue
  border as `.bg-input:focus`. The internal `.cm-editor` doesn't show
  its own outline (cleared via `:global`).
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    EditorView,
    keymap,
    Decoration,
    ViewPlugin,
    WidgetType,
    tooltips,
  } from "@codemirror/view";
  import type { DecorationSet, ViewUpdate } from "@codemirror/view";
  import {
    EditorState,
    Compartment,
    type Extension,
    type Transaction,
  } from "@codemirror/state";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { activeTheme } from "$lib/stores/theme";
  import { createCodemirrorTheme } from "$lib/components/editor/codemirror-theme";
  import { get } from "svelte/store";

  /**
   * Two-way bound text value. Mirrors `<input bind:value>` semantics —
   * external assignments update the editor doc, in-editor edits update
   * this prop via `bind:`.
   */
  export let value: string = "";
  /** Placeholder shown as inline decoration when the doc is empty. */
  export let placeholder: string = "";
  /** Extra extensions appended to the base config (autocomplete, etc.). */
  export let extraExtensions: Extension[] = [];
  /** Passthrough class list for the host `<div>`. */
  let className: string = "";
  export { className as class };

  /** DOM node that hosts the `EditorView`. */
  let host: HTMLDivElement;
  /** Active CodeMirror view; null until `onMount` runs. */
  let view: EditorView | null = null;
  /**
   * Set true while we mirror an external `value` change into the
   * editor so the resulting `updateListener` callback doesn't echo
   * the string back into `value` (which Svelte already holds).
   */
  let suppressUpdate = false;

  /** Compartment so theme reconfigures live without rebuilding the view. */
  const themeCompartment = new Compartment();

  /** Build the CodeMirror theme extension from the active app theme. */
  function buildThemeExt() {
    const theme = get(activeTheme);
    return createCodemirrorTheme(
      theme?.editor ?? null,
      theme?.meta.mode !== "light",
    );
  }

  /**
   * Widget that renders a placeholder string at the start of an
   * empty doc. We render a regular `span.cm-placeholder` and let the
   * stylesheet pick the muted color, matching `.bg-input::placeholder`.
   */
  class PlaceholderWidget extends WidgetType {
    text: string;
    constructor(text: string) {
      super();
      this.text = text;
    }
    eq(other: PlaceholderWidget) {
      return other.text === this.text;
    }
    toDOM(): HTMLElement {
      const span = document.createElement("span");
      span.className = "cm-placeholder";
      span.textContent = this.text;
      span.setAttribute("aria-hidden", "true");
      return span;
    }
  }

  /**
   * Inline placeholder plugin. Renders one `Decoration.widget` before
   * position 0 whenever the doc is empty. Reactive on the prop via
   * the closure over `getText()` so updating `placeholder` re-renders
   * without rebuilding the plugin.
   */
  function placeholderPlugin(getText: () => string) {
    return ViewPlugin.fromClass(
      class {
        decorations: DecorationSet;
        constructor(view: EditorView) {
          this.decorations = this.build(view);
        }
        update(u: ViewUpdate) {
          if (u.docChanged || u.viewportChanged) {
            this.decorations = this.build(u.view);
          }
        }
        build(view: EditorView): DecorationSet {
          const txt = getText();
          if (view.state.doc.length > 0 || !txt) {
            return Decoration.none;
          }
          const widget = Decoration.widget({
            widget: new PlaceholderWidget(txt),
            side: 1,
          });
          return Decoration.set([widget.range(0)]);
        }
      },
      { decorations: (v) => v.decorations },
    );
  }

  /**
   * Transaction filter that drops any change inserting a newline,
   * keeping the doc strictly single-line. Pastes that contain a
   * newline are rejected wholesale (the user can re-paste a cleaned
   * value). This is simpler and safer than rewriting the changeset
   * in-place, and the case is rare enough that the dropped paste is
   * an acceptable edge.
   */
  function singleLineFilter(tr: Transaction): Transaction | readonly Transaction[] {
    if (!tr.docChanged) return tr;
    let hadNewline = false;
    tr.changes.iterChanges((_fa, _ta, _fb, _tb, inserted) => {
      if (inserted.toString().includes("\n")) hadNewline = true;
    });
    return hadNewline ? [] : tr;
  }

  onMount(() => {
    view = new EditorView({
      state: EditorState.create({
        doc: value,
        extensions: [
          themeCompartment.of(buildThemeExt()),
          // Render tooltips (autocomplete popover, etc.) attached to
          // `document.body` with `position: fixed` so they escape any
          // `overflow: hidden` ancestor and never get clipped behind
          // the tabs / response panes below the URL bar.
          tooltips({ position: "fixed", parent: document.body }),
          history(),
          keymap.of([...defaultKeymap, ...historyKeymap]),
          // Single-line: swallow Enter so the doc never gets a
          // newline from keyboard input. Pasted newlines are
          // rejected by `singleLineFilter` below.
          EditorView.domEventHandlers({
            keydown(e) {
              if (e.key === "Enter") {
                e.preventDefault();
                return true;
              }
              return false;
            },
          }),
          EditorState.transactionFilter.of(singleLineFilter),
          placeholderPlugin(() => placeholder),
          EditorView.updateListener.of((u) => {
            if (suppressUpdate || !u.docChanged) return;
            value = u.state.doc.toString();
          }),
          ...extraExtensions,
        ],
      }),
      parent: host,
    });
  });

  onDestroy(() => view?.destroy());

  // Mirror external `value` writes into the editor doc without
  // bouncing the resulting update back through the listener.
  $: if (view) {
    const cur = view.state.doc.toString();
    if (cur !== (value ?? "")) {
      suppressUpdate = true;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: value ?? "" },
      });
      suppressUpdate = false;
    }
  }

  // Re-thread the theme whenever the active theme store changes.
  $: if (view && $activeTheme !== undefined) {
    view.dispatch({
      effects: themeCompartment.reconfigure(buildThemeExt()),
    });
  }
</script>

<div class={`mini-host ${className}`} bind:this={host}></div>

<style>
  /*
   * Match the visuals of `.bg-input` from `UrlBar.svelte` so the host
   * div sits flush with native inputs in the same row. We replicate
   * the recipe inline because Svelte's scoped CSS means the parent's
   * `.bg-input` selector wouldn't reach this host div anyway.
   */
  .mini-host {
    display: block;
    height: 30px;
    padding: 0;
    overflow: hidden;
    box-sizing: border-box;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    line-height: 28px; /* 30px - 2px borders */
  }

  /*
   * `:focus-within` matches `.bg-input:focus` from UrlBar so the
   * border picks up `--accent-blue` whenever the inner CodeMirror
   * editor (or any descendant) is focused. Documented as a slight
   * deviation from `.bg-input:focus` — the actual focused element is
   * the `.cm-content`, not the host div, so plain `:focus` wouldn't
   * fire.
   */
  .mini-host:focus-within {
    border-color: var(--accent-primary);
  }

  /*
   * Flatten CodeMirror's own chrome so it inherits the host's frame
   * instead of painting its own. The editor sits at full height of
   * the 30px host with the same horizontal padding the native
   * `.bg-input` uses (10px), and the font size matches.
   */
  .mini-host :global(.cm-editor) {
    height: 100%;
    background: transparent;
    outline: none;
  }
  .mini-host :global(.cm-editor.cm-focused) {
    outline: none;
  }
  .mini-host :global(.cm-scroller) {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    line-height: 28px; /* 30px - 2px borders */
  }
  .mini-host :global(.cm-content) {
    padding: 0 10px;
    caret-color: var(--accent-primary);
  }
  .mini-host :global(.cm-line) {
    padding: 0;
  }
  /* Placeholder color: muted secondary text, matching the
     `::placeholder` styling the native `.bg-input` would use. */
  .mini-host :global(.cm-placeholder) {
    color: var(--text-secondary);
    opacity: 0.7;
    pointer-events: none;
  }
</style>
