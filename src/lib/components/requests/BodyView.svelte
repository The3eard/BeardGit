<!--
  BodyView.svelte — Pretty/Raw response body viewer.

  Pretty mode mounts a read-only CodeMirror `EditorView` so JSON
  responses get the same syntax highlighting + theme as the body
  editor in `BodyTab.svelte`. Without highlighting, "Pretty" looked
  identical to "Raw" for already-pretty JSON, which was the bug.

  Raw mode keeps the lightweight `<pre>` rendering so very large
  non-JSON payloads (HTML dumps, binary-as-text, etc.) don't pay the
  cost of a CodeMirror parse. Both panes stay mounted and the
  inactive one is hidden via `display: none` so toggling between
  modes preserves the editor's scroll/selection state.

  Language detection reads the latest response's `Content-Type`. If
  it matches `*/*+json` (or `application/json`) we attach the JSON
  language extension via a `Compartment.reconfigure` so it can swap
  cleanly when the user runs a different request without recreating
  the view. Other content types fall back to plain text (still
  themed; just no parser). Adding `lang-html` / `lang-xml` /
  `lang-markdown` is a one-liner each — deferred for v1, JSON is the
  overwhelmingly common case.

  Toolbar buttons stay on the shared `Button` primitive so the
  Pretty/Raw segmented control reads the same as every other
  segmented switch in the app.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, lineNumbers } from "@codemirror/view";
  import { EditorState, Compartment } from "@codemirror/state";
  import { json } from "@codemirror/lang-json";
  import { Button } from "$lib/components/ui";
  import { lastResponse } from "./stores";
  import { activeTheme } from "$lib/stores/theme";
  import { createCodemirrorTheme } from "$lib/components/editor/codemirror-theme";
  import { get } from "svelte/store";

  /** DOM container that hosts the read-only `EditorView`. */
  let host: HTMLDivElement;
  /** Active CodeMirror view; null until `onMount` runs. */
  let view: EditorView | null = null;

  /**
   * Compartment wrapping the chrome+highlight extensions so we can
   * re-thread a new theme without recreating the view when the user
   * toggles light/dark mode.
   */
  const themeCompartment = new Compartment();
  /**
   * Compartment wrapping the language extension so we can swap
   * `json()` in or out depending on the response's `Content-Type`
   * without rebuilding the editor.
   */
  const langCompartment = new Compartment();

  /** Pretty/Raw segmented mode. */
  let mode: "pretty" | "raw" = "pretty";

  /** Raw decoded body text, or empty string when there is no response yet. */
  $: text = $lastResponse ? new TextDecoder().decode($lastResponse.body) : "";

  /** Pretty-printed JSON when the body parses, otherwise the raw text. */
  $: pretty = (() => {
    try {
      return JSON.stringify(JSON.parse(text), null, 2);
    } catch {
      return text;
    }
  })();

  /** `Content-Type` header from the latest response (lower-cased lookup). */
  $: contentType =
    ($lastResponse?.headers ?? []).find(
      ([k]) => k.toLowerCase() === "content-type",
    )?.[1] ?? "";

  /**
   * True when the response looks like JSON (either by Content-Type or
   * by the body parsing successfully). The fallback parse-check covers
   * servers that forget to set the header.
   */
  $: isJson =
    /\bjson\b/i.test(contentType) ||
    (() => {
      if (!text) return false;
      try {
        JSON.parse(text);
        return true;
      } catch {
        return false;
      }
    })();

  /**
   * Build the CodeMirror theme extension from the active app theme.
   * Read imperatively via `get()` so we can call this from both the
   * initial `onMount` setup and the reactive theme-change subscription.
   */
  function buildThemeExt() {
    const theme = get(activeTheme);
    return createCodemirrorTheme(
      theme?.editor ?? null,
      theme?.meta.mode !== "light",
    );
  }

  /** Build the language extension for the current response. */
  function buildLangExt() {
    return isJson ? json() : [];
  }

  onMount(() => {
    view = new EditorView({
      state: EditorState.create({
        doc: pretty,
        extensions: [
          themeCompartment.of(buildThemeExt()),
          langCompartment.of(buildLangExt()),
          lineNumbers(),
          EditorView.lineWrapping,
          // Read-only viewer: explicitly mark both the state and the
          // view as non-editable so the cursor doesn't appear and key
          // input is dropped.
          EditorState.readOnly.of(true),
          EditorView.editable.of(false),
        ],
      }),
      parent: host,
    });
  });

  onDestroy(() => view?.destroy());

  // Sync the doc whenever the formatted response text changes (new
  // run finishes, mode toggle, etc.). We compare against the current
  // editor doc so we don't dispatch identity-no-op transactions.
  $: if (view) {
    if (view.state.doc.toString() !== pretty) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: pretty },
      });
    }
  }

  // Re-thread the theme into the live editor whenever the active
  // theme store changes (light/dark toggle, theme picker, etc.).
  $: if (view && $activeTheme !== undefined) {
    view.dispatch({
      effects: themeCompartment.reconfigure(buildThemeExt()),
    });
  }

  // Reconfigure the language extension whenever the detected
  // content-type flips (a fresh response with a different shape).
  $: if (view) {
    void isJson; // tracked dependency
    view.dispatch({
      effects: langCompartment.reconfigure(buildLangExt()),
    });
  }
</script>

<div class="body-view">
  <div class="toolbar">
    <Button
      variant="neutral"
      size="xs"
      active={mode === "pretty"}
      onclick={() => (mode = "pretty")}
    >
      Pretty
    </Button>
    <Button
      variant="neutral"
      size="xs"
      active={mode === "raw"}
      onclick={() => (mode = "raw")}
    >
      Raw
    </Button>
  </div>
  <!--
    Both panes stay mounted and the inactive one is hidden so the
    CodeMirror view's scroll/selection state survives a Pretty<->Raw
    toggle. Otherwise we'd reflow + remeasure on every flip.
  -->
  <div class="pane" class:hidden-pane={mode !== "pretty"}>
    <div class="cm-host" bind:this={host}></div>
  </div>
  <div class="pane" class:hidden-pane={mode !== "raw"}>
    <pre>{text}</pre>
  </div>
</div>

<style>
  .body-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .toolbar {
    display: flex;
    gap: 4px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
  }

  .pane {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  .pane.hidden-pane {
    display: none;
  }

  .cm-host {
    height: 100%;
  }

  .cm-host :global(.cm-editor) {
    height: 100%;
  }

  pre {
    margin: 0;
    padding: 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-primary);
    background: var(--bg-secondary);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
