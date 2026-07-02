<!--
  RefPicker — autocomplete input over branches + tags, with free-form SHA
  entry. Used for the two sides of the compare header. Selecting a suggestion
  (or pressing Enter on a typed value) emits the ref via `onSelect`; the
  backend resolves whatever revspec is passed (branch, tag, HEAD, SHA).
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";

  export interface RefOption {
    name: string;
    kind: "branch" | "tag";
  }

  let {
    label,
    value,
    options,
    placeholder,
    onSelect,
  }: {
    label: string;
    value: string | null;
    options: RefOption[];
    placeholder?: string;
    onSelect: (ref: string) => void;
  } = $props();

  let query = $state("");
  let open = $state(false);
  let activeIndex = $state(0);
  let inputEl: HTMLInputElement | undefined = $state();

  // Mirror the committed value into the input whenever it changes externally
  // (swap, entry-point pre-fill) and the field isn't being edited.
  $effect(() => {
    if (!open) query = value ?? "";
  });

  let filtered = $derived.by<RefOption[]>(() => {
    const q = query.trim().toLowerCase();
    const list = q.length === 0 ? options : options.filter((o) => o.name.toLowerCase().includes(q));
    return list.slice(0, 50);
  });

  $effect(() => {
    if (activeIndex >= filtered.length) activeIndex = Math.max(0, filtered.length - 1);
  });

  function commit(ref: string) {
    const trimmed = ref.trim();
    if (!trimmed) return;
    open = false;
    onSelect(trimmed);
    inputEl?.blur();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      open = true;
      activeIndex = (activeIndex + 1) % Math.max(1, filtered.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      activeIndex = (activeIndex - 1 + filtered.length) % Math.max(1, filtered.length);
    } else if (e.key === "Enter") {
      e.preventDefault();
      // Prefer the highlighted suggestion; otherwise use the raw typed value
      // so SHAs / arbitrary revspecs work.
      commit(filtered[activeIndex]?.name ?? query);
    } else if (e.key === "Escape") {
      open = false;
      query = value ?? "";
    }
  }
</script>

<div class="ref-picker">
  <span class="rp-label">{label}</span>
  <div class="rp-field">
    <input
      bind:this={inputEl}
      bind:value={query}
      type="text"
      class="rp-input"
      {placeholder}
      autocomplete="off"
      spellcheck="false"
      aria-label={label}
      onfocus={() => (open = true)}
      oninput={() => (open = true)}
      onkeydown={onKeydown}
      onblur={() => setTimeout(() => (open = false), 120)}
    />
    {#if open && filtered.length > 0}
      <ul class="rp-list" role="listbox" aria-label={label}>
        {#each filtered as opt, i (opt.kind + ":" + opt.name)}
          <li>
            <button
              type="button"
              class="rp-item"
              class:rp-item--active={i === activeIndex}
              role="option"
              aria-selected={i === activeIndex}
              onmousedown={(e) => { e.preventDefault(); commit(opt.name); }}
              onmouseenter={() => (activeIndex = i)}
            >
              <span class="rp-kind" class:rp-kind--tag={opt.kind === "tag"}>
                {opt.kind === "tag" ? m.compare_kind_tag() : m.compare_kind_branch()}
              </span>
              <span class="rp-name">{opt.name}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .ref-picker {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    flex: 1;
  }

  .rp-label {
    font-size: var(--font-size-2xs);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
    font-weight: 600;
  }

  .rp-field {
    position: relative;
  }

  .rp-input {
    width: 100%;
    box-sizing: border-box;
    padding: 5px 8px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 5px;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    outline: none;
  }
  .rp-input:focus {
    border-color: var(--accent-primary);
  }

  .rp-list {
    position: absolute;
    top: calc(100% + 2px);
    left: 0;
    right: 0;
    z-index: 20;
    margin: 0;
    padding: 4px;
    list-style: none;
    max-height: 260px;
    overflow-y: auto;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: var(--shadow-overlay);
  }

  .rp-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 8px;
    background: none;
    border: none;
    border-radius: 4px;
    text-align: left;
    cursor: pointer;
    color: var(--text-primary);
    font: inherit;
  }
  .rp-item--active {
    background: var(--overlay-accent-blue);
  }

  .rp-kind {
    flex-shrink: 0;
    font-size: var(--font-size-2xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--accent-primary);
    min-width: 46px;
  }
  .rp-kind--tag {
    color: var(--accent-purple);
  }

  .rp-name {
    flex: 1;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
