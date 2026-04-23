<!--
  AssigneeStack — renders up to `max` avatar discs for the given usernames,
  with a `+N` overflow chip when the list exceeds `max`. Purely presentational;
  colours are deterministic per-login so the same user is always the same hue.

  Consumers: `IssueList.svelte` today, `MrPrList.svelte` / `PrDetail.svelte`
  next iteration (out of scope for this plan).
-->
<script lang="ts">
  interface Props {
    /** Usernames to render, in order. */
    assignees: string[];
    /** Maximum avatars to show before collapsing into `+N`. Default 3. */
    max?: number;
  }
  let { assignees, max = 3 }: Props = $props();

  /** Cheap deterministic hash → hue. Avoids importing a crypto/md5 lib just
   *  for a tiny avatar colour — collisions are fine, they'd happen on real
   *  avatars too. */
  function hueFor(login: string): number {
    let h = 0;
    for (let i = 0; i < login.length; i += 1) h = (h * 31 + login.charCodeAt(i)) >>> 0;
    return h % 360;
  }

  let visible = $derived(assignees.slice(0, max));
  let overflow = $derived(Math.max(0, assignees.length - max));
  let ariaLabel = $derived(`${assignees.length} assignees`);
</script>

{#if assignees.length > 0}
  <span class="assignee-stack" aria-label={ariaLabel}>
    {#each visible as login (login)}
      <span
        class="assignee-avatar"
        title={login}
        style:background="hsl({hueFor(login)}, 55%, 45%)"
      >{login.charAt(0).toUpperCase()}</span>
    {/each}
    {#if overflow > 0}
      <span class="assignee-overflow" title={assignees.slice(max).join(", ")}>+{overflow}</span>
    {/if}
  </span>
{/if}

<style>
  .assignee-stack {
    display: inline-flex;
    align-items: center;
    gap: -4px; /* overridden by negative margin below */
  }
  .assignee-avatar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    font-size: 9px;
    font-weight: 600;
    color: var(--text-primary);
    border: 1px solid var(--bg-primary);
    margin-left: -4px;
    flex-shrink: 0;
  }
  .assignee-avatar:first-child { margin-left: 0; }
  .assignee-overflow {
    font-size: 10px;
    color: var(--text-secondary);
    margin-left: 4px;
    flex-shrink: 0;
  }
</style>
