<!--
  RequestEditor.svelte — Right-column editor for the active request.

  Composes the URL bar with four tabs (Params / Headers / Body / Auth).
  The component owns request loading: when the user picks a different
  source from the tree, we call `requests_load`, take the first parsed
  request, and stash it in `currentRequest` so the tabs can bind to it.

  The tab bar uses an inline horizontal pattern that mirrors the
  styling vocabulary of `CategoryNav` (active row in accent-blue tonal
  fill, hover in `--overlay-hover`). The vertical `CategoryNav`
  primitive itself doesn't fit a horizontal tab strip, so we replicate
  its visual contract here without inventing new tokens.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import UrlBar from "./UrlBar.svelte";
  import HeadersTab from "./HeadersTab.svelte";
  import ParamsTab from "./ParamsTab.svelte";
  import BodyTab from "./BodyTab.svelte";
  import AuthTab from "./AuthTab.svelte";
  import { activeProject } from "$lib/stores/projects";
  import { currentRequest, currentSource } from "./stores";

  /** Project root for the active tab, or empty when none. */
  $: projectPath = $activeProject?.path ?? "";

  /** Names of the four editor tabs in display order. */
  type TabName = "params" | "headers" | "body" | "auth";
  const TABS: { id: TabName; label: string }[] = [
    { id: "params", label: "Params" },
    { id: "headers", label: "Headers" },
    { id: "body", label: "Body" },
    { id: "auth", label: "Auth" },
  ];
  let tab: TabName = "body";

  /**
   * Load the request from the currently selected source. Defaults to the
   * first parsed request in the file (multi-request `.http` files come
   * later). Resets the store when the source is cleared so stale data
   * from a previous selection doesn't leak through.
   */
  async function loadCurrent() {
    if (!$currentSource) {
      currentRequest.set(null);
      return;
    }
    try {
      const parsed = await invoke<
        Array<{
          name?: string | null;
          method?: string;
          url?: string;
          headers?: [string, string][];
          body?: string | null;
        }>
      >("requests_load", {
        sourceKind: $currentSource.kind,
        sourcePath: $currentSource.path,
        projectPath: projectPath || null,
      });
      const first = parsed[0];
      if (!first) {
        currentRequest.set(null);
        return;
      }
      currentRequest.set({
        name: first.name ?? undefined,
        method: first.method ?? "GET",
        url: first.url ?? "",
        headers: (first.headers ?? []) as [string, string][],
        body: first.body ?? undefined,
      });
    } catch (e) {
      currentRequest.set(null);
      console.error("requests_load failed", e);
    }
  }

  // Reload whenever the user picks a different source from the tree.
  $: $currentSource, loadCurrent();
  onMount(loadCurrent);
</script>

<div class="editor">
  <UrlBar />
  <div class="tabs" role="tablist" aria-label="Request editor tabs">
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
  <div class="tab-body">
    {#if tab === "params"}<ParamsTab />{/if}
    {#if tab === "headers"}<HeadersTab />{/if}
    {#if tab === "body"}<BodyTab />{/if}
    {#if tab === "auth"}<AuthTab />{/if}
  </div>
</div>

<style>
  .editor {
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
    font-size: 12px;
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
    border-color: var(--accent-blue);
  }

  .tabs__item--active {
    background: var(--overlay-accent-blue);
    color: var(--text-primary);
  }

  .tab-body {
    flex: 1;
    overflow: auto;
    padding: 8px;
  }
</style>
