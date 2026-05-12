<!--
  SearchInput.svelte — debounced search input primitive.

  The Settings top-bar search field. Wires:
  - Two-way `bind:value` so parents can read the current query.
  - A debounce (150 ms default, configurable) before emitting `onSearch`
    — the input itself updates synchronously for caret responsiveness.
  - A clear (X) button rendered only when the value is non-empty, which
    resets the value and emits `onSearch("")` immediately.
  - A global `Cmd+K` / `Ctrl+K` listener that focuses the input. The
    listener is registered on mount and cleaned up on destroy.

  ```svelte
  <SearchInput
    bind:value={query}
    placeholder="Search settings…"
    onSearch={handleSearch}
  />
  ```
-->
<script lang="ts">
  import { onMount } from "svelte";

  interface Props {
    /** Current query. Two-way bindable. */
    value?: string;
    /** Placeholder text. */
    placeholder?: string;
    /** Debounce for the search event in ms. Default 150. */
    debounceMs?: number;
    /**
     * Called with the query after the debounce window elapses and when
     * the clear button resets the value (fired immediately with "").
     */
    onSearch?: (value: string) => void;
  }

  let {
    value = $bindable(""),
    placeholder = "",
    debounceMs = 150,
    onSearch,
  }: Props = $props();

  let inputEl = $state<HTMLInputElement | null>(null);
  let timer: ReturnType<typeof setTimeout> | null = null;

  function handleInput(event: Event) {
    const next = (event.target as HTMLInputElement).value;
    value = next;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      onSearch?.(next);
      timer = null;
    }, debounceMs);
  }

  function clear() {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
    value = "";
    onSearch?.("");
    inputEl?.focus();
  }

  function handleGlobalKey(event: KeyboardEvent) {
    const isCmdK =
      (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k";
    if (isCmdK) {
      event.preventDefault();
      inputEl?.focus();
      inputEl?.select();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleGlobalKey);
    return () => {
      window.removeEventListener("keydown", handleGlobalKey);
      if (timer) clearTimeout(timer);
    };
  });
</script>

<div class="bg-search">
  <span class="bg-search__icon nf" aria-hidden="true">{"\uF002"}</span>
  <input
    bind:this={inputEl}
    class="bg-search__input"
    type="search"
    {value}
    {placeholder}
    data-testid="bg-search-input"
    oninput={handleInput}
  />
  {#if value}
    <button
      type="button"
      class="bg-search__clear"
      aria-label="Clear search"
      data-testid="bg-search-clear"
      onclick={clear}
    >
      {"\u00D7"}
    </button>
  {/if}
</div>

<style>
  .bg-search {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: var(--bg-toolbar);
    border: 1px solid var(--border);
    border-radius: 6px;
    transition: border-color 0.15s ease;
  }

  .bg-search:focus-within {
    border-color: var(--accent-primary);
  }

  .bg-search__icon {
    color: var(--text-secondary);
    font-family: var(--font-icons);
    font-size: 12px;
    flex-shrink: 0;
  }

  .bg-search__input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-size: 12px;
    font-family: inherit;
    padding: 0;
    min-width: 0;
  }

  /* Hide the native Safari/WebKit clear button so our custom one is the
     only control visible. */
  .bg-search__input::-webkit-search-cancel-button {
    display: none;
  }

  .bg-search__clear {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 16px;
    cursor: pointer;
    padding: 0 2px;
    line-height: 1;
    border-radius: 3px;
    transition: color 0.12s ease, background 0.12s ease;
  }

  .bg-search__clear:hover {
    color: var(--text-primary);
    background: var(--overlay-hover);
  }
</style>
