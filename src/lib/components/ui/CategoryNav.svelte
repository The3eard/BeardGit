<!--
  CategoryNav.svelte — vertical keyboard-navigable category list.

  Renders a list of settings categories as a vertical nav. Up/Down move
  the selection between categories (wrapping at the edges); Home/End
  jump to the first/last. Click or Enter selects. The currently active
  category is indicated visually and with `aria-current`.

  ```svelte
  <CategoryNav
    categories={[
      { id: "general", label: "General", icon: "\uF085" },
      { id: "appearance", label: "Appearance", icon: "\uF1FC" },
    ]}
    bind:activeId
    onSelect={(id) => goto(id)}
  />
  ```
-->
<script lang="ts">
  interface Category {
    /** Stable slug used in URLs and state (never displayed). */
    id: string;
    /** Human-readable label (translated). */
    label: string;
    /** Optional NerdFont glyph codepoint. */
    icon?: string;
  }

  interface Props {
    /** Ordered list of selectable categories. */
    categories: Category[];
    /** Active category id. Two-way bindable. */
    activeId: string;
    /** Called when the user selects a category. */
    onSelect?: (id: string) => void;
  }

  let {
    categories,
    activeId = $bindable(),
    onSelect,
  }: Props = $props();

  let listEl = $state<HTMLElement | null>(null);

  function focusIndex(index: number) {
    const items = listEl?.querySelectorAll<HTMLButtonElement>(
      ".bg-cat-nav__item",
    );
    if (!items) return;
    const len = items.length;
    if (len === 0) return;
    const wrapped = ((index % len) + len) % len;
    items[wrapped].focus();
  }

  function select(id: string) {
    activeId = id;
    onSelect?.(id);
  }

  function handleKey(event: KeyboardEvent, index: number) {
    const len = categories.length;
    if (len === 0) return;
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        focusIndex(index + 1);
        break;
      case "ArrowUp":
        event.preventDefault();
        focusIndex(index - 1);
        break;
      case "Home":
        event.preventDefault();
        focusIndex(0);
        break;
      case "End":
        event.preventDefault();
        focusIndex(len - 1);
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        select(categories[index].id);
        break;
    }
  }
</script>

<div
  class="bg-cat-nav"
  bind:this={listEl}
  role="tablist"
  aria-orientation="vertical"
>
  {#each categories as category, index (category.id)}
    <button
      type="button"
      class="bg-cat-nav__item"
      class:bg-cat-nav__item--active={category.id === activeId}
      role="tab"
      aria-selected={category.id === activeId}
      aria-current={category.id === activeId ? "page" : undefined}
      tabindex={category.id === activeId ? 0 : -1}
      data-testid={`bg-cat-nav-${category.id}`}
      onclick={() => select(category.id)}
      onkeydown={(e) => handleKey(e, index)}
    >
      {#if category.icon}
        <span class="bg-cat-nav__icon nf" aria-hidden="true"
          >{category.icon}</span
        >
      {/if}
      <span class="bg-cat-nav__label">{category.label}</span>
    </button>
  {/each}
</div>

<style>
  .bg-cat-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px;
  }

  .bg-cat-nav__item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    text-align: left;
    transition:
      background 0.12s ease,
      color 0.12s ease,
      border-color 0.12s ease;
  }

  .bg-cat-nav__item:hover {
    background: var(--overlay-hover);
    color: var(--text-primary);
  }

  .bg-cat-nav__item:focus-visible {
    outline: none;
    border-color: var(--accent-blue);
  }

  .bg-cat-nav__item--active {
    background: var(--overlay-accent-blue);
    color: var(--text-primary);
  }

  .bg-cat-nav__icon {
    font-family: var(--font-icons);
    font-size: 13px;
    width: 16px;
    text-align: center;
    flex-shrink: 0;
  }

  .bg-cat-nav__label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
