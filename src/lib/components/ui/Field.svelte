<!--
  Field.svelte — vertical label/description/control/error primitive.

  Where `FormRow` places the label to the left of the control, `Field`
  stacks them vertically — handy for wider controls (textareas, URL
  inputs, etc.) where the horizontal layout would crowd.

  ```svelte
  <Field label="API endpoint" description="Leave empty for defaults" error={err}>
    <input id="endpoint" bind:value={endpoint} />
  </Field>
  ```
-->
<script lang="ts">
  interface Props {
    /** Visible label text. */
    label: string;
    /** `for` attribute pointing at the control's `id`. */
    for?: string;
    /** Helper description below the label. */
    description?: string;
    /** Validation error message; when set, the field highlights. */
    error?: string;
    /** Control slot (input, select, switch, textarea). */
    children?: import("svelte").Snippet;
  }

  let {
    label,
    for: htmlFor,
    description,
    error,
    children,
  }: Props = $props();
</script>

<div class="bg-field" class:bg-field--error={!!error}>
  <label class="bg-field__label" for={htmlFor}>{label}</label>
  {#if description}
    <p class="bg-field__description">{description}</p>
  {/if}
  <div class="bg-field__control">
    {#if children}{@render children()}{/if}
  </div>
  {#if error}
    <p class="bg-field__error" role="alert" data-testid="bg-field-error">
      {error}
    </p>
  {/if}
</div>

<style>
  .bg-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .bg-field__label {
    font-size: 12px;
    color: var(--text-primary);
    font-weight: 500;
  }

  .bg-field__description {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.45;
  }

  .bg-field__control {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .bg-field__error {
    margin: 0;
    font-size: 11px;
    color: var(--accent-red);
    line-height: 1.45;
  }

  .bg-field--error .bg-field__label {
    color: var(--accent-red);
  }
</style>
