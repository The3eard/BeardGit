<!--
  EmptyState — shared placeholder for "nothing here yet" panels.

  Wraps the long-standing `.empty-state` block from `lib/styles/empty-state.css`
  so callers stop hand-rolling their own copy. The optional `action` snippet
  surfaces a CTA button (consumers wire the click handler) so an empty state
  always tells the user what to do next.

  Usage:
    <EmptyState
      title={m.stashes_empty_title()}
      description={m.stashes_empty_description()}
      icon={"\\uF0C7"}>
      {#snippet action()}
        <Button onclick={createStash}>{m.stashes_create()}</Button>
      {/snippet}
    </EmptyState>
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    /** Bold headline. Required — empty states without one are a smell. */
    title: string;
    /** Optional supporting copy. Renders below the title. */
    description?: string;
    /** Optional Nerd Font glyph, rendered above the title at 32px. */
    icon?: string;
    /** Optional primary CTA. Pass a snippet so callers control the button. */
    action?: Snippet;
  }

  const { title, description, icon, action }: Props = $props();
</script>

<div class="empty-state">
  {#if icon}
    <span class="empty-state-icon" aria-hidden="true">{icon}</span>
  {/if}
  <h3 class="empty-state-title">{title}</h3>
  {#if description}
    <p class="empty-state-description">{description}</p>
  {/if}
  {#if action}
    <div class="empty-state-action">{@render action()}</div>
  {/if}
</div>

<style>
  .empty-state-icon {
    font-family: var(--font-icons);
    font-size: 32px;
    color: var(--text-secondary);
    opacity: 0.7;
    line-height: 1;
    margin-bottom: 4px;
  }

  .empty-state-action {
    margin-top: 8px;
  }
</style>
