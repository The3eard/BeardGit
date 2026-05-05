<!--
  ParamsTab.svelte — Edit the query string of the current request URL.

  We treat the URL as the source of truth: each row edit re-serializes
  the rows back into a `?k=v&k=v` string and replaces the query portion
  of `currentRequest.url`. That way users editing the URL field
  directly and users editing rows here stay in sync without a separate
  "params" field.

  Form chrome uses the shared `Button` + `IconButton` primitives plus
  the `bg-input` recipe so rows match the rest of the app's editors.
-->
<script lang="ts">
  import { Button, IconButton } from "$lib/components/ui";
  import { currentRequest } from "./stores";

  /** Decode a URL's query string into rows of `[name, value]`. */
  function parseRows(url: string): [string, string][] {
    const idx = url.indexOf("?");
    if (idx < 0) return [];
    const qs = url.slice(idx + 1);
    return qs
      .split("&")
      .filter(Boolean)
      .map((p) => {
        const [k, v] = p.split("=");
        return [decodeURIComponent(k ?? ""), decodeURIComponent(v ?? "")] as [
          string,
          string,
        ];
      });
  }

  /** Encode rows back into a URL-safe `k=v&k=v` query string. */
  function serialize(rs: [string, string][]): string {
    return rs
      .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`)
      .join("&");
  }

  /** Rows derived from the URL's current query string. */
  let rows: [string, string][] = [];
  $: rows = parseRows($currentRequest?.url ?? "");

  /** Replace the URL's query portion with `next`'s serialization. */
  function commit(next: [string, string][]) {
    if (!$currentRequest) return;
    const base = ($currentRequest.url ?? "").split("?")[0];
    const qs = serialize(next);
    currentRequest.set({
      ...$currentRequest,
      url: qs ? `${base}?${qs}` : base,
    });
  }

  /** Update row `i` to `[k, v]` and write back to the URL. */
  function update(i: number, k: string, v: string) {
    const next = rows.slice();
    next[i] = [k, v];
    commit(next);
  }

  /** Remove row `i` from the URL's query string. */
  function remove(i: number) {
    commit(rows.filter((_, j) => j !== i));
  }

  /** Append an empty row to the URL's query string. */
  function add() {
    commit([...rows, ["", ""]]);
  }
</script>

<div class="kv">
  {#each rows as [k, v], i}
    <div class="kv__row">
      <input
        class="bg-input"
        value={k}
        on:input={(e) =>
          update(i, (e.currentTarget as HTMLInputElement).value, v)}
        placeholder="name"
      />
      <input
        class="bg-input"
        value={v}
        on:input={(e) =>
          update(i, k, (e.currentTarget as HTMLInputElement).value)}
        placeholder="value"
      />
      <IconButton
        icon={""}
        description="Remove parameter"
        tone="danger"
        size="sm"
        onclick={() => remove(i)}
      />
    </div>
  {/each}
  <div class="kv__add">
    <Button variant="neutral" size="xs" icon={""} onclick={add}>
      Add param
    </Button>
  </div>
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
    border-color: var(--accent-blue);
  }
</style>
