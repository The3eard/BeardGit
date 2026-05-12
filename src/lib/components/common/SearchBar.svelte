<script lang="ts">
  import type { SearchTag, FilterDef } from "../../search/types";
  import { parseInput } from "../../search/parser";
  import * as m from "$lib/paraglide/messages";
  import IconButton from "$lib/components/ui/IconButton.svelte";

  let {
    filters,
    tags = $bindable([]),
    placeholder = m.search_placeholder(),
    onSearch,
    testId,
  }: {
    filters: FilterDef[];
    tags: SearchTag[];
    placeholder?: string;
    onSearch: (tags: SearchTag[]) => void;
    /** Optional data-testid for the search `<input>` (E2E hook). */
    testId?: string;
  } = $props();

  let inputText = $state("");
  let showHints = $state(false);
  let inputEl: HTMLInputElement | undefined = $state();

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && inputText.trim()) {
      e.preventDefault();
      const availableTypes = filters.map(f => f.type);
      const newTags = parseInput(inputText, availableTypes);
      if (newTags.length > 0) {
        tags = [...tags, ...newTags];
        inputText = "";
        onSearch(tags);
      }
    } else if (e.key === "Backspace" && inputText === "" && tags.length > 0) {
      tags = tags.slice(0, -1);
      onSearch(tags);
    }
  }

  function removeTag(id: string) {
    tags = tags.filter(t => t.id !== id);
    onSearch(tags);
  }

  function handleFocus() {
    showHints = true;
  }

  function handleBlur() {
    // Delay hiding hints so click on hint can register
    setTimeout(() => { showHints = false; }, 200);
  }

  function tagColor(type: string): string {
    switch (type) {
      case "branch": return "var(--accent-primary)";
      case "user": return "var(--accent-green)";
      case "commit": return "var(--accent-orange)";
      case "status": return "var(--accent-purple)";
      default: return "var(--text-secondary)";
    }
  }
</script>

<div class="search-bar">
  <div class="search-tags-input">
    {#each tags as tag (tag.id)}
      <span class="search-tag" class:tag-filter={tag.type !== 'text'} style="--tag-color: {tagColor(tag.type)}">
        {#if tag.type !== 'text'}
          <span class="tag-type">{tag.type}</span>
        {/if}
        <span class="tag-value">{tag.value}</span>
        <IconButton tone="default" size="sm" icon={""} description="Remove filter" onclick={() => removeTag(tag.id)} />
      </span>
    {/each}
    <input
      bind:this={inputEl}
      class="search-input"
      type="text"
      bind:value={inputText}
      {placeholder}
      onkeydown={handleKeyDown}
      onfocus={handleFocus}
      onblur={handleBlur}
      data-testid={testId}
    />
  </div>
  {#if showHints && filters.length > 0}
    <div class="search-hints">
      {#each filters as f}
        <span class="hint" style="--hint-color: {tagColor(f.type)}">{f.type}:{f.placeholder}</span>
      {/each}
      <span class="hint hint-separator">{m.search_hint()}</span>
    </div>
  {/if}
</div>

<style>
  .search-bar {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    position: relative;
  }

  .search-tags-input {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    padding: 3px 6px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    min-height: 28px;
    cursor: text;
  }

  .search-tags-input:focus-within {
    border-color: var(--accent-primary);
  }

  .search-tag {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 1px 4px;
    border-radius: 3px;
    font-size: 11px;
    line-height: 1.4;
    background: var(--overlay-accent-muted);
    color: var(--text-secondary);
    white-space: nowrap;
    max-width: 200px;
  }

  .search-tag.tag-filter {
    background: color-mix(in srgb, var(--tag-color) 15%, transparent);
    color: var(--tag-color);
  }

  .tag-type {
    font-weight: 600;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    opacity: 0.85;
  }

  .tag-type::after {
    content: ":";
  }

  .tag-value {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .search-input {
    flex: 1;
    min-width: 80px;
    padding: 2px 4px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
  }

  .search-input::placeholder {
    color: var(--text-secondary);
    opacity: 0.6;
  }

  .search-hints {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 4px 6px;
    margin-top: 4px;
    font-size: 11px;
  }

  .hint {
    color: var(--hint-color, var(--text-secondary));
    opacity: 0.7;
    font-family: "SF Mono", Menlo, monospace;
    font-size: 10px;
  }

  .hint-separator {
    color: var(--text-secondary);
    opacity: 0.4;
    font-style: italic;
    font-family: -apple-system, BlinkMacSystemFont, sans-serif;
  }
</style>
