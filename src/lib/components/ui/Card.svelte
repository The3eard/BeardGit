<!--
  Card.svelte — shared card container primitive for the MT-5 settings IA
  overhaul.

  Replaces the ad-hoc `.appearance-card` / `.ai-card` / `.cli-auth-card`
  classes scattered across the settings tree with a single component
  whose header is opt-in: pass `title` (and optionally `description`) to
  render a header, or just wrap content without props for a plain
  rounded container.

  Consumers can populate the header-right corner via the `actions` slot
  (good for Edit / Refresh buttons). The default slot holds the body.

  ```svelte
  <Card title="Theme" description="Controls the appearance">
    {#snippet actions()}<Button>Reset</Button>{/snippet}
    <p>body content</p>
  </Card>
  ```
-->
<script lang="ts">
  interface Props {
    /** Optional title rendered in the card header. */
    title?: string;
    /** Optional description rendered under the title. */
    description?: string;
    /** Body content slot. */
    children?: import("svelte").Snippet;
    /** Actions rendered at the top-right of the header. */
    actions?: import("svelte").Snippet;
  }

  let { title, description, children, actions }: Props = $props();

  const hasHeader = $derived(!!title || !!description || !!actions);
</script>

<section class="bg-card" class:bg-card--titled={hasHeader}>
  {#if hasHeader}
    <header class="bg-card__header">
      <div class="bg-card__headings">
        {#if title}
          <h3 class="bg-card__title">{title}</h3>
        {/if}
        {#if description}
          <p class="bg-card__description">{description}</p>
        {/if}
      </div>
      {#if actions}
        <div class="bg-card__actions">{@render actions()}</div>
      {/if}
    </header>
  {/if}
  {#if children}
    <div class="bg-card__body">{@render children()}</div>
  {/if}
</section>

<style>
  .bg-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .bg-card__header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .bg-card__headings {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .bg-card__title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .bg-card__description {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-secondary);
  }

  .bg-card__actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .bg-card__body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
</style>
