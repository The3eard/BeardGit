<!--
  LazyComponent — wraps a dynamic `import()` so a heavy panel ships in
  its own chunk and only fetches when first rendered. Used by
  `routes/+page.svelte` to keep the initial JS bundle small (xterm,
  CodeMirror diff, AI workflow forms, …) without rewriting every view.

  Usage:

      <LazyComponent
        loader={() => import("$lib/components/foo/FooView.svelte")}
        props={{ a: 1, b: 2 }}
      />

  The `loader` thunk is what tells the bundler "split this chunk".
  Calling `import()` at module top-level would defeat the split.
-->
<script lang="ts" generics="P extends Record<string, unknown>">
  import type { Component } from "svelte";

  interface Props {
    /** A thunk returning the dynamic-import promise of the target component. */
    loader: () => Promise<{ default: Component<P> }>;
    /** Props to forward to the loaded component. */
    props?: P;
    /** Optional placeholder shown while the chunk is loading. */
    placeholder?: import("svelte").Snippet;
  }

  let { loader, props, placeholder }: Props = $props();

  let LoadedComponent = $state<Component<P> | null>(null);
  let loadError = $state<unknown>(null);

  $effect(() => {
    let cancelled = false;
    loader()
      .then((mod) => {
        if (!cancelled) LoadedComponent = mod.default;
      })
      .catch((err) => {
        if (!cancelled) loadError = err;
      });
    return () => {
      cancelled = true;
    };
  });
</script>

{#if loadError}
  <div class="lazy-error">Failed to load panel.</div>
{:else if LoadedComponent}
  <LoadedComponent {...(props ?? ({} as P))} />
{:else if placeholder}
  {@render placeholder()}
{:else}
  <div class="lazy-placeholder">
    <div class="spinner"></div>
  </div>
{/if}

<style>
  .lazy-placeholder,
  .lazy-error {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    font-size: 12px;
  }
</style>
