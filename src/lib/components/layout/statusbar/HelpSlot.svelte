<!--
  HelpSlot — opens the keyboard shortcut cheat sheet (`Shift+?`).

  Sits in the right cluster of the statusbar so the overlay always has a
  discoverable entry-point even for users who never stumble onto the bare-key
  shortcut. Behaviour matches the global `toggleCheatSheet()` action so
  clicking is symmetric with pressing the shortcut.
-->
<script lang="ts">
  import { toggleCheatSheet } from "$lib/stores/shortcuts";
  import * as m from "$lib/paraglide/messages";
</script>

<button
  class="help-slot"
  title={m.statusbar_help_tooltip()}
  data-testid="statusbar-help-slot"
  onclick={toggleCheatSheet}
  type="button"
  aria-label={m.statusbar_help_tooltip()}
>
  <span aria-hidden="true">?</span>
</button>

<style>
  .help-slot {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    width: 22px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    user-select: none;
    transition: color 0.15s;
  }

  .help-slot:hover {
    color: var(--text-primary);
  }
</style>
