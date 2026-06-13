<!--
  Checkbox.svelte — BeardGit-styled checkbox primitive.

  Replaces bare `<input type="checkbox">` so selection lists (staging,
  clean dialog, release assets) read as part of the app instead of as
  the platform's native control. A real, visually-hidden input keeps
  the native semantics — `event.target.checked`, keyboard toggling,
  form behaviour and screen-reader support all work unchanged, so
  call sites can swap `<input type="checkbox" …>` for `<Checkbox …>`
  one-for-one.

  For boolean *settings* (on/off preferences) prefer `Switch.svelte`;
  Checkbox is for *selection* ("include this item") semantics.
-->
<script lang="ts">
  interface Props {
    /** Whether the box is ticked. */
    checked?: boolean;
    /** Tri-state "some selected" visual (overrides the check glyph). */
    indeterminate?: boolean;
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
    indeterminate = false,
    disabled = false,
    id,
    ariaLabel,
    testid,
    onclick,
    onchange,
  }: Props = $props();
</script>

<label class="bg-checkbox" class:bg-checkbox--disabled={disabled}>
  <input
    {id}
    type="checkbox"
    {checked}
    {indeterminate}
    {disabled}
    aria-label={ariaLabel}
    data-testid={testid}
    {onclick}
    {onchange}
  />
  <span class="bg-checkbox__box" aria-hidden="true">
    {#if indeterminate}
      <span class="bg-checkbox__dash"></span>
    {:else}
      <span class="bg-checkbox__check nf">{""}</span>
    {/if}
  </span>
</label>

<style>
  .bg-checkbox {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
  }

  .bg-checkbox--disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  /* Real input kept for semantics, visually replaced by the box. It
     covers the whole label so the hit target matches the visual. */
  .bg-checkbox input {
    position: absolute;
    inset: 0;
    margin: 0;
    opacity: 0;
    cursor: inherit;
  }

  .bg-checkbox__box {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 15px;
    height: 15px;
    border-radius: 4px;
    border: 1px solid color-mix(in srgb, var(--text-secondary) 60%, transparent);
    background: var(--bg-primary);
    transition:
      background 0.12s ease,
      border-color 0.12s ease;
  }

  .bg-checkbox:hover input:not(:disabled) + .bg-checkbox__box {
    border-color: var(--accent-primary);
  }

  .bg-checkbox input:focus-visible + .bg-checkbox__box {
    outline: 2px solid var(--accent-primary);
    outline-offset: 1px;
  }

  .bg-checkbox input:checked + .bg-checkbox__box,
  .bg-checkbox input:indeterminate + .bg-checkbox__box {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
  }

  .bg-checkbox__check {
    font-family: var(--font-icons);
    font-size: 9px;
    line-height: 1;
    color: var(--bg-primary);
    opacity: 0;
    transform: scale(0.6);
    transition:
      opacity 0.12s ease,
      transform 0.12s ease;
  }

  .bg-checkbox input:checked + .bg-checkbox__box .bg-checkbox__check {
    opacity: 1;
    transform: scale(1);
  }

  .bg-checkbox__dash {
    width: 7px;
    height: 2px;
    border-radius: 1px;
    background: var(--bg-primary);
  }
</style>
