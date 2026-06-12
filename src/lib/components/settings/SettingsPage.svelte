<!--
  SettingsPage.svelte — MT-5 IA shell.

  Replaces the old flat 4-tab layout with the post-Phase-3/4 5-category
  architecture (General / Git / AI / Integrations / Advanced). Built
  entirely on top of the shared `$lib/components/ui` primitives — no
  inline button or card CSS belongs here.

  Responsibilities kept to the shell:

  - `SearchInput` in the top bar (Cmd+K focus comes from the
    primitive itself). Phase 5 wires the query against a flat index
    aggregated from every category component's `settingsIndex`
    export; for now the index is a `$state` populated by callbacks
    the child components fire on mount.
  - `CategoryNav` on the left with keyboard navigation built in.
  - Right-pane dispatch to whichever category component matches the
    active slug.
  - URL deep-linking via `settingsRoute` — `#ai` opens the AI
    category, `#general.theme` opens General + scrolls to the
    `theme` anchor.
  - Legacy deep-link bridge: when `pendingSettingsSection` holds a
    known id (the statusbar writes those) it is translated into the
    new category slug via the shared helper in `settingsRoute`.
  - `Esc` clears a search or, when empty, navigates away from
    Settings entirely (by flipping `activeViewStore` back to the
    default `"graph"` view).
-->
<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$lib/paraglide/messages";
  import { SearchInput, CategoryNav } from "$lib/components/ui";
  import {
    CATEGORY_IDS,
    DEFAULT_CATEGORY,
    bindPendingSectionBridge,
    initSettingsRouteSync,
    setCategory,
    settingsRoute,
    type CategoryId,
  } from "$lib/stores/settingsRoute";
  import { activeViewStore } from "$lib/stores/navigation";
  import GeneralSettings, {
    settingsIndex as generalIndex,
  } from "./GeneralSettings.svelte";
  import EditorSettings, {
    settingsIndex as editorIndex,
  } from "./EditorSettings.svelte";
  import GitSettings, {
    settingsIndex as gitIndex,
  } from "./GitSettings.svelte";
  import AiSettings, {
    settingsIndex as aiIndex,
  } from "./AiSettings.svelte";
  import IntegrationsSettings, {
    settingsIndex as integrationsIndex,
  } from "./IntegrationsSettings.svelte";
  import AdvancedSettings, {
    settingsIndex as advancedIndex,
  } from "./AdvancedSettings.svelte";
  import type { SettingDescriptor } from "./settings-index";

  /** Svelte-runified active category; the shell binds this to `CategoryNav`. */
  let activeCategory = $state<CategoryId>(DEFAULT_CATEGORY);

  /** Search query — kept on the shell so Phase 5 can jump to matches. */
  let searchQuery = $state("");

  /**
   * Whether the search input (or its dropdown) currently has focus.
   * Drives the dropdown visibility so we can render the
   * "Type to search…" empty state when the field is focused but
   * empty — matching the spec.
   */
  let searchFocused = $state(false);

  /**
   * Flat union of every category's declared settings. Aggregated
   * once at module scope (each category exports its own `const`
   * array) so we avoid re-walking on every render.
   */
  const SETTINGS_INDEX: SettingDescriptor[] = [
    ...generalIndex,
    ...editorIndex,
    ...gitIndex,
    ...aiIndex,
    ...integrationsIndex,
    ...advancedIndex,
  ];

  /** Filtered matches for the current search query (max 10). */
  const matches = $derived.by<SettingDescriptor[]>(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return [];
    return SETTINGS_INDEX.filter(
      (entry) =>
        entry.label.toLowerCase().includes(q) ||
        entry.description.toLowerCase().includes(q),
    ).slice(0, 10);
  });

  /** True when the dropdown should be visible to the user. */
  const showDropdown = $derived(
    searchFocused || searchQuery.trim().length > 0,
  );

  /** Category metadata — label + icon come from Paraglide. */
  const categories = $derived([
    {
      id: "general",
      label: m.settings_cat_general_title(),
      icon: "\uF085", // gear
    },
    {
      id: "editor",
      label: m.settings_category_editor(),
      icon: "\uF044", // pencil-square
    },
    {
      id: "git",
      label: m.settings_cat_git_title(),
      icon: "\uE702", // git branch
    },
    {
      id: "ai",
      label: m.settings_cat_ai_title(),
      // \uF2DB (microchip) ships in every Nerd Font build; the
      // previous \uF544 (robot, FA5-pro codepoint) rendered as
      // tofu on the bundled Nerd Font variant the user has.
      icon: "\uF2DB",
    },
    {
      id: "integrations",
      label: m.settings_cat_integrations_title(),
      icon: "\uF0C1", // link
    },
    {
      id: "advanced",
      label: m.settings_cat_advanced_title(),
      icon: "\uF013", // cog
    },
  ]);

  /** Currently-selected category record (for title + description). */
  const activeMeta = $derived(
    categories.find((c) => c.id === activeCategory) ?? categories[0],
  );

  /** Translated description for the active category. */
  const activeDescription = $derived.by(() => {
    switch (activeCategory) {
      case "general":
        return m.settings_cat_general_description();
      case "editor":
        return m.settings_cat_editor_description();
      case "git":
        return m.settings_cat_git_description();
      case "ai":
        return m.settings_cat_ai_description();
      case "integrations":
        return m.settings_cat_integrations_description();
      case "advanced":
        return m.settings_cat_advanced_description();
      default:
        return "";
    }
  });

  onMount(() => {
    // Seed from the URL + keep it synced after that.
    const stopSync = initSettingsRouteSync();
    const unsubBridge = bindPendingSectionBridge();
    const unsubRoute = settingsRoute.subscribe((route) => {
      activeCategory = route.category;
      if (route.anchor) {
        // Let the DOM render the target before attempting a scroll —
        // `queueMicrotask` is enough because the category component
        // rerenders synchronously when `activeCategory` flips.
        queueMicrotask(() => scrollToAnchor(route.anchor!));
      }
    });
    return () => {
      stopSync();
      unsubBridge();
      unsubRoute();
    };
  });

  function handleSelectCategory(id: string) {
    if (!(CATEGORY_IDS as readonly string[]).includes(id)) return;
    setCategory(id);
  }

  function handleSearch(value: string) {
    searchQuery = value;
  }

  function jumpToMatch(entry: SettingDescriptor) {
    setCategory(entry.category, entry.anchor);
    searchQuery = "";
  }

  function scrollToAnchor(anchor: string) {
    const el =
      document.getElementById(anchor) ??
      document.querySelector<HTMLElement>(
        `[data-setting-anchor="${anchor}"]`,
      );
    if (!el) return;
    el.scrollIntoView({ behavior: "smooth", block: "start" });
    el.classList.add("highlight");
    window.setTimeout(() => el.classList.remove("highlight"), 1200);
  }

  function handleShellKey(event: KeyboardEvent) {
    if (event.key !== "Escape") return;
    if (searchQuery) {
      searchQuery = "";
      return;
    }
    // No active query — bail back to the graph view (the user's
    // preferred "I'm done with Settings" escape hatch).
    activeViewStore.set("graph");
  }

  /**
   * When the user presses Enter inside the search input we jump to
   * the first match (if any). Matches what the spec calls out:
   * "Pressing Enter on the focused result OR on the first result
   * when search has a value".
   */
  function handleSearchKey(event: KeyboardEvent) {
    if (event.key === "Enter") {
      if (matches.length > 0) {
        event.preventDefault();
        jumpToMatch(matches[0]);
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      searchQuery = "";
    }
  }

  /** Enter on a focused result jumps to it; arrow keys let the user cycle. */
  function handleResultKey(event: KeyboardEvent, entry: SettingDescriptor) {
    if (event.key === "Enter") {
      event.preventDefault();
      jumpToMatch(entry);
    }
  }

  function handleSearchFocus() {
    searchFocused = true;
  }

  function handleSearchBlur(event: FocusEvent) {
    // Keep the dropdown open while focus is still within the
    // container (e.g. the user clicked a result). We check
    // `relatedTarget` against the wrapper rather than relying on a
    // timeout — more deterministic for the click-result path.
    const next = event.relatedTarget as Node | null;
    const wrapper = (event.currentTarget as HTMLElement | null)
      ?.parentElement;
    if (next && wrapper?.contains(next)) return;
    searchFocused = false;
  }
</script>

<svelte:window onkeydown={handleShellKey} />

<div class="settings-page" data-testid="settings-page">
  <div class="settings-topbar">
    <div class="settings-topbar__title">{m.settings_title()}</div>
    <!--
      Event delegation wrapper — the key/focus listeners bubble up
      from the `<input>` inside `SearchInput` and from each result
      `<button>`. The div itself stays non-interactive (no role, no
      tabindex) so screen readers announce the input, not the wrapper.
    -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="settings-topbar__search"
      onkeydown={handleSearchKey}
      onfocusin={handleSearchFocus}
      onfocusout={handleSearchBlur}
    >
      <SearchInput
        bind:value={searchQuery}
        placeholder={m.settings_search_placeholder()}
        onSearch={handleSearch}
      />
      {#if showDropdown}
        <div
          class="settings-search-results"
          data-testid="settings-search-results"
          role="listbox"
        >
          {#if searchQuery.trim().length === 0}
            <div class="settings-search-empty" data-testid="settings-search-empty">
              {m.settings_search_empty_state()}
            </div>
          {:else if matches.length === 0}
            <div class="settings-search-empty" data-testid="settings-search-none">
              {m.settings_search_no_results()}
            </div>
          {:else}
            {#each matches as match (match.id)}
              <button
                type="button"
                class="settings-search-result"
                data-testid={`settings-search-result-${match.id}`}
                onclick={() => jumpToMatch(match)}
                onkeydown={(e) => handleResultKey(e, match)}
                role="option"
                aria-selected="false"
              >
                <span class="settings-search-result__label">{match.label}</span>
                <span class="settings-search-result__meta">
                  {categories.find((c) => c.id === match.category)?.label ?? ""}
                </span>
              </button>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <div class="settings-body">
    <aside class="settings-sidebar">
      <CategoryNav
        categories={categories.map((c) => ({
          id: c.id,
          label: c.label,
          icon: c.icon,
        }))}
        bind:activeId={activeCategory as unknown as string}
        onSelect={handleSelectCategory}
      />
    </aside>

    <section class="settings-content" data-testid="settings-content">
      <header class="settings-content__header">
        <h2 class="settings-content__title">{activeMeta.label}</h2>
        {#if activeDescription}
          <p class="settings-content__description">{activeDescription}</p>
        {/if}
      </header>

      <div class="settings-content__body">
        {#if activeCategory === "general"}
          <GeneralSettings />
        {:else if activeCategory === "editor"}
          <EditorSettings />
        {:else if activeCategory === "git"}
          <GitSettings />
        {:else if activeCategory === "ai"}
          <AiSettings />
        {:else if activeCategory === "integrations"}
          <IntegrationsSettings />
        {:else if activeCategory === "advanced"}
          <AdvancedSettings />
        {/if}
      </div>
    </section>
  </div>
</div>

<style>
  .settings-page {
    display: flex;
    flex: 1;
    flex-direction: column;
    overflow: hidden;
  }

  .settings-topbar {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .settings-topbar__title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-primary);
    flex-shrink: 0;
  }

  .settings-topbar__search {
    flex: 1;
    max-width: 420px;
    position: relative;
  }

  .settings-search-results {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(100% + 6px);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: var(--shadow-overlay);
    z-index: 10;
    max-height: 280px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .settings-search-result {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-family: inherit;
    cursor: pointer;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }

  .settings-search-result:last-child {
    border-bottom: none;
  }

  .settings-search-result:hover {
    background: var(--overlay-hover);
  }

  .settings-search-result__meta {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .settings-search-empty {
    padding: 10px 12px;
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    text-align: center;
    font-style: italic;
  }

  .settings-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .settings-sidebar {
    width: clamp(160px, 14vw, 220px);
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    background: var(--bg-secondary);
    padding: 8px 0;
    overflow-y: auto;
  }

  .settings-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .settings-content__header {
    padding: 20px 28px 12px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .settings-content__title {
    margin: 0;
    font-size: var(--font-size-xl);
    font-weight: 600;
    color: var(--text-primary);
  }

  .settings-content__description {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .settings-content__body {
    padding: 20px 28px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* Pulse highlight applied when jumping to a search result. */
  :global(.highlight) {
    animation: bg-setting-pulse 1.1s ease-out;
  }

  @keyframes bg-setting-pulse {
    0% {
      box-shadow: 0 0 0 0 var(--overlay-accent-blue);
    }
    100% {
      box-shadow: 0 0 0 16px transparent;
    }
  }
</style>
