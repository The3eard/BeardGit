<!--
  FormRow.svelte — horizontal label/control layout primitive.

  Shared by every settings category: label on the left (taking a fixed
  share of the row), control slot on the right (auto-sizing), optional
  helper text beneath the pair. Replaces the `.setting-row` / `.bg-field`
  / `.tool-row` ad-hoc classes used today.

  ```svelte
  <FormRow label="Theme" for="theme-select" helperText="Persisted per user">
    <select id="theme-select" bind:value={theme}>…</select>
  </FormRow>
  ```
-->
<script lang="ts">
  interface Props {
    /** Text for the label. */
    label: string;
    /** `for` attribute pointing at the control's `id`. */
    for?: string;
    /** Helper text rendered below the row. */
    helperText?: string;
    /** Control slot (input, select, switch, etc.). */
    children?: import("svelte").Snippet;
  }

  let { label, for: htmlFor, helperText, children }: Props = $props();
</script>

<div class="bg-form-row">
  <div class="bg-form-row__pair">
    <label class="bg-form-row__label" for={htmlFor}>{label}</label>
    <div class="bg-form-row__control">
      {#if children}{@render children()}{/if}
    </div>
  </div>
  {#if helperText}
    <p class="bg-form-row__helper" data-testid="bg-form-row-helper">
      {helperText}
    </p>
  {/if}
</div>

<style>
  .bg-form-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .bg-form-row__pair {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .bg-form-row__label {
    font-size: 12px;
    color: var(--text-primary);
    flex: 0 0 auto;
    min-width: 0;
  }

  .bg-form-row__control {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 0 1 auto;
  }

  .bg-form-row__helper {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.45;
  }
</style>
