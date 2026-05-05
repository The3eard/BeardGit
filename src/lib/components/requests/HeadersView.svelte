<!--
  Tabular view of the response headers. Headers arrive as an ordered
  array of `[name, value]` tuples (so `Set-Cookie` repeats are
  preserved) — `CookiesView` filters that same array for cookies.

  Rows zebra-stripe with `var(--bg-secondary)` and the key column
  reads in `--text-primary`, value column in `--text-secondary`, so
  the table matches the plain-data tables used elsewhere in the app.
-->
<script lang="ts">
  import { lastResponse } from "./stores";
</script>

{#if !$lastResponse || $lastResponse.headers.length === 0}
  <p class="empty">No headers yet.</p>
{:else}
  <table class="headers">
    <tbody>
      {#each $lastResponse.headers as [k, v]}
        <tr>
          <td class="headers__key">{k}</td>
          <td class="headers__val">{v}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .empty {
    margin: 0;
    padding: 16px 12px;
    font-size: 12px;
    color: var(--text-secondary);
    font-style: italic;
  }

  .headers {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  .headers tr:nth-child(odd) {
    background: var(--bg-secondary);
  }

  .headers td {
    padding: 6px 12px;
    vertical-align: top;
    border-bottom: 1px solid var(--border);
  }

  .headers__key {
    width: 30%;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-weight: 600;
    word-break: break-word;
  }

  .headers__val {
    color: var(--text-secondary);
    font-family: var(--font-mono);
    word-break: break-word;
  }
</style>
