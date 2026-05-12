<!--
  HeadersTab.svelte — Key/value editor for request headers.

  Binds directly to `currentRequest.headers`. Each row commits with a
  `currentRequest.set({ ...current, headers: ... })` so Svelte stores
  pick up the change (mutating the array in place wouldn't trigger
  reactivity on the parent components).

  Uses the shared `Button` / `IconButton` primitives and the
  `bg-input` recipe so the form chrome matches the rest of the app.
-->
<script lang="ts">
  import { Button, IconButton } from "$lib/components/ui";
  import { currentRequest } from "./stores";

  /** Append an empty header row at the bottom of the list. */
  function add() {
    if (!$currentRequest) return;
    currentRequest.set({
      ...$currentRequest,
      headers: [...$currentRequest.headers, ["", ""]],
    });
  }

  /** Delete the row at index `i`. */
  function remove(i: number) {
    if (!$currentRequest) return;
    currentRequest.set({
      ...$currentRequest,
      headers: $currentRequest.headers.filter((_, j) => j !== i),
    });
  }

  /** Update the key + value pair at index `i`. */
  function commit(i: number, k: string, v: string) {
    if (!$currentRequest) return;
    const headers = $currentRequest.headers.slice();
    headers[i] = [k, v];
    currentRequest.set({ ...$currentRequest, headers });
  }
</script>

<div class="kv">
  {#if $currentRequest}
    {#each $currentRequest.headers as [k, v], i}
      <div class="kv__row">
        <input
          class="bg-input"
          value={k}
          on:input={(e) =>
            commit(i, (e.currentTarget as HTMLInputElement).value, v)}
          placeholder="name"
        />
        <input
          class="bg-input"
          value={v}
          on:input={(e) =>
            commit(i, k, (e.currentTarget as HTMLInputElement).value)}
          placeholder="value"
        />
        <IconButton
          icon={""}
          description="Remove header"
          tone="danger"
          size="sm"
          onclick={() => remove(i)}
        />
      </div>
    {/each}
    <div class="kv__add">
      <Button variant="neutral" size="xs" icon={""} onclick={add}>
        Add header
      </Button>
    </div>
  {/if}
</div>

<style>
  .kv {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .kv__row {
    display: grid;
    grid-template-columns: 1fr 2fr 28px;
    gap: 6px;
    align-items: center;
  }

  .kv__add {
    margin-top: 4px;
  }

  .bg-input {
    width: 100%;
    height: 30px;
    line-height: 28px;
    padding: 0 10px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: 12px;
    outline: none;
    box-sizing: border-box;
  }

  .bg-input:focus {
    border-color: var(--accent-primary);
  }
</style>
