<!--
  History tab for the Response viewer.

  Loads the most recent runs for the currently-selected request via
  `requests_history` and lets the user pick exactly two rows to diff.
  Diffing fetches both response bodies through `requests_diff_responses`
  and dispatches a `requests:diff` window event so a higher-level host
  can mount a merge view; a follow-up task wires that view inline.

  Uses the shared `<List>` primitive so the panel header, scroll
  container, loading bar, and empty state behave like every other list
  in the app. The `headerActions` snippet hosts the "Diff selected"
  button. List's built-in `selectedKey` only models a single highlight,
  so multi-selection (capped at 2 for diffing) is tracked locally in a
  `Set<number>` and surfaced through the `row` snippet's class binding
  rather than the primitive's `selected` flag.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Button } from "$lib/components/ui";
  import List from "../common/List.svelte";
  import TwoLineRow from "../common/TwoLineRow.svelte";
  import { currentSource } from "./stores";

  type Row = {
    id: number;
    status: number | null;
    duration_ms: number;
    executed_at: number;
    env_name: string | null;
    truncated: boolean;
  };

  let rows: Row[] = [];
  let selected = new Set<number>();
  let loading = false;

  async function reload() {
    if (!$currentSource) {
      rows = [];
      return;
    }
    loading = true;
    try {
      rows = await invoke<Row[]>("requests_history", {
        sourceKind: $currentSource.kind,
        sourcePath: $currentSource.path,
        limit: 50,
      });
    } finally {
      loading = false;
    }
  }

  function toggle(id: number) {
    if (selected.has(id)) {
      selected.delete(id);
    } else if (selected.size < 2) {
      selected.add(id);
    }
    selected = new Set(selected);
  }

  async function diff() {
    if (selected.size !== 2) return;
    const [a, b] = [...selected];
    const payload = await invoke<{
      left: string;
      right: string;
      content_type_hint: string | null;
    }>("requests_diff_responses", { historyIdA: a, historyIdB: b });
    // Hand off to the page-level diff host. A future task wires the merge view directly.
    window.dispatchEvent(
      new CustomEvent("requests:diff", { detail: payload }),
    );
  }

  /** Pick a colour register from the HTTP status code. Mirrors `ResponseHeaderBar`. */
  function statusKind(s: number | null): "ok" | "warn" | "err" | "muted" {
    if (s === null) return "muted";
    if (s >= 400) return "err";
    if (s >= 300) return "warn";
    return "ok";
  }

  function getKey(row: Row): string {
    return String(row.id);
  }

  function formatTime(secs: number): string {
    return new Date(secs * 1000).toLocaleString();
  }

  onMount(reload);
  $: $currentSource, reload();
</script>

<List
  items={rows}
  loading={loading}
  title="Run history"
  selectedKey={null}
  {getKey}
  emptyMessage="No runs yet."
>
  {#snippet headerActions()}
    <span class="hint">
      {selected.size === 2 ? "Ready to diff" : "Pick 2 rows"}
    </span>
    <Button
      variant="primary"
      size="sm"
      icon={""}
      disabled={selected.size !== 2}
      onclick={diff}
    >
      Diff selected
    </Button>
  {/snippet}

  {#snippet row({ item })}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="hrow"
      class:hrow--checked={selected.has(item.id)}
      on:click|stopPropagation={() => toggle(item.id)}
      on:keydown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          toggle(item.id);
        }
      }}
    >
      <TwoLineRow selected={selected.has(item.id)}>
        {#snippet leadIcon()}
          <span
            class="dot dot--{statusKind(item.status)}"
            aria-hidden="true"
          ></span>
        {/snippet}
        {#snippet keyLabel()}
          <span class="status-label">{item.status ?? "—"}</span>
        {/snippet}
        {#snippet title()}
          <span>{item.duration_ms} ms</span>
        {/snippet}
        {#snippet trailingDate()}
          <span>{formatTime(item.executed_at)}</span>
        {/snippet}
        {#snippet meta()}
          {#if item.env_name}
            <span class="env">{item.env_name}</span>
          {/if}
          {#if item.truncated}
            <span class="warn">truncated</span>
          {/if}
        {/snippet}
      </TwoLineRow>
    </div>
  {/snippet}
</List>

<style>
  .hint {
    font-size: 11px;
    color: var(--text-secondary);
    margin-right: 4px;
  }

  /* Manual multi-select highlight — `List`'s `selectedKey` is single-only,
     so we paint a tonal accent-blue stripe directly on rows whose id is
     in the local `selected` set. The colours intentionally match
     `.list-row.selected` so the visual reads as the same selection state. */
  .hrow {
    display: block;
    width: 100%;
    cursor: pointer;
    border-radius: 4px;
  }
  .hrow--checked {
    background: color-mix(in srgb, var(--accent-blue) 14%, transparent);
    box-shadow: inset 0 0 0 1px
      color-mix(in srgb, var(--accent-blue) 40%, transparent);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
  }
  .dot--ok { background: var(--accent-green); }
  .dot--warn { background: var(--accent-orange); }
  .dot--err { background: var(--accent-red); }
  .dot--muted { background: var(--text-secondary); }

  .status-label {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .env {
    color: var(--text-secondary);
  }

  .warn {
    color: var(--accent-orange);
  }
</style>
