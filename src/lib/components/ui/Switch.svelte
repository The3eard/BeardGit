<!--
  Switch.svelte — BeardGit-styled toggle for boolean settings.

  The canonical control for on/off *preferences* (Settings, repo
  config). A real, visually-hidden checkbox input with `role="switch"`
  keeps native semantics — `event.target.checked`, keyboard toggling
  and screen-reader support all work unchanged — so call sites can
  swap `<input type="checkbox" …>` for `<Switch …>` one-for-one.

  For *selection* semantics ("include this item in the operation")
  use `Checkbox.svelte` instead.
-->
<script lang="ts">
  interface Props {
    /** Whether the switch is on. */
    checked?: boolean;
    /** Disables interaction and dims the control. */
    disabled?: boolean;
    /** Forwarded to the underlying input (label targeting). */
    id?: string;
    /** Accessible name — required when there's no visible label. */
    ariaLabel?: string;
    /** Optional `data-testid` forwarded to the underlying input. */
    testid?: string;
    /** Click handler on the input (receives the native event). */
    onclick?: (event: MouseEvent) => void;
    /** Change handler on the input (receives the native event). */
    onchange?: (event: Event) => void;
  }

  let {
    checked = false,
    disabled = false,
    id,
    ariaLabel,
    testid,
    onclick,
    onchange,
  }: Props = $props();
</script>

<label class="bg-switch" class:bg-switch--disabled={disabled}>
  <input
    {id}
    type="checkbox"
    role="switch"
    {checked}
    {disabled}
    aria-label={ariaLabel}
    data-testid={testid}
    {onclick}
    {onchange}
  />
  <span class="bg-switch__track" aria-hidden="true">
    <span class="bg-switch__thumb"></span>
  </span>
</label>

<style>
  .bg-switch {
    display: inline-flex;
    align-items: center;
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
  }

  .bg-switch--disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  /* Real input kept for semantics, visually replaced by the track. It
     covers the whole label so the hit target matches the visual. */
  .bg-switch input {
    position: absolute;
    inset: 0;
    margin: 0;
    opacity: 0;
    cursor: inherit;
  }

  .bg-switch__track {
    display: inline-flex;
    align-items: center;
    width: 30px;
    height: 16px;
    padding: 2px;
    border-radius: 8px;
    box-sizing: border-box;
    background: color-mix(in srgb, var(--text-secondary) 35%, transparent);
    transition: background 0.15s ease;
  }

  .bg-switch:hover input:not(:disabled) + .bg-switch__track {
    background: color-mix(in srgb, var(--text-secondary) 50%, transparent);
  }

  .bg-switch input:focus-visible + .bg-switch__track {
    outline: 2px solid var(--accent-primary);
    outline-offset: 1px;
  }

  .bg-switch input:checked + .bg-switch__track {
    background: var(--accent-primary);
  }

  .bg-switch:hover input:checked:not(:disabled) + .bg-switch__track {
    background: color-mix(in srgb, var(--accent-primary) 85%, var(--text-primary));
  }

  .bg-switch__thumb {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--bg-primary);
    box-shadow: 0 1px 2px var(--overlay-shadow);
    transition: transform 0.15s ease;
  }

  .bg-switch input:checked + .bg-switch__track .bg-switch__thumb {
    transform: translateX(14px);
  }
</style>
