<!--
  SettingSection.svelte — titled section primitive used inside category
  components to group related settings.

  By default the body is always rendered under the title. Passing
  `collapsible` turns the title into a toggle button with a chevron
  glyph that hides/shows the body; `defaultOpen={false}` starts it
  collapsed.

  ```svelte
  <SettingSection title="Appearance" description="Theme + density">
    <FormRow …>…</FormRow>
  </SettingSection>
  ```
-->
<script lang="ts">
  interface Props {
    /** Section title (translated). */
    title: string;
    /** Optional subtitle / description below the title. */
    description?: string;
    /** When true, the title toggles the body. Default false. */
    collapsible?: boolean;
    /** Initial open state for collapsible sections. Default true. */
    defaultOpen?: boolean;
    /** Body content. */
    children?: import("svelte").Snippet;
  }

  let {
    title,
    description,
    collapsible = false,
    defaultOpen = true,
    children,
  }: Props = $props();

  // `defaultOpen` is read once as the initial value; runtime toggling is
  // driven by the `open` state below. Svelte's state_referenced_locally
  // lint flags the one-shot read — silence it intentionally.
  // svelte-ignore state_referenced_locally
  let open = $state(defaultOpen);

  function toggle() {
    if (!collapsible) return;
    open = !open;
  }

  const showBody = $derived(!collapsible || open);
</script>

<section class="bg-setting-section">
  {#if collapsible}
    <button
      type="button"
      class="bg-setting-section__toggle"
      aria-expanded={open}
      data-testid="bg-setting-section-toggle"
      onclick={toggle}
    >
      <span class="bg-setting-section__chevron nf" aria-hidden="true"
        >{open ? "\uF078" : "\uF054"}</span
      >
      <span class="bg-setting-section__headings">
        <span class="bg-setting-section__title">{title}</span>
        {#if description}
          <span class="bg-setting-section__description">{description}</span>
        {/if}
      </span>
    </button>
  {:else}
    <header class="bg-setting-section__header">
      <h4 class="bg-setting-section__title">{title}</h4>
      {#if description}
        <p class="bg-setting-section__description">{description}</p>
      {/if}
    </header>
  {/if}
  {#if showBody && children}
    <div class="bg-setting-section__body" data-testid="bg-setting-section-body">
      {@render children()}
    </div>
  {/if}
</section>

<style>
  .bg-setting-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .bg-setting-section__header {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .bg-setting-section__toggle {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    background: transparent;
    border: none;
    padding: 2px 0;
    cursor: pointer;
    color: var(--text-primary);
    font-family: inherit;
    text-align: left;
  }

  .bg-setting-section__toggle:hover .bg-setting-section__title {
    color: var(--accent-blue);
  }

  .bg-setting-section__chevron {
    font-family: var(--font-icons);
    font-size: 10px;
    color: var(--text-secondary);
    width: 12px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding-top: 3px;
  }

  .bg-setting-section__headings {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .bg-setting-section__title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .bg-setting-section__description {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.45;
  }

  .bg-setting-section__body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-left: 20px;
  }
</style>
