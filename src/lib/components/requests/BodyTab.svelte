<!--
  BodyTab.svelte — CodeMirror-backed editor for the request body.

  Mounts a single `EditorView` with JSON syntax highlighting (most
  request bodies in this app are JSON). Edits are pushed back into
  `currentRequest.body`, and external updates to the doc (e.g. after
  `requests_load` repopulates the form when the user picks a different
  file) are mirrored into the editor via a guarded dispatch so we don't
  reflect our own changes back at ourselves.

  We intentionally avoid the umbrella `codemirror` package's
  `basicSetup`; the rest of the codebase composes extensions piecewise
  from `@codemirror/view` + `@codemirror/state`, which keeps the bundle
  smaller and matches the existing CodeEditor wrapper.

  Theme: pulls the active TOML theme from `activeTheme` and feeds it
  through the canonical `createCodemirrorTheme()` bridge (the same
  helper `MergeEditor` and `CodeEditor` use). The theme is wrapped in
  a `Compartment` so we can `reconfigure()` it whenever the user
  switches light/dark without recreating the view. The surrounding
  `.body-shell` keeps its `--bg-secondary` frame for the rounded
  border, but its inner background is `transparent` so the CodeMirror
  theme owns the editor surface — no more white gutter/margin column
  in dark mode.

  Variable autocomplete: `varCompletion()` provides `{{var}}` /
  `{{secret}}` / `{{forge_*}}` suggestions when the cursor is inside
  an unclosed mustache. CodeMirror's `activateOnTyping` doesn't fire on
  the non-word character `{`, so we add an `EditorView.updateListener`
  that watches for the just-typed two-char sequence `{{` and calls
  `startCompletion` explicitly to open the popover. The same source is
  also wired into the URL bar's `MiniCodeInput` — the user wanted it
  in both places.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, lineNumbers, keymap, tooltips } from "@codemirror/view";
  import { EditorState, Compartment } from "@codemirror/state";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { json } from "@codemirror/lang-json";
  import {
    autocompletion,
    completionKeymap,
    startCompletion,
  } from "@codemirror/autocomplete";
  import { currentRequest, currentEnv } from "./stores";
  import { activeProject } from "$lib/stores/projects";
  import { activeTheme } from "$lib/stores/theme";
  import { createCodemirrorTheme } from "$lib/components/editor/codemirror-theme";
  import { varCompletion } from "./varCompletion";
  import { get } from "svelte/store";

  /** DOM container that hosts the `EditorView`. */
  let host: HTMLDivElement;
  /** Active CodeMirror view; null until `onMount` runs. */
  let view: EditorView | null = null;
  /**
   * Set true when we dispatch a doc replacement triggered by an
   * external change to `currentRequest`. Prevents the resulting
   * `updateListener` callback from echoing back into the store.
   */
  let suppressUpdate = false;

  /**
   * Compartment wrapping the chrome+highlight extensions returned by
   * `createCodemirrorTheme`. Stored in module scope so `reconfigure()`
   * calls always target the same compartment instance — exactly the
   * trick `MergeEditor` uses for its line-numbers toggle.
   */
  const themeCompartment = new Compartment();

  /**
   * Build a CodeMirror theme extension from the current `activeTheme`.
   * We read the store imperatively (via `get`) when the editor first
   * mounts and again whenever the store changes, so the helper can be
   * called from both the `onMount` setup and the reactive subscription.
   */
  function buildThemeExt() {
    const theme = get(activeTheme);
    return createCodemirrorTheme(
      theme?.editor ?? null,
      theme?.meta.mode !== "light",
    );
  }

  onMount(() => {
    const initial = get(currentRequest)?.body ?? "";
    view = new EditorView({
      state: EditorState.create({
        doc: initial,
        extensions: [
          themeCompartment.of(buildThemeExt()),
          // Render tooltips against document.body with position: fixed
          // so the autocomplete popover escapes the panel's
          // `overflow: hidden` ancestors and isn't clipped behind the
          // response viewer / tabs below.
          tooltips({ position: "fixed", parent: document.body }),
          lineNumbers(),
          history(),
          // Default keymap + history + completion keymap so undo/redo,
          // line nav, Tab/Enter accept, and Ctrl+Space invoke
          // completion all work as expected.
          keymap.of([
            ...defaultKeymap,
            ...historyKeymap,
            ...completionKeymap,
          ]),
          EditorView.lineWrapping,
          json(),
          autocompletion({
            override: [
              varCompletion(
                () => get(activeProject)?.path ?? "",
                () => get(currentEnv),
              ),
            ],
            activateOnTyping: true,
          }),
          EditorView.updateListener.of((u) => {
            if (suppressUpdate || !u.docChanged) return;
            const cur = get(currentRequest);
            if (!cur) return;
            currentRequest.set({ ...cur, body: u.state.doc.toString() });
          }),
          // CodeMirror's default `activateOnTyping` won't fire on the
          // non-word `{` char, so kick the popover ourselves whenever
          // the user just typed `{{`. Defer via `queueMicrotask` so
          // the doc change finishes applying first.
          EditorView.updateListener.of((u) => {
            if (!u.docChanged) return;
            const head = u.state.selection.main.head;
            const before = u.state.doc.sliceString(
              Math.max(0, head - 2),
              head,
            );
            if (before === "{{") {
              queueMicrotask(() => startCompletion(u.view));
            }
          }),
        ],
      }),
      parent: host,
    });
  });

  // Mirror external changes (e.g. after a fresh `requests_load`) into
  // the editor without bouncing them back through the update listener.
  $: if (view) {
    const expected = $currentRequest?.body ?? "";
    if (view.state.doc.toString() !== expected) {
      suppressUpdate = true;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: expected },
      });
      suppressUpdate = false;
    }
  }

  // Re-thread the theme into the live editor whenever the active theme
  // store changes (light/dark toggle, theme picker, etc.).
  $: if (view && $activeTheme !== undefined) {
    view.dispatch({
      effects: themeCompartment.reconfigure(buildThemeExt()),
    });
  }

  onDestroy(() => view?.destroy());
</script>

<div class="body-shell">
  <div class="body-editor" bind:this={host}></div>
</div>

<style>
  .body-shell {
    height: 100%;
    min-height: 200px;
    /* Keep the rounded border around the editor, but let the
       CodeMirror theme own the inner surface so the gutter and
       padding match whatever bg/foreground tokens the active theme
       resolved to. Otherwise a stale `--bg-secondary` showed
       through the gutter as a white column in dark mode. */
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }

  .body-editor {
    height: 100%;
    min-height: 200px;
  }

  .body-editor :global(.cm-editor) {
    height: 100%;
  }
</style>
