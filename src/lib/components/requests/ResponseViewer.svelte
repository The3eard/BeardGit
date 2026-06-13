<!--
  Top-level Response viewer for the Requests panel.

  Composes a status header (lifecycle / status code / duration) with a
  tabbed pane that surfaces the response body, headers, set-cookies,
  and per-request history (with diff). Each tab is a small dedicated
  Svelte file under `requests/` so they can evolve independently — see
  `BodyView`, `HeadersView`, `CookiesView`, `HistoryView`.

  Tab visuals mirror `RequestEditor`'s strip and reuse the `CategoryNav`
  vocabulary (accent-blue tonal active state, `--overlay-hover` for
  hover) so the two panes feel like one design language.
-->
<script lang="ts">
  import ResponseHeaderBar from "./ResponseHeaderBar.svelte";
  import BodyView from "./BodyView.svelte";
  import HeadersView from "./HeadersView.svelte";
  import CookiesView from "./CookiesView.svelte";
  import HistoryView from "./HistoryView.svelte";

  type TabName = "body" | "headers" | "cookies" | "history";
  const TABS: { id: TabName; label: string }[] = [
    { id: "body", label: "Body" },
    { id: "headers", label: "Headers" },
    { id: "cookies", label: "Cookies" },
    { id: "history", label: "History" },
  ];
  let tab: TabName = "body";
</script>

<div class="viewer">
  <ResponseHeaderBar />
  <div class="tabs" role="tablist" aria-label="Response viewer tabs">
    {#each TABS as t (t.id)}
      <button
        type="button"
        role="tab"
        class="tabs__item"
        class:tabs__item--active={tab === t.id}
        aria-selected={tab === t.id}
        on:click={() => (tab = t.id)}
      >
        {t.label}
      </button>
    {/each}
  </div>
  <div class="pane">
    {#if tab === "body"}<BodyView />{/if}
    {#if tab === "headers"}<HeadersView />{/if}
    {#if tab === "cookies"}<CookiesView />{/if}
    {#if tab === "history"}<HistoryView />{/if}
  </div>
</div>

<style>
  .viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
  }

  .tabs__item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    font-family: inherit;
    font-size: var(--font-size-sm);
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease,
      border-color 0.12s ease;
  }

  .tabs__item:hover {
    background: var(--overlay-hover);
    color: var(--text-primary);
  }

  .tabs__item:focus-visible {
    outline: none;
    border-color: var(--accent-primary);
  }

  .tabs__item--active {
    background: var(--overlay-accent-blue);
    color: var(--text-primary);
  }

  .pane {
    flex: 1;
    overflow: auto;
  }
</style>
