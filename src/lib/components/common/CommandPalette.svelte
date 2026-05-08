<!--
  CommandPalette — Cmd+Shift+P fuzzy-match action picker.

  Indexes (a) the registered keyboard shortcuts and (b) the sidebar's
  navigation views, so users can jump to any panel or trigger any
  shortcut without memorising the bindings. The single biggest
  discoverability win called out in the May 2026 UX audit.

  - Open via shortcut, statusbar entry-point, or `openCommandPalette()`.
  - Up/Down navigate the result list, Enter executes, Escape closes.
  - Match scoring is plain-substring per word — no fuzzy-match library
    pulled in — but the search splits the query on whitespace so
    "stash list" matches "Show stashes".
-->
<script lang="ts">
  import { commandPaletteOpen, closeCommandPalette } from "$lib/stores/commandPalette";
  import { shortcuts, formatShortcut, type Shortcut } from "$lib/stores/shortcuts";
  import { activeViewStore } from "$lib/stores/navigation";
  import * as m from "$lib/paraglide/messages";

  type CommandKind = "navigation" | "shortcut";
  interface Command {
    id: string;
    kind: CommandKind;
    label: string;
    shortcut?: string;
    run: () => void;
  }

  function navItems(): Command[] {
    const views: Array<[string, () => string]> = [
      ["graph", m.sidebar_graph],
      ["changes", m.sidebar_changes],
      ["editor", m.sidebar_editor],
      ["branches", m.sidebar_branches],
      ["tags", m.sidebar_tags],
      ["stashes", m.sidebar_stashes],
      ["worktrees", m.sidebar_worktrees],
      ["reflog", m.sidebar_reflog],
      ["bisect", m.sidebar_bisect],
      ["submodules", m.sidebar_submodules],
      ["ai-config", m.sidebar_ai_config],
      ["ai-sessions", m.sidebar_ai_sessions],
      ["requests", m.sidebar_requests],
    ];
    return views.map(([id, label]) => ({
      id: `nav.${id}`,
      kind: "navigation" as const,
      label: label(),
      run: () => activeViewStore.set(id),
    }));
  }

  function shortcutItems(list: Shortcut[]): Command[] {
    return list
      .filter((s) => s.action) // hide pure render-only entries (search anchors, etc.)
      .map((s) => ({
        id: `sc.${s.id}`,
        kind: "shortcut" as const,
        label: s.label,
        shortcut: formatShortcut(s.keys),
        run: () => s.action(),
      }));
  }

  let query = $state("");
  let selectedIndex = $state(0);
  let inputEl: HTMLInputElement | undefined = $state();

  /** Focus and reset state every time the palette opens. */
  $effect(() => {
    if ($commandPaletteOpen) {
      query = "";
      selectedIndex = 0;
      // Defer focus so the input is mounted before we set it.
      queueMicrotask(() => inputEl?.focus());
    }
  });

  let allCommands = $derived([...navItems(), ...shortcutItems($shortcuts)]);

  /** Plain word-substring match. Score = count of matched query tokens. */
  let filtered = $derived.by<Command[]>(() => {
    const q = query.trim().toLowerCase();
    if (q.length === 0) return allCommands;
    const tokens = q.split(/\s+/).filter(Boolean);
    return allCommands
      .map((c) => {
        const haystack = c.label.toLowerCase();
        const score = tokens.every((t) => haystack.includes(t)) ? tokens.length : 0;
        return { c, score };
      })
      .filter(({ score }) => score > 0)
      .sort((a, b) => b.score - a.score)
      .map(({ c }) => c);
  });

  /** Keep selectedIndex within bounds whenever the list shrinks. */
  $effect(() => {
    if (selectedIndex >= filtered.length) selectedIndex = Math.max(0, filtered.length - 1);
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % Math.max(1, filtered.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + filtered.length) % Math.max(1, filtered.length);
    } else if (e.key === "Enter") {
      e.preventDefault();
      const cmd = filtered[selectedIndex];
      if (cmd) {
        closeCommandPalette();
        // Defer the action one tick so the close animation/state flush
        // doesn't interleave with whatever the action does (e.g. opening
        // another dialog).
        queueMicrotask(() => cmd.run());
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      closeCommandPalette();
    }
  }

  function handleBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) closeCommandPalette();
  }
</script>

{#if $commandPaletteOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div class="cp-backdrop" onclick={handleBackdrop} role="presentation">
    <div class="cp-panel" role="dialog" aria-labelledby="cp-title" aria-modal="true">
      <h2 id="cp-title" class="cp-title">{m.command_palette_title()}</h2>
      <input
        bind:this={inputEl}
        bind:value={query}
        onkeydown={onKeydown}
        type="text"
        class="cp-input"
        placeholder={m.command_palette_placeholder()}
        aria-label={m.command_palette_placeholder()}
        autocomplete="off"
        spellcheck="false"
      />
      {#if filtered.length === 0}
        <p class="cp-empty">{m.command_palette_no_results()}</p>
      {:else}
        <ul class="cp-list" role="listbox" aria-label={m.command_palette_title()}>
          {#each filtered as cmd, i (cmd.id)}
            <li
              class="cp-item"
              class:cp-item--selected={i === selectedIndex}
              role="option"
              aria-selected={i === selectedIndex}
            >
              <button
                type="button"
                class="cp-item__btn"
                onclick={() => {
                  closeCommandPalette();
                  queueMicrotask(() => cmd.run());
                }}
                onmouseenter={() => (selectedIndex = i)}
              >
                <span class="cp-item__kind">
                  {cmd.kind === "navigation"
                    ? m.command_palette_section_navigation()
                    : m.command_palette_section_shortcuts()}
                </span>
                <span class="cp-item__label">{cmd.label}</span>
                {#if cmd.shortcut}
                  <kbd class="cp-item__shortcut">{cmd.shortcut}</kbd>
                {/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
{/if}

<style>
  .cp-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5); /* beardgit:allow-hex: modal backdrop neutral */
    z-index: 1000;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
  }

  .cp-panel {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: min(560px, 90vw);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 12px 32px var(--overlay-shadow);
    overflow: hidden;
  }

  .cp-title {
    margin: 0;
    padding: 12px 16px 6px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .cp-input {
    margin: 0 12px 8px;
    padding: 8px 12px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font: inherit;
    font-size: 14px;
    outline: none;
  }
  .cp-input:focus {
    border-color: var(--accent-primary);
  }

  .cp-empty {
    padding: 16px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .cp-list {
    list-style: none;
    margin: 0;
    padding: 4px;
    overflow-y: auto;
    flex: 1;
  }

  .cp-item {
    margin: 0;
  }

  .cp-item__btn {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px 10px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    font: inherit;
  }

  .cp-item--selected .cp-item__btn {
    background: var(--overlay-accent-blue);
  }

  .cp-item__kind {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
    flex-shrink: 0;
    min-width: 64px;
  }

  .cp-item__label {
    flex: 1;
    font-size: 13px;
  }

  .cp-item__shortcut {
    padding: 1px 6px;
    background: var(--overlay-active);
    border: 1px solid var(--border);
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
  }
</style>
