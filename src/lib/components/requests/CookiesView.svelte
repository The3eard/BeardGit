<!--
  Lists `Set-Cookie` headers from the last response, one per line.
  Source of truth is `lastResponse.headers` (ordered tuples) so
  multiple cookies on the same response are all surfaced.

  Empty state matches the rest of the app's empty placeholders
  (italic, `--text-secondary`); cookie lines render in the standard
  monospace `--font-mono` over `--bg-secondary` so they read like
  code blocks.
-->
<script lang="ts">
  import { lastResponse } from "./stores";

  $: cookies = ($lastResponse?.headers ?? [])
    .filter(([k]) => k.toLowerCase() === "set-cookie")
    .map(([, v]) => v);
</script>

{#if cookies.length === 0}
  <p class="empty">No cookies in response.</p>
{:else}
  <ul class="cookies">
    {#each cookies as c}
      <li><code>{c}</code></li>
    {/each}
  </ul>
{/if}

<style>
  .empty {
    margin: 0;
    padding: 16px 12px;
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    font-style: italic;
  }

  .cookies {
    list-style: none;
    padding: 8px;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .cookies li {
    padding: 6px 10px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .cookies code {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    word-break: break-all;
  }
</style>
